use crate::args::{CliInput, parse_global_options, parse_with_onlyfile};
use crate::compile::{compile_for_cli_input_in_dir, ensure_no_error_diagnostics};
use crate::discover::discover_onlyfile;
use crate::error::{OnlyError, Result};
use crate::render::{
    render_available_tasks, render_error_message, render_global_help, render_help_hint,
    render_namespace_help,
};
use only_engine::ExecutionPlan;
use only_semantic::DocumentAst;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Loaded `Onlyfile` source plus parsed semantic document.
///
/// Args:
/// None.
///
/// Returns:
/// Host-side file metadata, raw contents, and parsed semantic document.
#[derive(Debug, Clone)]
pub struct LoadedOnlyfile {
    pub path: PathBuf,
    pub base_dir: PathBuf,
    pub contents: String,
    pub document: DocumentAst,
}

/// Runs the default CLI entry point with two-phase parsing.
///
/// Args:
/// None.
///
/// Returns:
/// Process exit code for the current invocation.
pub fn run() -> ExitCode {
    match run_inner() {
        Ok(code) => code,
        Err(OnlyError::NotFound(message)) => {
            eprintln!("{}", render_error_message(&message));
            eprintln!("{}", render_help_hint());
            ExitCode::from(2)
        }
        Err(error) => {
            eprintln!("{}", render_error_message(&error.to_string()));
            ExitCode::from(2)
        }
    }
}

/// Returns the published CLI version string.
///
/// Args:
/// None.
///
/// Returns:
/// Static package version text.
pub fn version_string() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Runs the application with pre-parsed CLI input.
///
/// Args:
/// cli: Normalized command-line input.
///
/// Returns:
/// Process exit code for the requested action.
pub fn run_with(cli: CliInput) -> Result<ExitCode> {
    let discovered = discover_onlyfile(cli.onlyfile_path.as_deref())?;

    if cli.print_discovered_path {
        println!("{}", discovered.path.display());
        return Ok(ExitCode::SUCCESS);
    }

    let compiled = compile_for_cli_input_in_dir(&discovered.contents, &cli, discovered.base_dir)?;
    only_engine::run_plan(&compiled.plan).map_err(|error| OnlyError::runtime(error.to_string()))
}

/// Loads and parses the requested Onlyfile.
///
/// Args:
/// path: Optional explicit file path.
///
/// Returns:
/// Discovered Onlyfile metadata and parsed document.
pub fn load_onlyfile(path: Option<&Path>) -> Result<LoadedOnlyfile> {
    let discovered = discover_onlyfile(path)?;
    let document = parse_onlyfile(&discovered.contents)?;

    Ok(LoadedOnlyfile {
        path: discovered.path,
        base_dir: discovered.base_dir,
        contents: discovered.contents,
        document,
    })
}

/// Parses Onlyfile source text into the current semantic document.
///
/// Args:
/// content: Raw file contents.
///
/// Returns:
/// Parsed semantic document.
pub fn parse_onlyfile(content: &str) -> Result<DocumentAst> {
    let compiled = only_semantic::compile_document(content);
    ensure_no_error_diagnostics(&compiled.diagnostics)?;
    Ok(compiled.document)
}

/// Builds an execution plan for the requested CLI invocation from raw source text.
///
/// Args:
/// source: Raw Onlyfile source text.
/// cli: Normalized command-line input.
///
/// Returns:
/// Resolved execution plan.
pub fn build_execution_plan(source: &str, cli: &CliInput) -> Result<ExecutionPlan> {
    Ok(crate::compile::compile_for_cli_input(source, cli)?.plan)
}

/// Builds an execution plan for the requested CLI invocation from raw source text and working directory.
///
/// Args:
/// source: Raw Onlyfile source text.
/// cli: Normalized command-line input.
/// working_dir: Directory used during runtime execution.
///
/// Returns:
/// Resolved execution plan.
pub fn build_execution_plan_in_dir(
    source: &str,
    cli: &CliInput,
    working_dir: PathBuf,
) -> Result<ExecutionPlan> {
    Ok(compile_for_cli_input_in_dir(source, cli, working_dir)?.plan)
}

/// Runs the resolved execution plan.
///
/// Args:
/// plan: Resolved execution plan.
///
/// Returns:
/// Process exit code from the executed plan.
pub fn run_plan(plan: &ExecutionPlan) -> Result<ExitCode> {
    only_engine::run_plan(plan).map_err(|error| OnlyError::runtime(error.to_string()))
}

fn run_inner() -> Result<ExitCode> {
    let partial = parse_global_options()?;

    if partial.top_level_help_requested {
        println!("{}", render_global_help().ansi());
        return Ok(ExitCode::SUCCESS);
    }

    if partial.top_level_version_requested {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(ExitCode::SUCCESS);
    }

    let discovered = load_onlyfile(partial.onlyfile_path.as_deref())?;

    if partial.print_discovered_path {
        println!("{}", discovered.path.display());
        return Ok(ExitCode::SUCCESS);
    }

    let cli = parse_with_onlyfile(&discovered.document)?;

    if cli.task_path.is_empty() {
        print!("{}", render_available_tasks(&discovered.document));
        return Ok(ExitCode::SUCCESS);
    }

    if let [namespace_name] = cli.task_path.as_slice()
        && let Some(namespace) = discovered
            .document
            .namespaces
            .iter()
            .find(|namespace| namespace.name == *namespace_name)
    {
        println!(
            "{}",
            render_namespace_help(&discovered.document, namespace).ansi()
        );
        return Ok(ExitCode::SUCCESS);
    }

    let compiled = compile_for_cli_input_in_dir(&discovered.contents, &cli, discovered.base_dir)?;
    only_engine::run_plan(&compiled.plan).map_err(|error| OnlyError::runtime(error.to_string()))
}
