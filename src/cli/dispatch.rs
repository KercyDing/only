use std::process::ExitCode;

use crate::cli::args::CliInput;
use crate::diagnostic::error::Result;

/// Dispatches normalized CLI input into the library entry point.
///
/// Args:
/// cli: Parsed command-line input.
///
/// Returns:
/// Exit code for the invocation.
pub fn dispatch(cli: CliInput) -> Result<ExitCode> {
    crate::run_with(cli)
}
