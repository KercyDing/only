use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::mpsc::Sender;

use crate::EngineError;
use crate::process::{
    OutputChunk, build_command_env, join_output_reader, run_with_system_shell, spawn_output_reader,
};

pub(crate) fn run_command(
    command: &str,
    working_dir: &Path,
    shell: &str,
    shell_fallback: bool,
    output: Sender<OutputChunk>,
) -> Result<ExitCode, EngineError> {
    let resolved_shell = resolve_shell(shell, shell_fallback)?;
    match resolved_shell.as_str() {
        "deno" => run_with_deno_task_shell(command, working_dir, output),
        "sh" => run_with_system_shell("sh", "-c", command, working_dir, output),
        "bash" => run_with_system_shell("bash", "-c", command, working_dir, output),
        "powershell" => run_with_system_shell(
            power_shell_command(),
            "-Command",
            command,
            working_dir,
            output,
        ),
        "pwsh" => run_with_system_shell("pwsh", "-Command", command, working_dir, output),
        other => Err(EngineError::Runtime(format!("unsupported shell '{other}'"))),
    }
}

fn resolve_shell(shell: &str, shell_fallback: bool) -> Result<String, EngineError> {
    match shell {
        "pwsh" => {
            if shell_exists("pwsh") {
                return Ok("pwsh".to_string());
            }
            if shell_fallback && shell_exists(power_shell_command()) {
                return Ok("powershell".to_string());
            }
            Err(EngineError::Runtime(
                "pwsh not found. Install PowerShell 7+ or use shell?=pwsh for auto fallback."
                    .to_string(),
            ))
        }
        "bash" => {
            if shell_exists("bash") {
                return Ok("bash".to_string());
            }
            if shell_fallback && shell_exists("sh") {
                return Ok("sh".to_string());
            }
            Err(EngineError::Runtime(
                "bash not found. Install bash or use shell?=bash for auto fallback.".to_string(),
            ))
        }
        "powershell" => {
            if shell_exists(power_shell_command()) {
                return Ok("powershell".to_string());
            }
            Err(EngineError::Runtime(
                "powershell not found. Ensure Windows PowerShell is installed.".to_string(),
            ))
        }
        "sh" => {
            if shell_exists("sh") {
                return Ok("sh".to_string());
            }
            Err(EngineError::Runtime(
                "sh not found. Ensure a POSIX shell is available.".to_string(),
            ))
        }
        "deno" => Ok("deno".to_string()),
        other => Ok(other.to_string()),
    }
}

fn shell_exists(shell: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| shell_exists_in_dir(&directory, shell))
    })
}

fn shell_exists_in_dir(directory: &Path, shell: &str) -> bool {
    let candidate = directory.join(shell);
    if candidate.is_file() {
        return true;
    }

    #[cfg(windows)]
    {
        let has_extension = Path::new(shell).extension().is_some();
        if has_extension {
            return false;
        }

        let extensions = std::env::var_os("PATHEXT")
            .and_then(|value| value.into_string().ok())
            .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string());

        extensions
            .split(';')
            .map(str::trim)
            .filter(|extension| !extension.is_empty())
            .any(|extension| directory.join(format!("{shell}{extension}")).is_file())
    }

    #[cfg(not(windows))]
    {
        false
    }
}

fn run_with_deno_task_shell(
    command: &str,
    working_dir: &Path,
    output: Sender<OutputChunk>,
) -> Result<ExitCode, EngineError> {
    let parsed = deno_task_shell::parser::parse(command).map_err(|error| {
        EngineError::Runtime(format!("failed to parse command `{command}`: {error}"))
    })?;
    let env_vars = build_command_env();
    let state = deno_task_shell::ShellState::new(
        env_vars,
        working_dir.to_path_buf(),
        HashMap::<String, Rc<dyn deno_task_shell::ShellCommand>>::new(),
        deno_task_shell::KillSignal::default(),
    );
    let (stdout_reader, stdout_writer) = deno_task_shell::pipe();
    let (stderr_reader, stderr_writer) = deno_task_shell::pipe();
    let stdout_handle = spawn_output_reader(
        ShellPipeReaderAdapter(stdout_reader),
        crate::process::OutputStream::Stdout,
        output.clone(),
    );
    let stderr_handle = spawn_output_reader(
        ShellPipeReaderAdapter(stderr_reader),
        crate::process::OutputStream::Stderr,
        output,
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| EngineError::Runtime(format!("failed to start task runtime: {error}")))?;
    let local = tokio::task::LocalSet::new();
    let status = local.block_on(
        &runtime,
        deno_task_shell::execute_with_pipes(
            parsed,
            state,
            deno_task_shell::ShellPipeReader::stdin(),
            stdout_writer,
            stderr_writer,
        ),
    );
    join_output_reader(stdout_handle)?;
    join_output_reader(stderr_handle)?;

    Ok(ExitCode::from(status as u8))
}

struct ShellPipeReaderAdapter(deno_task_shell::ShellPipeReader);

impl Read for ShellPipeReaderAdapter {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0
            .read(buf)
            .map_err(|error| std::io::Error::other(error.to_string()))
    }
}

fn power_shell_command() -> &'static str {
    if cfg!(windows) {
        "powershell.exe"
    } else {
        "powershell"
    }
}
