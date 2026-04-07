use std::collections::{HashMap, HashSet};

use crate::diagnostic::error::{OnlyError, Result};
use crate::model::{Namespace, Onlyfile, TaskDefinition};

/// Validates semantic constraints after syntax parsing succeeds.
///
/// Args:
/// document: Parsed Onlyfile document.
///
/// Returns:
/// Success when the document satisfies current semantic rules.
pub fn validate(document: &Onlyfile) -> Result<()> {
    validate_namespace_conflicts(document)?;
    validate_duplicate_signatures(document)?;
    validate_dependency_references(document)?;
    Ok(())
}

fn validate_namespace_conflicts(document: &Onlyfile) -> Result<()> {
    let global_task_names: HashSet<&str> = document
        .global_tasks
        .iter()
        .map(|task| task.signature.name.as_str())
        .collect();

    for namespace in &document.namespaces {
        if global_task_names.contains(namespace.name.as_str()) {
            return Err(OnlyError::parse(format!(
                "conflict: global task '{}' and namespace '{}' cannot coexist",
                namespace.name, namespace.name
            )));
        }
    }

    Ok(())
}

fn validate_duplicate_signatures(document: &Onlyfile) -> Result<()> {
    validate_scope_duplicates(None, &document.global_tasks)?;

    for namespace in &document.namespaces {
        validate_scope_duplicates(Some(namespace.name.as_str()), &namespace.tasks)?;
    }

    Ok(())
}

fn validate_scope_duplicates(namespace: Option<&str>, tasks: &[TaskDefinition]) -> Result<()> {
    let mut seen = HashMap::<String, &TaskDefinition>::new();

    for task in tasks {
        let key = task_signature_key(task);
        if let Some(previous) = seen.insert(key, task) {
            return Err(OnlyError::parse(format!(
                "duplicate task definition: '{}' conflicts with '{}'",
                task.display_name(namespace),
                previous.display_name(namespace)
            )));
        }
    }

    Ok(())
}

fn validate_dependency_references(document: &Onlyfile) -> Result<()> {
    let global_task_names: HashSet<&str> = document
        .global_tasks
        .iter()
        .map(|task| task.signature.name.as_str())
        .collect();

    let namespace_task_names: HashMap<&str, HashSet<&str>> = document
        .namespaces
        .iter()
        .map(|namespace| {
            (
                namespace.name.as_str(),
                namespace
                    .tasks
                    .iter()
                    .map(|task| task.signature.name.as_str())
                    .collect(),
            )
        })
        .collect();

    validate_scope_dependencies(
        None,
        &document.global_tasks,
        &global_task_names,
        &namespace_task_names,
    )?;

    for namespace in &document.namespaces {
        validate_scope_dependencies(
            Some(namespace),
            &namespace.tasks,
            &global_task_names,
            &namespace_task_names,
        )?;
    }

    Ok(())
}

fn validate_scope_dependencies(
    namespace: Option<&Namespace>,
    tasks: &[TaskDefinition],
    global_task_names: &HashSet<&str>,
    namespace_task_names: &HashMap<&str, HashSet<&str>>,
) -> Result<()> {
    for task in tasks {
        for dependency in &task.signature.dependencies {
            if dependency.contains('.') {
                let mut parts = dependency.split('.');
                let namespace_name = parts.next().expect("split always returns first part");
                let task_name = parts.next().unwrap_or_default();
                let is_valid = namespace_task_names
                    .get(namespace_name)
                    .is_some_and(|tasks| tasks.contains(task_name));

                if !is_valid {
                    return Err(undefined_dependency_error(
                        namespace.map(|item| item.name.as_str()),
                        task,
                        dependency,
                    ));
                }

                continue;
            }

            let in_same_namespace = namespace
                .and_then(|current| namespace_task_names.get(current.name.as_str()))
                .is_some_and(|tasks| tasks.contains(dependency.as_str()));

            if in_same_namespace || global_task_names.contains(dependency.as_str()) {
                continue;
            }

            return Err(undefined_dependency_error(
                namespace.map(|item| item.name.as_str()),
                task,
                dependency,
            ));
        }
    }

    Ok(())
}

fn undefined_dependency_error(
    namespace: Option<&str>,
    task: &TaskDefinition,
    dependency: &str,
) -> OnlyError {
    OnlyError::parse(format!(
        "undefined dependency '{}' referenced from '{}'",
        dependency,
        task.display_name(namespace)
    ))
}

fn task_signature_key(task: &TaskDefinition) -> String {
    let parameter_names = task
        .signature
        .parameters
        .iter()
        .map(|parameter| match &parameter.default_value {
            Some(default) => format!("{}={default}", parameter.name),
            None => parameter.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(",");

    let guard = task
        .signature
        .guard
        .as_ref()
        .map(|guard| format!("{:?}:{}", guard.probe.kind, guard.probe.argument))
        .unwrap_or_default();

    format!("{}|{}|{}", task.signature.name, parameter_names, guard)
}

#[cfg(test)]
mod tests {
    use crate::model::{Namespace, Onlyfile, SourceSpan, TaskDefinition, TaskSignature};

    use super::validate;

    #[test]
    fn rejects_global_task_namespace_name_conflict() {
        let document = Onlyfile {
            global_tasks: vec![task("build", &[])],
            namespaces: vec![Namespace {
                name: "build".into(),
                span: SourceSpan::new(0, 0),
                tasks: vec![],
            }],
            ..Onlyfile::default()
        };

        let error = validate(&document).expect_err("conflict should fail");
        assert_eq!(
            error.to_string(),
            "conflict: global task 'build' and namespace 'build' cannot coexist"
        );
    }

    #[test]
    fn rejects_undefined_dependency() {
        let document = Onlyfile {
            global_tasks: vec![task("deploy", &["build"])],
            ..Onlyfile::default()
        };

        let error = validate(&document).expect_err("undefined dependency should fail");
        assert_eq!(
            error.to_string(),
            "undefined dependency 'build' referenced from 'deploy'"
        );
    }

    fn task(name: &str, dependencies: &[&str]) -> TaskDefinition {
        TaskDefinition {
            signature: TaskSignature {
                name: name.into(),
                parameters: vec![],
                guard: None,
                dependencies: dependencies.iter().map(|value| (*value).into()).collect(),
                span: SourceSpan::new(0, 0),
            },
            doc: None,
            commands: vec![],
            span: SourceSpan::new(0, 0),
        }
    }
}
