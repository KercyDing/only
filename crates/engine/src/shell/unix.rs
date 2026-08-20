use std::path::Path;
use std::sync::mpsc::Sender;

use super::run_with_deno_task_shell_pipe;
use crate::EngineError;
use crate::process::{OutputChunk, TerminalContext};

pub(super) fn run_deno(
    command: &str,
    working_dir: &Path,
    output: Sender<OutputChunk>,
    terminal: &TerminalContext,
) -> Result<crate::process::CommandStatus, EngineError> {
    #[cfg(unix)]
    if terminal.backend == crate::process::TerminalBackend::Pty
        && let Some(status) =
            super::run_with_deno_task_shell_pty(command, working_dir, output.clone(), terminal)?
    {
        return Ok(status);
    }
    run_with_deno_task_shell_pipe(command, working_dir, output, terminal)
}

pub(super) fn power_shell_command() -> &'static str {
    "powershell"
}
