use crate::path_lookup::command_exists_in_path;
use only_semantic::GuardKind;

pub(crate) fn probe_matches(kind: &GuardKind, argument: &str) -> bool {
    if argument.is_empty() {
        return false;
    }

    match kind {
        GuardKind::Os => std::env::consts::OS == argument,
        GuardKind::Arch => std::env::consts::ARCH == argument,
        GuardKind::Env => std::env::var_os(argument).is_some(),
        GuardKind::Has => command_exists_in_path(argument),
        GuardKind::Unknown(_) => false,
    }
}
