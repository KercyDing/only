use std::process::{Command, ExitCode};

use crate::diagnostic::error::{OnlyError, Result};

/// Runs a single shell command and propagates its exit status.
///
/// Args:
/// command: Shell command text to execute.
///
/// Returns:
/// Process exit code produced by the shell.
pub fn run_command(command: &str) -> Result<ExitCode> {
    let status = Command::new("/bin/sh")
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
/// command: Rendered shell command.
/// code: Exit code returned by the shell.
///
/// Returns:
/// Structured runtime error with execution context.
pub fn command_failed(task: &str, command: &str, code: ExitCode) -> OnlyError {
    OnlyError::runtime(format!(
        "task '{task}' failed while running `{command}` with exit code {:?}",
        code
    ))
}
