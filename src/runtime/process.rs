use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, ExitCode};

use crate::diagnostic::error::{OnlyError, Result};
use crate::model::ShellKind;

/// Runs a single shell command and propagates its exit status.
///
/// Args:
/// command: Shell command text to execute.
/// working_dir: Directory used as the shell working directory.
/// shell: Selected execution backend.
/// shell_fallback: Whether to fallback to alternative shell if primary not found.
///
/// Returns:
/// Process exit code produced by the shell.
pub fn run_command(
    command: &str,
    working_dir: &Path,
    shell: ShellKind,
    shell_fallback: bool,
) -> Result<ExitCode> {
    let resolved_shell = resolve_shell(shell, shell_fallback)?;
    match resolved_shell {
        ShellKind::Deno => run_with_deno_task_shell(command, working_dir),
        ShellKind::Sh => run_with_system_shell("sh", "-c", command, working_dir),
        ShellKind::Bash => run_with_system_shell("bash", "-c", command, working_dir),
        ShellKind::PowerShell => {
            run_with_system_shell(power_shell_command(), "-Command", command, working_dir)
        }
        ShellKind::Pwsh => run_with_system_shell("pwsh", "-Command", command, working_dir),
    }
}

/// Resolves the shell to use, with optional fallback.
///
/// Args:
/// shell: Requested shell kind.
/// shell_fallback: Whether to fallback to alternative shell.
///
/// Returns:
/// Resolved shell kind, or error if no suitable shell found.
fn resolve_shell(shell: ShellKind, shell_fallback: bool) -> Result<ShellKind> {
    match shell {
        ShellKind::Pwsh => {
            if shell_exists("pwsh") {
                return Ok(ShellKind::Pwsh);
            }
            if shell_fallback && shell_exists(power_shell_command()) {
                return Ok(ShellKind::PowerShell);
            }
            if shell_fallback {
                return Err(OnlyError::runtime(
                    "pwsh not found and fallback to powershell failed. \
                     Install PowerShell 7+ (pwsh) or ensure Windows PowerShell is available.",
                ));
            }
            Err(OnlyError::runtime(
                "pwsh not found. Install PowerShell 7+ or use shell?=pwsh for auto fallback.",
            ))
        }
        ShellKind::Bash => {
            if shell_exists("bash") {
                return Ok(ShellKind::Bash);
            }
            if shell_fallback && shell_exists("sh") {
                return Ok(ShellKind::Sh);
            }
            if shell_fallback {
                return Err(OnlyError::runtime(
                    "bash not found and fallback to sh failed. \
                     Install bash or ensure sh is available.",
                ));
            }
            Err(OnlyError::runtime(
                "bash not found. Install bash or use shell?=bash for auto fallback.",
            ))
        }
        ShellKind::PowerShell => {
            if shell_exists(power_shell_command()) {
                return Ok(ShellKind::PowerShell);
            }
            Err(OnlyError::runtime(
                "powershell not found. Ensure Windows PowerShell is installed.",
            ))
        }
        ShellKind::Sh => {
            if shell_exists("sh") {
                return Ok(ShellKind::Sh);
            }
            Err(OnlyError::runtime(
                "sh not found. Ensure a POSIX shell is available.",
            ))
        }
        ShellKind::Deno => Ok(ShellKind::Deno),
    }
}

/// Checks if a shell command exists in PATH.
fn shell_exists(shell: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| shell_exists_in_dir(&directory, shell))
    })
}

fn shell_exists_in_dir(directory: &std::path::Path, shell: &str) -> bool {
    let candidate = directory.join(shell);
    if candidate.is_file() {
        return true;
    }

    #[cfg(windows)]
    {
        let has_extension = std::path::Path::new(shell).extension().is_some();
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

/// Builds a runtime error for a failed command execution.
///
/// Args:
/// task: Qualified task name.
/// step_index: One-based command index within the task.
/// step_total: Total number of commands in the task.
/// command: Rendered shell command.
/// code: Exit code returned by the shell.
///
/// Returns:
/// Structured runtime error with execution context.
pub fn command_failed(
    task: &str,
    step_index: usize,
    step_total: usize,
    command: &str,
    code: ExitCode,
) -> OnlyError {
    OnlyError::runtime(format!(
        "task '{task}' failed at step [{step_index}/{step_total}] while running `{command}` with exit code {:?}",
        code
    ))
}

fn run_with_deno_task_shell(command: &str, working_dir: &Path) -> Result<ExitCode> {
    let parsed = deno_task_shell::parser::parse(command).map_err(|error| {
        OnlyError::runtime(format!("failed to parse command `{command}`: {error}"))
    })?;
    let env_vars = build_command_env();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| OnlyError::runtime(format!("failed to start task runtime: {error}")))?;
    let local = tokio::task::LocalSet::new();
    let status = local.block_on(
        &runtime,
        deno_task_shell::execute(
            parsed,
            env_vars,
            working_dir.to_path_buf(),
            Default::default(),
            deno_task_shell::KillSignal::default(),
        ),
    );

    Ok(ExitCode::from(status as u8))
}

fn run_with_system_shell(
    program: &str,
    arg: &str,
    command: &str,
    working_dir: &Path,
) -> Result<ExitCode> {
    let mut process = Command::new(program);
    process
        .current_dir(working_dir)
        .arg(arg)
        .arg(command)
        .envs(build_command_env());

    let status = process.status().map_err(|source| {
        OnlyError::io_with_path("failed to start shell command", program.into(), source)
    })?;

    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}

fn build_command_env() -> HashMap<OsString, OsString> {
    let mut env_vars = std::env::vars_os().collect::<HashMap<_, _>>();
    env_vars
        .entry(OsString::from("INIT_CWD"))
        .or_insert_with(|| std::env::current_dir().unwrap_or_default().into_os_string());
    env_vars
}

fn power_shell_command() -> &'static str {
    if cfg!(windows) {
        "powershell.exe"
    } else {
        "powershell"
    }
}
