use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic::error::{OnlyError, Result};
use crate::model::Onlyfile;

const ONLYFILE_CANDIDATES: [&str; 2] = ["Onlyfile", "onlyfile"];

#[derive(Debug, Clone)]
pub struct DiscoveredOnlyfile {
    pub path: PathBuf,
    pub base_dir: PathBuf,
    pub contents: String,
    pub document: Onlyfile,
}

/// Discovers and reads the target `Onlyfile`.
///
/// Args:
/// explicit_path: Optional explicit file path.
///
/// Returns:
/// File metadata and raw contents.
pub fn discover_onlyfile(explicit_path: Option<&Path>) -> Result<DiscoveredOnlyfile> {
    let path = match explicit_path {
        Some(path) => path.to_path_buf(),
        None => discover_in_current_dir()?,
    };

    let contents = fs::read_to_string(&path).map_err(|source| {
        OnlyError::io_with_path("failed to read Onlyfile", path.clone(), source)
    })?;

    let base_dir = path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);

    Ok(DiscoveredOnlyfile {
        path,
        base_dir,
        contents,
        document: Onlyfile::default(),
    })
}

fn discover_in_current_dir() -> Result<PathBuf> {
    let cwd = std::env::current_dir().map_err(OnlyError::cwd)?;

    for candidate in ONLYFILE_CANDIDATES {
        let path = cwd.join(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }

    Err(OnlyError::not_found(format!(
        "no Onlyfile found in {}",
        cwd.display()
    )))
}
