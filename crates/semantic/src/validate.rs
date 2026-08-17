use std::collections::HashSet;

use only_diagnostic::{Diagnostic, DiagnosticCode, DiagnosticPhase, DiagnosticSeverity};
use text_size::TextRange;

use crate::{DirectiveAst, DocumentAst, SymbolIndex, TaskAst};

pub(crate) fn validate_document(document: &DocumentAst, symbols: &SymbolIndex) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let task_names: HashSet<_> = symbols.tasks.iter().map(|task| task.name.clone()).collect();
    let global_task_names: HashSet<_> = document
        .tasks
        .iter()
        .filter(|task| task.namespace.is_none())
        .map(|task| task.name.clone())
        .collect();
    let supports_command_blocks =
        document
            .directives
            .iter()
            .find_map(|directive| match directive {
                DirectiveAst::Version { major, minor, .. } => Some(*major > 0 || *minor >= 2),
                DirectiveAst::Shell { .. } => None,
                DirectiveAst::Variable { .. } => None,
            })
            == Some(true);
    let supports_engineering = document.directives.iter().any(|directive| {
        matches!(directive, DirectiveAst::Version { major, minor, .. } if *major > 0 || *minor >= 3)
    });
    let globals = document
        .directives
        .iter()
        .filter_map(|directive| match directive {
            DirectiveAst::Variable { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    if !supports_engineering && (!globals.is_empty() || document.uses_namespace_close) {
        let range = document
            .directives
            .iter()
            .find_map(|directive| match directive {
                DirectiveAst::Variable { range, .. } => Some(*range),
                _ => None,
            })
            .or_else(|| document.namespaces.first().map(|namespace| namespace.range))
            .unwrap_or_default();
        diagnostics.push(error(
            "semantic.engineering-version",
            "this syntax needs `!version 0.3` or newer".to_string(),
            range,
        ));
    }

    for namespace in &document.namespaces {
        if global_task_names.contains(&namespace.name) {
            diagnostics.push(error(
                "semantic.namespace-conflict",
                format!(
                    "task and namespace cannot both be named '{}'",
                    namespace.name
                ),
                namespace.range,
            ));
        }
    }

    for task in &document.tasks {
        validate_task(
            task,
            &task_names,
            &globals,
            supports_command_blocks,
            supports_engineering,
            &mut diagnostics,
        );
    }

    report_duplicate_directives(document, &mut diagnostics);
    report_duplicate_variables(document, &mut diagnostics);
    report_duplicate_tasks(document, &mut diagnostics);
    diagnostics
}

fn validate_task(
    task: &TaskAst,
    task_names: &HashSet<smol_str::SmolStr>,
    globals: &HashSet<smol_str::SmolStr>,
    supports_command_blocks: bool,
    supports_engineering: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut params = HashSet::new();
    for (index, param) in task.params.iter().enumerate() {
        if !params.insert(param.name.clone()) {
            diagnostics.push(error(
                "semantic.duplicate-parameter",
                format!(
                    "parameter '{}' is used more than once in task '{}'",
                    param.name,
                    task.qualified_name()
                ),
                task.range,
            ));
        }

        if param.is_slice && index + 1 != task.params.len() {
            diagnostics.push(error(
                "semantic.slice-parameter-position",
                format!(
                    "slice parameter '{}..' must be last in task '{}'",
                    param.name,
                    task.qualified_name()
                ),
                task.range,
            ));
        }

        if param.is_slice && param.default_value.is_some() {
            diagnostics.push(error(
                "semantic.slice-parameter-default",
                format!("slice parameter '{}..' cannot have a default", param.name),
                task.range,
            ));
        }
    }

    if !supports_engineering
        && (task.guards.len() > 1 || task.uses_multiline_header || task.shell_fallback)
    {
        diagnostics.push(error(
            "semantic.engineering-version",
            "this syntax needs `!version 0.3` or newer".to_string(),
            task.range,
        ));
    }

    let mut guards = HashSet::new();
    for guard in &task.guards {
        let key = format!("{}:{}", guard.kind, guard.argument);
        if !guards.insert(key) {
            diagnostics.push(error(
                "semantic.duplicate-guard",
                format!(
                    "guard '@{}(\"{}\")' is repeated",
                    guard.kind, guard.argument
                ),
                guard.range,
            ));
        }
    }

    for dependency in &task.dependencies {
        if !task_names.contains(&dependency.name) {
            diagnostics.push(error(
                "semantic.undefined-dependency",
                format!(
                    "task '{}' depends on missing task '{}'",
                    task.qualified_name(),
                    dependency.name
                ),
                dependency.range,
            ));
        }
    }

    for step in &task.steps {
        if let crate::TaskStepAst::CommandBlock(block) = step
            && !supports_command_blocks
        {
            diagnostics.push(error(
                "semantic.command-block-version",
                "command blocks need `!version 0.2` or newer".to_string(),
                block.range,
            ));
        }
        let interpolations = match step {
            crate::TaskStepAst::Command(command) => &command.interpolations,
            crate::TaskStepAst::CommandBlock(block) => &block.interpolations,
        };
        for interpolation in interpolations {
            if !params.contains(&interpolation.name) && !globals.contains(&interpolation.name) {
                diagnostics.push(error(
                    "semantic.undefined-variable",
                    format!("variable '{}' is not defined", interpolation.name),
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
        if !task.guards.is_empty() {
            let guard_key = (task.qualified_name().to_string(), render_guards(task));
            if seen_guards.insert(guard_key, task).is_some() {
                diagnostics.push(error(
                    "semantic.ambiguous-guard",
                    format!(
                        "task '{}' has the same guard more than once",
                        task.qualified_name()
                    ),
                    task.range,
                ));
            }
        }

        let key = task_signature_key(task);
        if seen.insert(key, task).is_some() {
            diagnostics.push(error(
                "semantic.duplicate-task",
                format!("task '{}' is defined more than once", task.qualified_name()),
                task.range,
            ));
        }
    }
}

fn report_duplicate_directives(document: &DocumentAst, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen = std::collections::HashMap::<&'static str, TextRange>::new();

    for directive in &document.directives {
        let (name, range) = match directive {
            DirectiveAst::Version { .. } => continue,
            DirectiveAst::Shell { range, .. } => ("shell", *range),
            DirectiveAst::Variable { .. } => continue,
        };

        if let Some(previous_range) = seen.insert(name, range) {
            diagnostics.push(error(
                "semantic.duplicate-directive",
                format!("`!{name}` is used more than once"),
                previous_range.cover(range),
            ));
        }
    }
}

fn report_duplicate_variables(document: &DocumentAst, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen = std::collections::HashMap::<&str, TextRange>::new();
    for directive in &document.directives {
        let DirectiveAst::Variable { name, range, .. } = directive else {
            continue;
        };
        if let Some(previous) = seen.insert(name.as_str(), *range) {
            diagnostics.push(error(
                "variable.duplicate",
                format!("variable '{name}' is used more than once"),
                previous.cover(*range),
            ));
        }
    }
}

fn task_signature_key(task: &TaskAst) -> String {
    let parameter_names = task
        .params
        .iter()
        .map(
            |parameter| match (parameter.is_slice, &parameter.default_value) {
                (true, Some(default)) => format!("{}..={default}", parameter.name),
                (true, None) => format!("{}..", parameter.name),
                (false, Some(default)) => format!("{}={default}", parameter.name),
                (false, None) => parameter.name.to_string(),
            },
        )
        .collect::<Vec<_>>()
        .join(",");

    let guard = render_guards(task);

    format!("{}|{}|{}", task.qualified_name(), parameter_names, guard)
}

fn render_guards(task: &TaskAst) -> String {
    task.guards
        .iter()
        .map(|guard| format!("{}:{}", guard.kind, guard.argument))
        .collect::<Vec<_>>()
        .join("?")
}
