//! Core library entry points for the `only` binary and future hosts.

pub mod cli;
pub mod config;
pub mod diagnostic;
pub mod model;
pub mod parser;
pub mod planner;
pub mod runtime;
pub mod support;

use std::path::Path;
use std::process::ExitCode;

pub use cli::args::CliInput;
pub use config::discover::{DiscoveredOnlyfile, discover_onlyfile};
pub use diagnostic::error::{OnlyError, Result};
pub use model::Onlyfile;

/// Runs the default CLI entry point.
///
/// Returns:
/// Process exit code for the current invocation.
pub fn run() -> ExitCode {
    match run_with(cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

/// Runs the application with pre-parsed CLI input.
///
/// Args:
/// cli: Normalized command-line input.
///
/// Returns:
/// Process exit code for the requested action.
///
/// Edge Cases:
/// Returns an error when `Onlyfile` discovery or loading fails.
pub fn run_with(cli: CliInput) -> Result<ExitCode> {
    let discovered = load_onlyfile(cli.onlyfile_path.as_deref())?;

    if cli.print_discovered_path {
        println!("{}", discovered.path.display());
    }

    Ok(ExitCode::SUCCESS)
}

/// Loads and parses the requested `Onlyfile`.
///
/// Args:
/// path: Optional explicit path to the file.
///
/// Returns:
/// The discovered file metadata and parsed document.
pub fn load_onlyfile(path: Option<&Path>) -> Result<DiscoveredOnlyfile> {
    let discovered = discover_onlyfile(path)?;
    let document = parse_onlyfile(&discovered.contents)?;

    Ok(DiscoveredOnlyfile {
        document,
        ..discovered
    })
}

/// Parses `Onlyfile` source text into the domain model.
///
/// Args:
/// content: Raw file contents.
///
/// Returns:
/// Parsed `Onlyfile` model.
pub fn parse_onlyfile(content: &str) -> Result<Onlyfile> {
    parser::parse_onlyfile(content)
}

/// Placeholder for the future planning stage.
///
/// Args:
/// _onlyfile: Parsed source document.
/// _cli: Normalized CLI input.
///
/// Returns:
/// A not-yet-implemented error until planner exists.
pub fn build_execution_plan(_onlyfile: &Onlyfile, _cli: &CliInput) -> Result<()> {
    Err(OnlyError::unsupported(
        "execution planner is not implemented yet",
    ))
}

/// Placeholder for the future runtime stage.
///
/// Args:
/// _plan: Resolved execution plan.
///
/// Returns:
/// A not-yet-implemented error until runtime exists.
pub fn run_plan(_plan: &()) -> Result<ExitCode> {
    Err(OnlyError::unsupported("runtime is not implemented yet"))
}
