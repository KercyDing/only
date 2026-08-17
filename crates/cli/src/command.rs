use crate::args::{CliInput, parse_global_options, parse_with_onlyfile};
use crate::compile::{
    CliCompileResult, compile_for_cli_input_in_dir, ensure_no_error_diagnostics, resolve_target,
};
use crate::discover::discover_onlyfile;
use crate::error::{OnlyError, Result};
use crate::render::{
    render_available_tasks, render_error_message, render_global_help, render_help_hint,
    render_namespace_help,
};
use only_engine::{
    ExecutionPlan, RuntimeOptions, render_command, run_plan_with_options, select_root_task_variant,
};
use only_semantic::{DocumentAst, GuardAst, ShellKind, TaskAst};
use only_syntax::format_source;
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

    ensure_dry_run_has_target(&cli)?;

    let compiled = compile_for_cli_input_in_dir(&discovered.contents, &cli, discovered.base_dir)?;
    if cli.dry_run {
        println!("{}", render_dry_run_for_cli(&compiled, &cli)?);
        return Ok(ExitCode::SUCCESS);
    }
    run_compiled_plan(&compiled.plan, &cli)
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
    let compiled = only_semantic::compile_document_for_runner(content, env!("CARGO_PKG_VERSION"));
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

fn ensure_dry_run_has_target(cli: &CliInput) -> Result<()> {
    if cli.dry_run_full && !cli.dry_run {
        return Err(OnlyError::parse(
            "--full only works with --dry-run\nhelp: use `only --dry-run --full <task>`",
        ));
    }

    if cli.dry_run && cli.task_path.is_empty() {
        return Err(OnlyError::parse(
            "--dry-run needs a task\nhelp: use `only --dry-run <task>`",
        ));
    }

    Ok(())
}

fn render_dry_run_for_cli(compiled: &CliCompileResult, cli: &CliInput) -> Result<String> {
    let (target, _) = resolve_target(&compiled.compiled, cli)?;
    let variant = select_root_task_variant(&compiled.compiled.document, &target)
        .map_err(|error| OnlyError::runtime(error.to_string()))?;
    render_dry_run(&compiled.plan, variant, cli.dry_run_full)
}

fn run_compiled_plan(plan: &ExecutionPlan, cli: &CliInput) -> Result<ExitCode> {
    run_plan_with_options(plan, RuntimeOptions { quiet: cli.quiet })
        .map_err(|error| OnlyError::runtime(error.to_string()))
}

fn render_dry_run(plan: &ExecutionPlan, variant: &TaskAst, full: bool) -> Result<String> {
    let mut output = String::new();
    let header = format!("Dry run: {}", render_task_variant(variant));
    push_line(&mut output, &header);

    let stages = plan_stages(plan);
    let mut index = 0usize;
    while index < stages.len() {
        let (stage, stage_nodes) = stages[index];
        let stage_last = index + 1 == stages.len();
        let stage_label = render_stage_label(stage, stage_nodes.len());
        push_tree_line(&mut output, "", stage_last, &stage_label);
        let stage_prefix = if stage_last { "   " } else { "│  " };

        for (node_index, node) in stage_nodes.iter().enumerate() {
            let node_last = node_index + 1 == stage_nodes.len();
            let has_block = node.steps.iter().any(only_engine::ExecutionStep::is_block);
            let node_label = if full || has_block {
                node.name.to_string()
            } else {
                render_node_summary(&node.name, node.steps.len())
            };
            push_tree_line(&mut output, stage_prefix, node_last, &node_label);
            if !full && !has_block {
                continue;
            }

            let command_prefix = if node_last {
                format!("{stage_prefix}   ")
            } else {
                format!("{stage_prefix}│  ")
            };

            let shell = node
                .shell
                .as_ref()
                .map(|shell| shell.kind.as_str())
                .or_else(|| plan.shell.as_ref().map(ShellKind::as_str))
                .unwrap_or(ShellKind::Deno.as_str());
            for (step_index, step) in node.steps.iter().enumerate() {
                let rendered = render_command(step.source(), &node.params)
                    .map_err(|error| OnlyError::runtime(error.to_string()))?;
                let step_last = step_index + 1 == node.steps.len();
                match step {
                    only_engine::ExecutionStep::Command(_) => {
                        push_tree_line(&mut output, &command_prefix, step_last, &rendered);
                    }
                    only_engine::ExecutionStep::CommandBlock { line_count, .. } if !full => {
                        push_tree_line(
                            &mut output,
                            &command_prefix,
                            step_last,
                            &format!("block ({shell}, {line_count} lines)"),
                        );
                    }
                    only_engine::ExecutionStep::CommandBlock { .. } => {
                        push_tree_line(
                            &mut output,
                            &command_prefix,
                            step_last,
                            &format!("block ({shell})"),
                        );
                        let line_prefix = if step_last {
                            format!("{command_prefix}   ")
                        } else {
                            format!("{command_prefix}│  ")
                        };
                        let lines = rendered.lines().collect::<Vec<_>>();
                        for (line_index, line) in lines.iter().enumerate() {
                            push_tree_line(
                                &mut output,
                                &line_prefix,
                                line_index + 1 == lines.len(),
                                line,
                            );
                        }
                    }
                }
            }
        }

        index += 1;
    }

    Ok(output.trim_end().to_string())
}

