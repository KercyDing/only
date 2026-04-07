use std::path::Path;
use std::process::{Command, ExitCode};

use crate::diagnostic::error::{OnlyError, Result};

/// Runs a single shell command and propagates its exit status.
///
/// Args:
/// command: Shell command text to execute.
/// working_dir: Directory used as the shell working directory.
///
/// Returns:
/// Process exit code produced by the shell.
pub fn run_command(command: &str, working_dir: &Path) -> Result<ExitCode> {
    let status = Command::new("/bin/sh")
        .current_dir(working_dir)
        .arg("-c")
        .arg(command)
        .status()
        .map_err(|source| {
            OnlyError::io_with_path("failed to start shell command", "/bin/sh".into(), source)
        })?;

    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
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
