use std::collections::HashSet;

use only_diagnostic::{Diagnostic, DiagnosticCode, DiagnosticPhase, DiagnosticSeverity};
use text_size::TextRange;

use crate::interpolation::scan_interpolations;
use crate::{
    DirectiveAst, DirectiveKind, DocumentAst, GuardKind, MetadataKind, ShellKind, ShellOperator,
    ShellSelection, SymbolIndex, TaskAst,
};

pub(crate) fn validate_document(document: &DocumentAst, symbols: &SymbolIndex) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let task_names: HashSet<_> = symbols.tasks.iter().map(|task| task.name.clone()).collect();
    let global_task_names: HashSet<_> = document
        .tasks
        .iter()
        .filter(|task| task.namespace.is_none())
        .map(|task| task.name.clone())
        .collect();
    let globals = document
        .directives
        .iter()
        .filter_map(|directive| match directive {
            DirectiveAst::Variable { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    for directive in &document.directives {
        if let DirectiveAst::Shell { shell, range } = directive {
            validate_shell(
                &ShellSelection::required(shell.clone()),
                *range,
                &mut diagnostics,
            );
        }
    }

    for namespace in &document.namespaces {
        validate_metadata(
            &namespace.metadata,
            namespace.range,
            &globals,
            false,
            &mut diagnostics,
        );
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
        validate_task(task, &task_names, &globals, &mut diagnostics);
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
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_metadata(&task.metadata, task.range, globals, true, diagnostics);

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

    if let Some(shell) = &task.shell {
        validate_shell(&shell.selection, shell.range, diagnostics);
    }

    let mut guards = HashSet::new();
    for guard in &task.guards {
        if !guard.kind.is_supported() {
            diagnostics.push(error(
                "semantic.unknown-guard",
                format!(
                    "guard '@{}' is not supported; use {}",
                    guard.kind,
                    supported_guards()
                ),
                guard.range,
            ));
        }

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

fn validate_metadata(
    metadata: &crate::TaskMetadataAst,
    range: TextRange,
    globals: &HashSet<smol_str::SmolStr>,
    allow_result_messages: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if metadata.desc.is_some() && metadata.help.is_none() {
        diagnostics.push(error(
            "semantic.metadata-help-required",
            "`[help]` is required when `[desc]` is used".to_string(),
            range,
        ));
    }
    if metadata.help_count > 1 {
        diagnostics.push(error(
            "semantic.duplicate-help",
            "`[help]` can be used only once".to_string(),
            range,
        ));
    }
    for field in &metadata.unknown_fields {
        diagnostics.push(error(
            "semantic.unknown-metadata-field",
            format!(
                "unknown metadata field `{field}`; expected {}",
                MetadataKind::expected_list()
            ),
            range,
        ));
    }
    if !allow_result_messages && (metadata.pass.is_some() || metadata.fail.is_some()) {
        diagnostics.push(error(
            "semantic.group-result-metadata",
            "`[pass]` and `[fail]` are only valid on tasks".to_string(),
            range,
        ));
    }
    for (field, text) in [
        ("help", metadata.help.as_ref()),
        ("desc", metadata.desc.as_ref()),
        ("pass", metadata.pass.as_ref()),
        ("fail", metadata.fail.as_ref()),
    ] {
        let Some(text) = text else { continue };
        for interpolation in scan_interpolations(text.as_str()) {
            if !globals.contains(&interpolation.name) {
                diagnostics.push(error(
                    "semantic.metadata-variable",
                    format!(
                        "{field} can only use a global variable named '{}'",
                        interpolation.name
                    ),
                    range,
                ));
            }
        }
    }
}

fn validate_shell(shell: &ShellSelection, range: TextRange, diagnostics: &mut Vec<Diagnostic>) {
    if !shell.kind.is_supported() {
        diagnostics.push(error(
            "semantic.unknown-shell",
            format!(
                "shell '{}' is not supported; use {}",
                shell.kind,
                supported_shells()
            ),
            range,
        ));
    } else if shell.operator == ShellOperator::Fallback && shell.kind.fallback().is_none() {
        diagnostics.push(error(
            "semantic.invalid-shell-fallback",
            format!(
                "shell '{}' has no fallback; use `shell={}`",
                shell.kind, shell.kind
            ),
            range,
        ));
    }
}

fn supported_shells() -> String {
    format_choices(
        ShellKind::SUPPORTED
            .iter()
            .map(|shell| format!("'{}'", shell.as_str()))
            .collect(),
    )
}

fn supported_guards() -> String {
    format_choices(
        GuardKind::SUPPORTED
            .iter()
            .map(|guard| format!("'@{}'", guard.as_str()))
            .collect(),
    )
}

fn format_choices(mut choices: Vec<String>) -> String {
    let Some(last) = choices.pop() else {
        return String::new();
    };
    if choices.is_empty() {
        return last;
    }
    format!("{}, or {last}", choices.join(", "))
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
    let mut seen = std::collections::HashMap::<DirectiveKind, TextRange>::new();

    for directive in &document.directives {
        let (name, range) = match directive {
            DirectiveAst::Version { .. } => continue,
            DirectiveAst::Shell { range, .. } => (DirectiveKind::Shell, *range),
            DirectiveAst::Variable { .. } => continue,
        };

        if let Some(previous_range) = seen.insert(name.clone(), range) {
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
