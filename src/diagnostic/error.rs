use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, OnlyError>;

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
    pub fn cwd(source: io::Error) -> Self {
        Self::Io {
            message: "failed to read current directory",
            path: None,
            source,
        }
    }

    pub fn io_with_path(message: &'static str, path: PathBuf, source: io::Error) -> Self {
        Self::Io {
            message,
            path: Some(path),
            source,
        }
    }

    pub fn not_found(message: String) -> Self {
        Self::NotFound(message)
    }

    pub fn parse(message: impl Into<String>) -> Self {
        Self::Parse(message.into())
    }

    pub fn unsupported(message: &'static str) -> Self {
        Self::Unsupported(message)
    }

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
