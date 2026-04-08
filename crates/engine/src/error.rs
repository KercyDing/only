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
            Self::Runtime(_) => None,
            Self::Io { source, .. } => Some(source),
        }
    }
}

pub(crate) fn command_failed(
    task: &str,
    step_index: usize,
    step_total: usize,
    command: &str,
    code: std::process::ExitCode,
) -> EngineError {
    EngineError::Runtime(format!(
        "task '{task}' failed at step [{step_index}/{step_total}] while running `{command}` with exit code {:?}",
        code
    ))
}
