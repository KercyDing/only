use std::collections::{HashMap, HashSet};

use only_semantic::{DirectiveAst, DocumentAst, TaskAst};

use crate::planner::PlanError;
use crate::probe::probe_matches;
use crate::{ExecutionNode, PlanParam};

pub(crate) type TaskIndex<'a> = HashMap<String, Vec<&'a TaskAst>>;

pub(crate) fn build_task_index(document: &DocumentAst) -> TaskIndex<'_> {
    let mut tasks = HashMap::<String, Vec<&TaskAst>>::new();
    for task in &document.tasks {
        tasks
            .entry(task.qualified_name().to_string())
            .or_default()
            .push(task);
    }
    tasks
}

pub(crate) fn resolve_root_task<'a>(
    tasks: &'a TaskIndex<'a>,
    target: &str,
) -> Result<&'a TaskAst, PlanError> {
    let Some(root_variants) = tasks.get(target) else {
        return Err(PlanError::UnknownTask(target.to_string()));
    };
    select_root_task_from_variants(root_variants, target)
}

pub(crate) fn resolve_root_task_in_document<'a>(
    document: &'a DocumentAst,
    target: &str,
) -> Result<&'a TaskAst, PlanError> {
    let root_variants = document
        .tasks
        .iter()
        .filter(|task| task.qualified_name() == target)
        .collect::<Vec<_>>();

    if root_variants.is_empty() {
        return Err(PlanError::UnknownTask(target.to_string()));
    }

    select_root_task_from_variants(&root_variants, target)
}

fn select_root_task_from_variants<'a>(
    root_variants: &[&'a TaskAst],
    target: &str,
) -> Result<&'a TaskAst, PlanError> {
    let Some(root) = select_task_variant(root_variants) else {
        return Err(PlanError::TaskUnavailable(target.to_string()));
    };
    if root.is_helper() {
        return Err(PlanError::HelperTask(target.to_string()));
    }
    Ok(root)
}

pub(crate) fn build_execution_nodes(
    ordered: Vec<(usize, &TaskAst, HashMap<String, String>)>,
) -> Vec<ExecutionNode> {
    ordered
        .into_iter()
        .map(|(stage, task, bindings)| ExecutionNode {
            stage,
            name: task.qualified_name().to_string(),
            commands: task
                .commands
                .iter()
                .map(|command| command.text.to_string())
                .collect(),
            params: task
                .params
                .iter()
                .map(|param| PlanParam {
                    name: param.name.to_string(),
                    default_value: param.default_value.as_ref().map(ToString::to_string),
                    value: bindings.get(param.name.as_str()).cloned(),
                })
                .collect(),
            shell: task.shell.as_ref().map(ToString::to_string),
            shell_fallback: task.shell_fallback,
        })
        .collect()
}

pub(crate) fn select_task_variant<'a>(variants: &[&'a TaskAst]) -> Option<&'a TaskAst> {
    let mut fallback = None;

    for task in variants {
        match &task.guard {
            Some(guard) => {
                if probe_matches(guard.kind.as_str(), guard.argument.as_str()) {
                    return Some(task);
                }
            }
            None => fallback = Some(*task),
        }
    }

    fallback
}

pub(crate) fn document_echo(document: &DocumentAst) -> bool {
    document
        .directives
        .iter()
        .fold(true, |echo, directive| match directive {
            DirectiveAst::Echo { value, .. } => *value,
            DirectiveAst::Preview { .. } | DirectiveAst::Shell { .. } => echo,
        })
}

pub(crate) fn document_preview(document: &DocumentAst) -> bool {
    document
        .directives
        .iter()
        .fold(false, |preview, directive| match directive {
            DirectiveAst::Preview { value, .. } => *value,
            DirectiveAst::Echo { .. } | DirectiveAst::Shell { .. } => preview,
        })
}

pub(crate) fn document_shell(document: &DocumentAst) -> Option<String> {
    document
        .directives
        .iter()
        .fold(None, |shell, directive| match directive {
            DirectiveAst::Shell {
                shell: directive_shell,
                ..
            } => Some(directive_shell.to_string()),
            DirectiveAst::Echo { .. } | DirectiveAst::Preview { .. } => shell,
        })
}

pub(crate) fn bind_parameters(
    task: &TaskAst,
    inputs: Option<&HashMap<String, String>>,
) -> Result<HashMap<String, String>, PlanError> {
    let input_map = inputs.cloned().unwrap_or_default();
    let mut parameters = HashMap::new();

    for param in &task.params {
        if let Some(value) = input_map.get(param.name.as_str()) {
            parameters.insert(param.name.to_string(), value.clone());
            continue;
        }

        if let Some(default) = &param.default_value {
            parameters.insert(param.name.to_string(), default.to_string());
            continue;
        }

        return Err(PlanError::MissingRequiredParameter(param.name.to_string()));
    }

    Ok(parameters)
}

pub(crate) fn merge_parameter_inputs(
    positional_args: Vec<&str>,
    named_overrides: Vec<(&str, &str)>,
    task: &TaskAst,
) -> Result<HashMap<String, String>, PlanError> {
    if positional_args.len() > task.params.len() {
        return Err(PlanError::TooManyArguments {
            task: task.qualified_name().to_string(),
            expected: task.params.len(),
            got: positional_args.len(),
        });
    }

    let allowed = task
        .params
        .iter()
        .map(|param| param.name.as_str())
        .collect::<HashSet<_>>();

    let mut merged = HashMap::new();
    for (index, value) in positional_args.into_iter().enumerate() {
        let parameter = &task.params[index];
        merged.insert(parameter.name.to_string(), value.to_string());
    }

    for (name, value) in named_overrides {
        if !allowed.contains(name) {
            return Err(PlanError::UnknownParameter {
                task: task.qualified_name().to_string(),
                name: name.to_string(),
            });
        }

        if merged.insert(name.to_string(), value.to_string()).is_some() {
            return Err(PlanError::DuplicateOverride(name.to_string()));
        }
    }

    Ok(merged)
}
