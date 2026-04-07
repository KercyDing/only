use std::collections::HashSet;

use crate::cli::args::CliInput;
use crate::diagnostic::error::{OnlyError, Result};
use crate::model::{Directive, Namespace, Onlyfile, ProbeKind, TaskDefinition};

use super::dag::{ExecutionNode, ExecutionPlan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvocationTarget {
    GlobalTask(String),
    NamespacedTask { namespace: String, task: String },
}

/// Builds an execution plan for the requested CLI target.
///
/// Args:
/// document: Parsed Onlyfile document.
/// cli: Normalized CLI input.
///
/// Returns:
/// Ordered execution plan after dependency expansion.
pub fn build_execution_plan(document: &Onlyfile, cli: &CliInput) -> Result<ExecutionPlan> {
    let target = resolve_target(document, cli)?;
    let mut nodes = Vec::new();
    let mut visiting = Vec::new();
    let mut visited = HashSet::new();
    let overrides = cli.parameter_overrides.clone();

    match target {
        InvocationTarget::GlobalTask(task) => {
            let resolved = select_global_task(document, &task)?.ok_or_else(|| {
                OnlyError::parse(format!(
                    "task '{task}' is not available for this environment"
                ))
            })?;
            visit_task(
                document,
                None,
                resolved,
                &overrides,
                &mut visiting,
                &mut visited,
                &mut nodes,
            )?;
        }
        InvocationTarget::NamespacedTask { namespace, task } => {
            let namespace_ref = find_namespace(document, &namespace)?;
            let resolved = select_task_in_namespace(namespace_ref, &task)?.ok_or_else(|| {
                OnlyError::parse(format!(
                    "task '{}.{}' is not available for this environment",
                    namespace, task
                ))
            })?;
            visit_task(
                document,
                Some(namespace_ref),
                resolved,
                &overrides,
                &mut visiting,
                &mut visited,
                &mut nodes,
            )?;
        }
    }

    Ok(ExecutionPlan {
        nodes,
        verbose: is_verbose_enabled(document),
    })
}

fn resolve_target(document: &Onlyfile, cli: &CliInput) -> Result<InvocationTarget> {
    match (&cli.task, &cli.subtask) {
        (Some(namespace_or_task), Some(task)) => Ok(InvocationTarget::NamespacedTask {
            namespace: namespace_or_task.clone(),
            task: task.clone(),
        }),
        (Some(name), None) => {
            if find_namespace(document, name).is_ok() {
                return Ok(InvocationTarget::NamespacedTask {
                    namespace: name.clone(),
                    task: "default".into(),
                });
            }

            Ok(InvocationTarget::GlobalTask(name.clone()))
        }
        (None, None) => Err(OnlyError::parse(
            "no task selected; provide a global task or namespace task target",
        )),
        (None, Some(_)) => Err(OnlyError::parse(
            "invalid CLI target; subtask cannot be provided without a task or namespace",
        )),
    }
}

fn visit_task(
    document: &Onlyfile,
    namespace: Option<&Namespace>,
    task: &TaskDefinition,
    overrides: &[(String, String)],
    visiting: &mut Vec<String>,
    visited: &mut HashSet<String>,
    nodes: &mut Vec<ExecutionNode>,
) -> Result<()> {
    let qualified_name = task.display_name(namespace.map(|item| item.name.as_str()));
    if visited.contains(&qualified_name) {
        return Ok(());
    }

    if visiting.contains(&qualified_name) {
        visiting.push(qualified_name.clone());
        return Err(OnlyError::parse(format!(
            "cyclic dependency detected: {}",
            visiting.join(" -> ")
        )));
    }

    visiting.push(qualified_name.clone());

    for dependency in &task.signature.dependencies {
        match resolve_dependency(document, namespace, dependency)? {
            Some((dependency_namespace, dependency_task)) => {
                visit_task(
                    document,
                    dependency_namespace,
                    dependency_task,
                    &[],
                    visiting,
                    visited,
                    nodes,
                )?;
            }
            None => continue,
        }
    }

    visiting.pop();
    visited.insert(qualified_name.clone());
    nodes.push(ExecutionNode {
        qualified_name,
        commands: task
            .commands
            .iter()
            .map(|command| command.text.clone())
            .collect(),
        parameters: bind_parameters(task, overrides)?
            .into_iter()
            .collect::<Vec<_>>(),
    });
    Ok(())
}

fn resolve_dependency<'a>(
    document: &'a Onlyfile,
    namespace: Option<&'a Namespace>,
    dependency: &str,
) -> Result<Option<(Option<&'a Namespace>, &'a TaskDefinition)>> {
    if let Some((namespace_name, task_name)) = dependency.split_once('.') {
        let dependency_namespace = find_namespace(document, namespace_name)?;
        let dependency_task = select_task_in_namespace(dependency_namespace, task_name)?;
        return Ok(dependency_task.map(|task| (Some(dependency_namespace), task)));
    }

    if let Some(current_namespace) = namespace {
        if let Some(task) = select_task_in_namespace(current_namespace, dependency)? {
            return Ok(Some((Some(current_namespace), task)));
        }
    }

    let global_task = select_global_task(document, dependency)?;
    Ok(global_task.map(|task| (None, task)))
}

