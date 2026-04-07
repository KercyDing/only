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
///
/// Returns:
/// Process exit code produced by the shell.
pub fn run_command(command: &str, working_dir: &Path, shell: ShellKind) -> Result<ExitCode> {
    match shell {
        ShellKind::Deno => run_with_deno_task_shell(command, working_dir),
        ShellKind::Sh => run_with_system_shell("sh", "-c", command, working_dir),
        ShellKind::Bash => run_with_system_shell("bash", "-c", command, working_dir),
        ShellKind::PowerShell => {
            run_with_system_shell(power_shell_command(), "-Command", command, working_dir)
        }
        ShellKind::Pwsh => run_with_system_shell("pwsh", "-Command", command, working_dir),
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
