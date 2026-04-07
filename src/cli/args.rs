use std::path::PathBuf;

use clap::ArgMatches;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliInput {
    pub onlyfile_path: Option<PathBuf>,
    pub print_discovered_path: bool,
}

impl From<ArgMatches> for CliInput {
    fn from(matches: ArgMatches) -> Self {
        Self {
            onlyfile_path: matches.get_one::<String>("onlyfile").map(PathBuf::from),
            print_discovered_path: matches.get_flag("print-discovered-path"),
        }
    }
}
