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
        None => discover_from_current_dir_or_parents()?,
    };

    let contents = fs::read_to_string(&path).map_err(|source| {
        OnlyError::io_with_path("failed to read Onlyfile", path.clone(), source)
    })?;

    let base_dir = path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);

    let base_dir = if base_dir.is_absolute() {
        base_dir
    } else {
        std::env::current_dir()
            .map_err(OnlyError::cwd)?
            .join(&base_dir)
    };

    Ok(DiscoveredOnlyfile {
        path,
        base_dir,
        contents,
        document: Onlyfile::default(),
    })
}

fn discover_from_current_dir_or_parents() -> Result<PathBuf> {
    let cwd = std::env::current_dir().map_err(OnlyError::cwd)?;

    discover_from_dir(&cwd).ok_or_else(|| {
        OnlyError::not_found("No Onlyfile found in current directory or any parent.".to_string())
    })
}

fn discover_from_dir(start_dir: &Path) -> Option<PathBuf> {
    start_dir.ancestors().find_map(|directory| {
        ONLYFILE_CANDIDATES
            .iter()
            .map(|candidate| directory.join(candidate))
            .find(|path| path.is_file())
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::config::discover::discover_from_dir;

    #[test]
    fn discovers_onlyfile_in_parent_directory() {
        let root = std::env::temp_dir().join(format!("only-discover-{}", std::process::id()));
        let nested = root.join("a/b");
        fs::create_dir_all(&nested).expect("nested dir should be created");
        fs::write(root.join("Onlyfile"), "test():\n    true\n")
            .expect("Onlyfile should be written");

        let discovered = discover_from_dir(&nested).expect("parent Onlyfile should be found");
        assert_eq!(discovered, root.join("Onlyfile"));

        fs::remove_dir_all(&root).expect("temp tree should be removed");
    }
}
