use std::path::Path;

/// Checks whether a named command exists in the system PATH.
///
/// Args:
/// name: Command name to look up.
///
/// Returns:
/// `true` when the command is found in any PATH directory.
pub(crate) fn command_exists_in_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| exists_in_dir(&directory, name))
    })
}

fn exists_in_dir(directory: &Path, name: &str) -> bool {
    let candidate = directory.join(name);
    if candidate.is_file() {
        return true;
    }

    #[cfg(windows)]
    {
        let has_extension = Path::new(name).extension().is_some();
        if has_extension {
            return false;
        }

        let extensions = std::env::var_os("PATHEXT")
            .and_then(|value| value.into_string().ok())
            .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string());

        extensions
            .split(';')
            .map(str::trim)
            .filter(|extension| !extension.is_empty())
            .any(|extension| directory.join(format!("{name}{extension}")).is_file())
    }

    #[cfg(not(windows))]
    {
        false
    }
}
