use std::collections::HashSet;

use only_diagnostic::{Diagnostic, DiagnosticCode, DiagnosticPhase, DiagnosticSeverity};
use text_size::TextRange;

use crate::{DocumentAst, SymbolIndex, TaskAst};

pub(crate) fn validate_document(document: &DocumentAst, symbols: &SymbolIndex) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let task_names: HashSet<_> = symbols.tasks.iter().map(|task| task.name.clone()).collect();
    let global_task_names: HashSet<_> = document
        .tasks
        .iter()
        .filter(|task| task.namespace.is_none())
        .map(|task| task.name.clone())
        .collect();

    for namespace in &document.namespaces {
        if global_task_names.contains(&namespace.name) {
            diagnostics.push(error(
                "semantic.namespace-conflict",
                format!(
                    "conflict: global task '{}' and namespace '{}' cannot coexist",
                    namespace.name, namespace.name
                ),
                namespace.range,
            ));
        }
    }

    for task in &document.tasks {
        validate_task(task, &task_names, &mut diagnostics);
    }

    report_duplicate_tasks(document, &mut diagnostics);
    diagnostics
}

fn validate_task(
    task: &TaskAst,
    task_names: &HashSet<smol_str::SmolStr>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut params = HashSet::new();
    for param in &task.params {
        if !params.insert(param.name.clone()) {
            diagnostics.push(error(
                "semantic.duplicate-parameter",
                format!(
                    "duplicate parameter '{}' in task '{}'",
                    param.name,
                    task.qualified_name()
                ),
                task.range,
            ));
        }
    }

    for dependency in &task.dependencies {
        if !task_names.contains(&dependency.name) {
            diagnostics.push(error(
                "semantic.undefined-dependency",
                format!(
                    "undefined dependency '{}' referenced from '{}'",
                    dependency.name,
                    task.qualified_name()
                ),
                dependency.range,
            ));
        }
    }

    for command in &task.commands {
        for interpolation in &command.interpolations {
            if !params.contains(&interpolation.name) {
                diagnostics.push(error(
                    "semantic.undefined-variable",
                    format!("undefined variable '{}'", interpolation.name),
                    interpolation.range,
                ));
            }
        }
    }
}

fn error(code: &str, message: String, range: TextRange) -> Diagnostic {
    Diagnostic::new(
        DiagnosticSeverity::Error,
        DiagnosticCode::new(code),
        message,
        DiagnosticPhase::Semantic,
        range,
    )
}

fn report_duplicate_tasks(document: &DocumentAst, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen = std::collections::HashMap::<String, &TaskAst>::new();
    let mut seen_guards = std::collections::HashMap::<(String, String), &TaskAst>::new();

    for task in &document.tasks {
        if let Some(guard) = &task.guard {
            let guard_key = (
                task.qualified_name().to_string(),
                format!("{}:{}", guard.kind, guard.argument),
            );
            if let Some(previous) = seen_guards.insert(guard_key, task) {
                diagnostics.push(error(
                    "semantic.ambiguous-guard",
                    format!(
                        "ambiguous guard: '{}' conflicts with '{}'",
                        task.qualified_name(),
                        previous.qualified_name()
                    ),
                    task.range,
                ));
            }
        }

        let key = task_signature_key(task);
        if let Some(previous) = seen.insert(key, task) {
            diagnostics.push(error(
                "semantic.duplicate-task",
                format!(
                    "duplicate task definition: '{}' conflicts with '{}'",
                    task.qualified_name(),
                    previous.qualified_name()
                ),
                task.range,
            ));
        }
    }
}

fn task_signature_key(task: &TaskAst) -> String {
    let parameter_names = task
        .params
        .iter()
        .map(|parameter| match &parameter.default_value {
            Some(default) => format!("{}={default}", parameter.name),
            None => parameter.name.to_string(),
        })
        .collect::<Vec<_>>()
        .join(",");

    let guard = task
        .guard
        .as_ref()
        .map(|guard| format!("{}:{}", guard.kind, guard.argument))
        .unwrap_or_default();

    format!("{}|{}|{}", task.qualified_name(), parameter_names, guard)
}
