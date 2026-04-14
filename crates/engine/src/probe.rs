use crate::path_lookup::command_exists_in_path;

pub(crate) fn probe_matches(kind: &str, argument: &str) -> bool {
    if argument.is_empty() {
        return false;
    }

    match kind {
        "os" => std::env::consts::OS == argument,
        "arch" => std::env::consts::ARCH == argument,
        "env" => std::env::var_os(argument).is_some(),
        "has" => command_exists_in_path(argument),
        _ => false,
    }
}