fn select_global_task<'a>(
    document: &'a Onlyfile,
    name: &str,
) -> Result<Option<&'a TaskDefinition>> {
    select_task(
        document
            .global_tasks
            .iter()
            .filter(|task| task.signature.name == name),
    )
}

fn select_task_in_namespace<'a>(
    namespace: &'a Namespace,
    name: &str,
) -> Result<Option<&'a TaskDefinition>> {
    select_task(
        namespace
            .tasks
            .iter()
            .filter(|task| task.signature.name == name),
    )
}

fn select_task<'a>(
    tasks: impl Iterator<Item = &'a TaskDefinition>,
) -> Result<Option<&'a TaskDefinition>> {
    let mut fallback = None;

    for task in tasks {
        match &task.signature.guard {
            Some(guard) => {
                if evaluate_probe(&guard.probe.kind, &guard.probe.argument) {
                    return Ok(Some(task));
                }
            }
            None => fallback = Some(task),
        }
    }

    Ok(fallback)
}

fn find_namespace<'a>(document: &'a Onlyfile, name: &str) -> Result<&'a Namespace> {
    document
        .namespaces
        .iter()
        .find(|namespace| namespace.name == name)
        .ok_or_else(|| OnlyError::parse(format!("namespace '{name}' is not defined")))
}

fn evaluate_probe(kind: &ProbeKind, argument: &str) -> bool {
    if argument.is_empty() {
        return false;
    }

    match kind {
        ProbeKind::Os => std::env::consts::OS == argument,
        ProbeKind::Arch => std::env::consts::ARCH == argument,
        ProbeKind::Env => std::env::var_os(argument).is_some(),
        ProbeKind::Cmd => command_exists(argument),
    }
}

fn command_exists(command: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| directory.join(command).is_file())
    })
}

fn bind_parameters(
    task: &TaskDefinition,
    overrides: &[(String, String)],
) -> Result<std::collections::HashMap<String, String>> {
    let override_map = overrides
        .iter()
        .cloned()
        .collect::<std::collections::HashMap<_, _>>();
    let mut parameters = std::collections::HashMap::new();

    for parameter in &task.signature.parameters {
        if let Some(value) = override_map.get(&parameter.name) {
            parameters.insert(parameter.name.clone(), value.clone());
            continue;
        }

        if let Some(default) = &parameter.default_value {
            parameters.insert(parameter.name.clone(), default.clone());
            continue;
        }

        return Err(OnlyError::parse(format!(
            "missing required parameter '{{{{{}}}}}'",
            parameter.name
        )));
    }

    Ok(parameters)
}

fn is_verbose_enabled(document: &Onlyfile) -> bool {
    document
        .directives
        .iter()
        .fold(false, |_, directive| match directive {
            Directive::Verbose { value, .. } => *value,
        })
}
