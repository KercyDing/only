use std::fmt;
use std::process::ExitCode;

/// Engine-level runtime and host execution errors.
///
/// Args:
/// None.
///
/// Returns:
/// Stable typed error values for planner/runtime consumers.
#[derive(Debug)]
pub enum EngineError {
    CommandFailed {
        task: String,
        step: usize,
        total: usize,
        command: String,
        code: ExitCode,
    },
    Interpolation(String),
    ShellNotFound(String),
    UnsupportedShell(String),
    Runtime(String),
    Io {
        message: &'static str,
        path: String,
        source: std::io::Error,
    },
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandFailed {
                task,
                step,
                total,
                command,
                code,
            } => write!(
                f,
                "task '{task}' failed at step [{step}/{total}] while running `{command}` with exit code {code:?}"
            ),
            Self::Interpolation(message) => f.write_str(message),
            Self::ShellNotFound(message) => f.write_str(message),
            Self::UnsupportedShell(shell) => write!(f, "unsupported shell '{shell}'"),
            Self::Runtime(message) => f.write_str(message),
            Self::Io {
                message,
                path,
                source,
            } => write!(f, "{message}: {path}: {source}"),
        }
    }
}

impl std::error::Error for EngineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub(crate) fn command_failed(
    task: &str,
    step_index: usize,
    step_total: usize,
    command: &str,
    code: ExitCode,
) -> EngineError {
    EngineError::CommandFailed {
        task: task.to_string(),
        step: step_index,
        total: step_total,
        command: command.to_string(),
        code,
    }
}
