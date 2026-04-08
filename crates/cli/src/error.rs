use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, OnlyError>;

/// Host-facing application error.
///
/// Args:
/// None.
///
/// Returns:
/// Structured error variants for CLI parsing, planning, and runtime failures.
///
/// Edge Cases:
/// I/O errors may or may not carry a filesystem path, depending on the caller.
#[derive(Debug)]
pub enum OnlyError {
    Io {
        message: &'static str,
        path: Option<PathBuf>,
        source: io::Error,
    },
    NotFound(String),
    Parse(String),
    Runtime(String),
    Unsupported(&'static str),
}

impl OnlyError {
    /// Creates an error for current-directory lookup failures.
    ///
    /// Args:
    /// source: Original I/O error.
    ///
    /// Returns:
    /// Wrapped host error without a path.
    pub fn cwd(source: io::Error) -> Self {
        Self::Io {
            message: "failed to read current directory",
            path: None,
            source,
        }
    }

    /// Creates an I/O error bound to a concrete path.
    ///
    /// Args:
    /// message: Stable host-facing message prefix.
    /// path: Filesystem path associated with the failure.
    /// source: Original I/O error.
    ///
    /// Returns:
    /// Wrapped host error with a path payload.
    pub fn io_with_path(message: &'static str, path: PathBuf, source: io::Error) -> Self {
        Self::Io {
            message,
            path: Some(path),
            source,
        }
    }

    /// Creates a not-found error.
    ///
    /// Args:
    /// message: Human-readable failure message.
    ///
    /// Returns:
    /// Missing-resource error variant.
    pub fn not_found(message: String) -> Self {
        Self::NotFound(message)
    }

    /// Creates a parse-phase error.
    ///
    /// Args:
    /// message: Human-readable failure message.
    ///
    /// Returns:
    /// Parse error variant.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::Parse(message.into())
    }

    /// Creates an unsupported-feature error.
    ///
    /// Args:
    /// message: Stable host-facing message.
    ///
    /// Returns:
    /// Unsupported error variant.
    pub fn unsupported(message: &'static str) -> Self {
        Self::Unsupported(message)
    }

    /// Creates a runtime error.
    ///
    /// Args:
    /// message: Human-readable failure message.
    ///
    /// Returns:
    /// Runtime error variant.
    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for OnlyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                message,
                path,
                source,
            } => {
                if let Some(path) = path {
                    write!(f, "{message}: {}: {source}", path.display())
                } else {
                    write!(f, "{message}: {source}")
                }
            }
            Self::NotFound(message) | Self::Parse(message) | Self::Runtime(message) => {
                f.write_str(message)
            }
            Self::Unsupported(message) => f.write_str(message),
        }
    }
}

impl StdError for OnlyError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::NotFound(_) | Self::Parse(_) | Self::Runtime(_) | Self::Unsupported(_) => None,
        }
    }
}
