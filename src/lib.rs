//! Core library entry points for the `only` binary and future hosts.

use anstyle::{AnsiColor, Style};

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
pub use model::{Directive, Onlyfile};
pub use planner::ExecutionPlan;

/// Runs the default CLI entry point with two-phase parsing.
///
/// Phase 1: Parse global options (-f, -p) to discover Onlyfile.
/// Phase 2: Build dynamic subcommands from Onlyfile and parse task.
///
/// Returns:
/// Process exit code for the current invocation.
pub fn run() -> ExitCode {
    match run_inner() {
        Ok(code) => code,
        Err(OnlyError::NotFound(message)) => {
            eprintln!("Error: {message}");
            eprintln!("{}", render_help_hint());
            ExitCode::from(2)
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn run_inner() -> Result<ExitCode> {
    // Phase 1: Parse global options only
    let partial = cli::parse_global_options()?;

    if partial.top_level_help_requested {
        println!("{}", cli::app::render_global_help().ansi());
        return Ok(ExitCode::SUCCESS);
    }

    // Discover Onlyfile
    let discovered = load_onlyfile(partial.onlyfile_path.as_deref())?;

    if partial.print_discovered_path {
        println!("{}", discovered.path.display());
        return Ok(ExitCode::SUCCESS);
    }

    // Phase 2: Build dynamic CLI with subcommands and parse
    let cli = cli::parse_with_onlyfile(&discovered.document)?;

    if cli.task_path.is_empty() {
        print!("{}", cli::app::render_available_tasks(&discovered.document));
        return Ok(ExitCode::SUCCESS);
    }

    if let [namespace_name] = cli.task_path.as_slice()
        && let Some(namespace) = discovered
            .document
            .namespaces
            .iter()
            .find(|namespace| namespace.name == *namespace_name)
    {
        println!("{}", cli::app::render_namespace_help(namespace).ansi());
        return Ok(ExitCode::SUCCESS);
    }

    let plan = build_execution_plan(&discovered.document, &cli)?;
    run_plan(&plan)
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
        return Ok(ExitCode::SUCCESS);
    }

    let plan = build_execution_plan(&discovered.document, &cli)?;
    run_plan(&plan)
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

/// Builds a resolved execution plan from parsed input and CLI target.
///
/// Args:
/// onlyfile: Parsed source document.
/// cli: Normalized CLI input.
///
/// Returns:
/// Resolved execution plan.
pub fn build_execution_plan(onlyfile: &Onlyfile, cli: &CliInput) -> Result<ExecutionPlan> {
    planner::build_execution_plan(onlyfile, cli)
}

/// Runs the resolved execution plan.
///
/// Args:
/// plan: Resolved execution plan.
///
/// Returns:
/// Process exit code from the first failing command or overall success.
pub fn run_plan(plan: &ExecutionPlan) -> Result<ExitCode> {
    runtime::engine::run_plan(plan)
}

fn render_help_hint() -> String {
    let style = Style::new()
        .fg_color(Some(AnsiColor::BrightCyan.into()))
        .bold();

    format!(
        "Run '{}only --help{}' to view usage.",
        style.render(),
        style.render_reset()
    )
}
