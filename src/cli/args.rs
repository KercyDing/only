use std::path::PathBuf;

use clap::ArgMatches;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliInput {
    pub onlyfile_path: Option<PathBuf>,
    pub print_discovered_path: bool,
    pub task: Option<String>,
    pub subtask: Option<String>,
    pub parameter_overrides: Vec<(String, String)>,
}

impl From<ArgMatches> for CliInput {
    fn from(matches: ArgMatches) -> Self {
        Self {
            onlyfile_path: matches.get_one::<String>("onlyfile").map(PathBuf::from),
            print_discovered_path: matches.get_flag("print-discovered-path"),
            task: matches.get_one::<String>("task").cloned(),
            subtask: matches.get_one::<String>("subtask").cloned(),
            parameter_overrides: matches
                .get_many::<String>("set")
                .into_iter()
                .flatten()
                .filter_map(|item| item.split_once('='))
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect(),
        }
    }
}
