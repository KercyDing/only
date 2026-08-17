use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::mpsc::Sender;

use only_semantic::{ShellKind, ShellOperator, ShellSelection};

use crate::EngineError;
use crate::path_lookup::command_exists_in_path;
use crate::process::{
    OutputChunk, build_command_env, join_output_reader, run_with_system_shell,
    run_with_system_shell_inherit, spawn_output_reader,
};

pub(crate) fn run_command(
    command: &str,
    working_dir: &Path,
    shell: &ShellSelection,
    output: Sender<OutputChunk>,
) -> Result<ExitCode, EngineError> {
    let resolved_shell = resolve_shell(shell)?;
    match resolved_shell {
        ShellKind::Deno => run_with_deno_task_shell(command, working_dir, output),
        ShellKind::Sh => run_with_system_shell("sh", "-c", command, working_dir, output),
        ShellKind::Bash => run_with_system_shell("bash", "-c", command, working_dir, output),
        ShellKind::Powershell => run_with_system_shell(
            power_shell_command(),
            "-Command",
            command,
            working_dir,
            output,
        ),
        ShellKind::Pwsh => run_with_system_shell("pwsh", "-Command", command, working_dir, output),
        ShellKind::Unknown(name) => Err(EngineError::UnsupportedShell(name.to_string())),
    }
}

pub(crate) fn run_command_inherit(
    command: &str,
    working_dir: &Path,
    shell: &ShellSelection,
) -> Result<ExitCode, EngineError> {
    let resolved_shell = resolve_shell(shell)?;
    match resolved_shell {
        ShellKind::Deno => run_with_deno_task_shell_inherit(command, working_dir),
        ShellKind::Sh => run_with_system_shell_inherit("sh", "-c", command, working_dir),
        ShellKind::Bash => run_with_system_shell_inherit("bash", "-c", command, working_dir),
        ShellKind::Powershell => {
            run_with_system_shell_inherit(power_shell_command(), "-Command", command, working_dir)
        }
        ShellKind::Pwsh => run_with_system_shell_inherit("pwsh", "-Command", command, working_dir),
        ShellKind::Unknown(name) => Err(EngineError::UnsupportedShell(name.to_string())),
    }
}

fn resolve_shell(shell: &ShellSelection) -> Result<ShellKind, EngineError> {
    match &shell.kind {
        ShellKind::Pwsh => {
            if command_exists_in_path("pwsh") {
                return Ok(ShellKind::Pwsh);
            }
            if shell.operator == ShellOperator::Fallback
                && command_exists_in_path(power_shell_command())
            {
                return Ok(ShellKind::Powershell);
            }
            Err(EngineError::ShellNotFound(
                "pwsh was not found\nhelp: install PowerShell 7+, or use `shell~=pwsh`".to_string(),
            ))
        }
        ShellKind::Bash => {
            if command_exists_in_path("bash") {
                return Ok(ShellKind::Bash);
            }
            if shell.operator == ShellOperator::Fallback && command_exists_in_path("sh") {
                return Ok(ShellKind::Sh);
            }
            Err(EngineError::ShellNotFound(
                "bash was not found\nhelp: install bash, or use `shell~=bash`".to_string(),
            ))
        }
        ShellKind::Powershell => {
            if command_exists_in_path(power_shell_command()) {
                return Ok(ShellKind::Powershell);
            }
            Err(EngineError::ShellNotFound(
                "powershell was not found\nhelp: install Windows PowerShell".to_string(),
            ))
        }
        ShellKind::Sh => {
            if command_exists_in_path("sh") {
                return Ok(ShellKind::Sh);
            }
            Err(EngineError::ShellNotFound(
                "sh was not found\nhelp: install a POSIX shell".to_string(),
            ))
        }
        ShellKind::Deno => Ok(ShellKind::Deno),
        ShellKind::Unknown(name) => Err(EngineError::UnsupportedShell(name.to_string())),
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

fn run_with_deno_task_shell_inherit(
    command: &str,
    working_dir: &Path,
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
            deno_task_shell::ShellPipeWriter::stdout(),
            deno_task_shell::ShellPipeWriter::stderr(),
        ),
    );

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
