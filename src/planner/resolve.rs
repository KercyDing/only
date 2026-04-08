use std::collections::HashSet;
use std::path::PathBuf;

use crate::cli::args::CliInput;
use crate::diagnostic::error::{OnlyError, Result};
use crate::model::{Directive, Namespace, Onlyfile, ProbeKind, ShellKind, TaskDefinition};

use super::dag::{ExecutionNode, ExecutionPlan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvocationTarget {
    GlobalTask {
        task: String,
        args: Vec<String>,
    },
    NamespacedTask {
        namespace: String,
        task: String,
        args: Vec<String>,
    },
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
    let working_dir = std::env::current_dir().map_err(OnlyError::cwd)?;
    build_execution_plan_in_dir(document, cli, working_dir)
}

/// Builds a resolved execution plan with an explicit working directory.
///
/// Args:
/// document: Parsed Onlyfile document.
/// cli: Normalized CLI input.
/// working_dir: Directory used for task execution.
///
/// Returns:
/// Ordered execution plan after dependency expansion.
pub fn build_execution_plan_in_dir(
    document: &Onlyfile,
    cli: &CliInput,
    working_dir: PathBuf,
) -> Result<ExecutionPlan> {
    let target = resolve_target(document, cli)?;
    let mut nodes = Vec::new();
    let mut visiting = Vec::new();
    let mut visited = HashSet::new();
    match target {
        InvocationTarget::GlobalTask { task, args } => {
            let resolved = select_global_task(document, &task)?.ok_or_else(|| {
                OnlyError::parse(format!(
                    "task '{task}' is not available for this environment"
                ))
            })?;
            let overrides = merge_parameter_inputs(args, &cli.parameter_overrides, resolved)?;
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
        InvocationTarget::NamespacedTask {
            namespace,
            task,
            args,
        } => {
            let namespace_ref = find_namespace(document, &namespace)?;
            let resolved = select_task_in_namespace(namespace_ref, &task)?.ok_or_else(|| {
                OnlyError::parse(format!(
                    "task '{}.{}' is not available for this environment",
                    namespace, task
                ))
            })?;
            let overrides = merge_parameter_inputs(args, &cli.parameter_overrides, resolved)?;
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
        working_dir,
        shell: configured_shell(document),
    })
}

fn resolve_target(document: &Onlyfile, cli: &CliInput) -> Result<InvocationTarget> {
    match cli.task_path.as_slice() {
        [] => Err(OnlyError::parse(
            "no task selected; provide a global task or namespace task target",
        )),
        [name] => {
            if find_namespace(document, name).is_ok() {
                return Err(OnlyError::parse(format!(
                    "namespace '{name}' requires a task target"
                )));
            }

            Ok(InvocationTarget::GlobalTask {
                task: name.clone(),
                args: Vec::new(),
            })
        }
        [first, second, rest @ ..] => {
            if find_namespace(document, first).is_ok() {
                return Ok(InvocationTarget::NamespacedTask {
                    namespace: first.clone(),
                    task: second.clone(),
                    args: rest.to_vec(),
                });
            }

            Ok(InvocationTarget::GlobalTask {
                task: first.clone(),
                args: std::iter::once(second.clone())
                    .chain(rest.iter().cloned())
                    .collect(),
            })
        }
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
        shell: task.signature.shell,
        shell_fallback: task.signature.shell_fallback,
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

    if let Some(current_namespace) = namespace
        && let Some(task) = select_task_in_namespace(current_namespace, dependency)?
    {
        return Ok(Some((Some(current_namespace), task)));
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
        ProbeKind::Has => command_exists(argument),
    }
}

fn command_exists(command: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| command_exists_in_dir(&directory, command))
    })
}

fn command_exists_in_dir(directory: &std::path::Path, command: &str) -> bool {
    let candidate = directory.join(command);
    if candidate.is_file() {
        return true;
    }

    #[cfg(windows)]
    {
        let has_extension = std::path::Path::new(command).extension().is_some();
        if has_extension {
            return false;
        }

        let extensions = std::env::var_os("PATHEXT")
            .and_then(|value| value.into_string().ok())
            .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string());

        extensions
            .split(';')
            .map(str::trim)
            .filter(|extension| !extension.is_empty())
            .any(|extension| directory.join(format!("{command}{extension}")).is_file())
    }

    #[cfg(not(windows))]
    {
        false
    }
}

fn bind_parameters(
    task: &TaskDefinition,
    overrides: &[(String, String)],
) -> Result<std::collections::HashMap<String, String>> {
    let mut override_map = std::collections::HashMap::new();
    let allowed = task
        .signature
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect::<std::collections::HashSet<_>>();

    for (name, value) in overrides {
        if !allowed.contains(name.as_str()) {
            return Err(OnlyError::parse(format!(
                "unknown parameter '{name}' for task '{}'",
                task.signature.name
            )));
        }

        if override_map.insert(name.clone(), value.clone()).is_some() {
            return Err(OnlyError::parse(format!(
                "duplicate parameter override '{name}'"
            )));
        }
    }

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

fn merge_parameter_inputs(
    positional_args: Vec<String>,
    named_overrides: &[(String, String)],
    task: &TaskDefinition,
) -> Result<Vec<(String, String)>> {
    if positional_args.len() > task.signature.parameters.len() {
        return Err(OnlyError::parse(format!(
            "too many arguments for task '{}'; expected at most {}, got {}",
            task.signature.name,
            task.signature.parameters.len(),
            positional_args.len()
        )));
    }

    let mut merged = Vec::new();
    for (index, value) in positional_args.into_iter().enumerate() {
        let parameter = &task.signature.parameters[index];
        merged.push((parameter.name.clone(), value));
    }

    merged.extend(named_overrides.iter().cloned());
    Ok(merged)
}

fn is_verbose_enabled(document: &Onlyfile) -> bool {
    document
        .directives
        .iter()
        .fold(false, |current, directive| match directive {
            Directive::Verbose { value, .. } => *value,
            Directive::Shell { .. } => current,
        })
}

fn configured_shell(document: &Onlyfile) -> ShellKind {
    document
        .directives
        .iter()
        .fold(ShellKind::Deno, |current, directive| match directive {
            Directive::Verbose { .. } => current,
            Directive::Shell { shell, .. } => *shell,
        })
}
