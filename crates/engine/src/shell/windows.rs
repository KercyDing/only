use std::path::Path;
use std::sync::mpsc::Sender;

use super::run_with_deno_task_shell_pipe;
use crate::EngineError;
use crate::process::{OutputChunk, TerminalBackend, TerminalContext};

pub(super) fn run_deno(
    command: &str,
    working_dir: &Path,
    output: Sender<OutputChunk>,
    terminal: &TerminalContext,
) -> Result<crate::process::CommandStatus, EngineError> {
    if terminal.backend == TerminalBackend::Pty {
        let captured = terminal.for_captured_output();
        return run_with_deno_task_shell_pipe(command, working_dir, output, &captured);
    }
    run_with_deno_task_shell_pipe(command, working_dir, output, terminal)
}

pub(super) fn power_shell_command() -> &'static str {
    "powershell.exe"
}
