use clap::{Arg, ArgAction, Command};

/// Builds the top-level CLI command.
///
/// Returns:
/// Configured clap command for the `only` binary.
pub fn build() -> Command {
    Command::new("only")
        .about("A minimalist, deterministic task runner")
        .arg(
            Arg::new("onlyfile")
                .long("onlyfile")
                .value_name("PATH")
                .help("Use a specific Onlyfile path"),
        )
        .arg(
            Arg::new("print-discovered-path")
                .long("print-discovered-path")
                .action(ArgAction::SetTrue)
                .help("Print the resolved Onlyfile path and exit successfully"),
        )
        .arg(
            Arg::new("positionals")
                .value_name("ARGS")
                .num_args(1..)
                .action(ArgAction::Append)
                .help("Task target and optional task arguments"),
        )
        .arg(
            Arg::new("set")
                .long("set")
                .value_name("NAME=VALUE")
                .action(ArgAction::Append)
                .help("Override a target task parameter"),
        )
}