fn plan_stages(plan: &ExecutionPlan) -> Vec<(usize, &[only_engine::ExecutionNode])> {
    let mut stages = Vec::new();
    let mut index = 0usize;

    while index < plan.nodes.len() {
        let stage = plan.nodes[index].stage;
        let stage_start = index;
        while index < plan.nodes.len() && plan.nodes[index].stage == stage {
            index += 1;
        }
        stages.push((stage, &plan.nodes[stage_start..index]));
    }

    stages
}

fn render_node_summary(name: &str, command_count: usize) -> String {
    let noun = if command_count == 1 {
        "command"
    } else {
        "commands"
    };
    format!("{name} ({command_count} {noun})")
}

fn render_stage_label(stage: usize, node_count: usize) -> String {
    if node_count > 1 {
        format!("stage {} (parallel)", stage + 1)
    } else {
        format!("stage {}", stage + 1)
    }
}

fn push_tree_line(output: &mut String, prefix: &str, is_last: bool, text: &str) {
    let mut line = String::new();
    line.push_str(prefix);
    line.push_str(if is_last { "└─ " } else { "├─ " });
    line.push_str(text);
    push_line(output, &line);
}

fn push_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
}

fn render_task_variant(task: &TaskAst) -> String {
    let mut variant = match &task.namespace {
        Some(namespace) => format!("{namespace}.{}", task.signature()),
        None => task.signature().to_string(),
    };

    for guard in &task.guards {
        variant.push_str(" ? ");
        variant.push_str(&render_guard(guard));
    }

    variant
}

fn render_guard(guard: &GuardAst) -> String {
    format!("@{}(\"{}\")", guard.kind, guard.argument)
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

    if partial.top_level_upgrade_requested {
        return crate::upgrade::run_upgrade();
    }

    let discovered = discover_onlyfile(partial.onlyfile_path.as_deref())?;

    if partial.print_discovered_path {
        println!("{}", discovered.path.display());
        return Ok(ExitCode::SUCCESS);
    }

    if partial.format_requested || partial.format_check {
        if partial.format_check && !partial.format_requested {
            return Err(OnlyError::parse("--check only works with --fmt"));
        }
        let _ = parse_onlyfile(&discovered.contents)?;
        let formatted = format_source(&discovered.contents).map_err(OnlyError::parse)?;
        if partial.format_check {
            if formatted != discovered.contents {
                let line = first_changed_line(&discovered.contents, &formatted);
                println!(
                    "{} needs formatting (first change: line {line})",
                    discovered.path.display()
                );
                return Ok(ExitCode::from(1));
            }
            return Ok(ExitCode::SUCCESS);
        }
        write_formatted_file(&discovered.path, &formatted)?;
        return Ok(ExitCode::SUCCESS);
    }

    let document = parse_onlyfile(&discovered.contents)?;
    let discovered = LoadedOnlyfile {
        path: discovered.path,
        base_dir: discovered.base_dir,
        contents: discovered.contents,
        document,
    };

    let cli = parse_with_onlyfile(&discovered.document)?;

    ensure_dry_run_has_target(&cli)?;

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
    if cli.dry_run {
        println!("{}", render_dry_run_for_cli(&compiled, &cli)?);
        return Ok(ExitCode::SUCCESS);
    }
    run_compiled_plan(&compiled.plan, &cli)
}

fn first_changed_line(original: &str, formatted: &str) -> usize {
    original
        .lines()
        .zip(formatted.lines())
        .position(|(left, right)| left != right)
        .map_or_else(
            || usize::min(original.lines().count(), formatted.lines().count()) + 1,
            |index| index + 1,
        )
}

fn write_formatted_file(path: &Path, contents: &str) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temporary = parent.join(format!(".only-format-{}", std::process::id()));
    std::fs::write(&temporary, contents).map_err(|error| OnlyError::runtime(error.to_string()))?;
    std::fs::rename(&temporary, path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        OnlyError::runtime(error.to_string())
    })
}
