use crate::args::CliInput;
use crate::error::{OnlyError, Result};
use only_diagnostic::{Diagnostic, DiagnosticSeverity};
use only_engine::{
    ExecutionPlan, Invocation, build_execution_plan, try_build_execution_plan_in_dir,
};
use only_semantic::{SemanticSnapshot, compile_document};
use std::path::PathBuf;

#[derive(Debug)]
pub struct CliCompileResult {
    pub compiled: SemanticSnapshot,
    pub diagnostics: Vec<only_diagnostic::Diagnostic>,
    pub plan: ExecutionPlan,
}

/// Compiles in-memory source text for CLI execution.
///
/// Args:
/// source: Raw Onlyfile source text.
///
/// Returns:
/// Semantic snapshot, diagnostics and execution plan for the first task.
pub fn compile_for_cli(source: &str) -> CliCompileResult {
    let compiled = compile_document(source);
    let task_name = compiled
        .document
        .tasks
        .iter()
        .find(|task| !task.is_helper())
        .map(|task| task.qualified_name().to_string())
        .unwrap_or_default();
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: &task_name,
            args: vec![],
            overrides: vec![],
        },
    );
    let diagnostics = compiled.diagnostics.clone();

    CliCompileResult {
        compiled,
        diagnostics,
        plan,
    }
}

/// Compiles in-memory source text for a concrete CLI target using the new semantic/engine path.
///
/// Args:
/// source: Raw Onlyfile source text.
/// cli: Normalized CLI input.
///
/// Returns:
/// Semantic snapshot, diagnostics and execution plan for the requested target.
pub fn compile_for_cli_input(source: &str, cli: &CliInput) -> Result<CliCompileResult> {
    let working_dir = std::env::current_dir().map_err(OnlyError::cwd)?;
    compile_for_cli_input_in_dir(source, cli, working_dir)
}

/// Compiles in-memory source text for a concrete CLI target using an explicit working directory.
///
/// Args:
/// source: Raw Onlyfile source text.
/// cli: Normalized CLI input.
/// working_dir: Directory used by runtime execution.
///
/// Returns:
/// Semantic snapshot, diagnostics and execution plan for the requested target.
pub fn compile_for_cli_input_in_dir(
    source: &str,
    cli: &CliInput,
    working_dir: PathBuf,
) -> Result<CliCompileResult> {
    let compiled = compile_document(source);
    ensure_no_error_diagnostics(&compiled.diagnostics)?;
    let (target, args) = resolve_target(&compiled, cli)?;
    let args = args.iter().map(String::as_str).collect();
    let overrides = cli
        .parameter_overrides
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let plan = try_build_execution_plan_in_dir(
        &compiled.document,
        Invocation::Task {
            target: &target,
            args,
            overrides,
        },
        working_dir,
    )
    .map_err(map_plan_error)?;
    let diagnostics = compiled.diagnostics.clone();

    Ok(CliCompileResult {
        compiled,
        diagnostics,
        plan,
    })
}

pub(crate) fn resolve_target(
    compiled: &SemanticSnapshot,
    cli: &CliInput,
) -> Result<(String, Vec<String>)> {
    let namespaces = compiled
        .document
        .namespaces
        .iter()
        .map(|namespace| namespace.name.as_str())
        .collect::<std::collections::HashSet<_>>();

    match cli.task_path.as_slice() {
        [] => Err(OnlyError::parse(
            "no task selected; provide a global task or namespace task target",
        )),
        [name] => {
            if namespaces.contains(name.as_str()) {
                return Err(OnlyError::parse(format!(
                    "namespace '{name}' requires a task target"
                )));
            }

            Ok((name.clone(), Vec::new()))
        }
        [first, second, rest @ ..] => {
            if namespaces.contains(first.as_str()) {
                return Ok((format!("{first}.{second}"), rest.to_vec()));
            }

            let args = std::iter::once(second.clone())
                .chain(rest.iter().cloned())
                .collect();
            Ok((first.clone(), args))
        }
    }
}

fn map_plan_error(error: only_engine::PlanError) -> OnlyError {
    OnlyError::parse(error.to_string())
}

pub(crate) fn ensure_no_error_diagnostics(diagnostics: &[Diagnostic]) -> Result<()> {
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();

    if errors.is_empty() {
        return Ok(());
    }

    Err(OnlyError::parse(errors.join("\n")))
}
