pub(crate) fn probe_matches(kind: &str, argument: &str) -> bool {
    if argument.is_empty() {
        return false;
    }

    match kind {
        "os" => std::env::consts::OS == argument,
        "arch" => std::env::consts::ARCH == argument,
        "env" => std::env::var_os(argument).is_some(),
        "has" => command_exists(argument),
        _ => false,
    }
}

fn command_exists(command: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| command_exists_in_dir(&directory, command))
    })
}

fn command_exists_in_dir(directory: &std::path::Path, command: &str) -> bool {
    let candidate = directory.join(command);
    if candidate.is_file() {
        return true;
    }

    #[cfg(windows)]
    {
        let has_extension = std::path::Path::new(command).extension().is_some();
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
            .any(|extension| directory.join(format!("{command}{extension}")).is_file())
    }

    #[cfg(not(windows))]
    {
        false
    }
}
