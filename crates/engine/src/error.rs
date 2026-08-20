use std::fmt;

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
        reason: String,
    },
    CommandBlockFailed {
        task: String,
        step: usize,
        total: usize,
        reason: String,
    },
    CommandBlockStartFailed {
        shell: String,
        source: Box<EngineError>,
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
                command: _,
                reason,
            } => write!(
                f,
                "task '{task}' failed at step [{step}/{total}] with {}",
                reason
            ),
            Self::CommandBlockFailed {
                task,
                step,
                total,
                reason,
            } => write!(
                f,
                "task '{task}' failed at step [{step}/{total}] with {}",
                reason
            ),
            Self::CommandBlockStartFailed { shell, source } => {
                write!(
                    f,
                    "could not start command block with shell '{shell}'\n{source}"
                )
            }
            Self::Interpolation(message) => f.write_str(message),
            Self::ShellNotFound(message) => f.write_str(message),
            Self::UnsupportedShell(shell) => write!(f, "shell '{shell}' is not supported"),
            Self::Runtime(message) => f.write_str(message),
            Self::Io {
                message,
                path,
                source,
            } => write!(f, "{message}: {path}: {source}"),
        }
    }
}

pub(crate) fn command_block_failed(
    task: &str,
    step_index: usize,
    step_total: usize,
    reason: &str,
) -> EngineError {
    EngineError::CommandBlockFailed {
        task: task.to_string(),
        step: step_index,
        total: step_total,
        reason: reason.to_string(),
    }
}

impl std::error::Error for EngineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::CommandBlockStartFailed { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

pub(crate) fn command_block_start_failed(shell: &str, source: EngineError) -> EngineError {
    match source {
        EngineError::ShellNotFound(_)
        | EngineError::UnsupportedShell(_)
        | EngineError::Io { .. } => EngineError::CommandBlockStartFailed {
            shell: shell.to_string(),
            source: Box::new(source),
        },
        other => other,
    }
}

pub(crate) fn command_failed(
    task: &str,
    step_index: usize,
    step_total: usize,
    command: &str,
    reason: &str,
) -> EngineError {
    EngineError::CommandFailed {
        task: task.to_string(),
        step: step_index,
        total: step_total,
        command: command.to_string(),
        reason: reason.to_string(),
    }
}
