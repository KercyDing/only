use std::collections::{HashMap, HashSet};

use only_semantic::TaskAst;

use crate::planner::PlanError;
use crate::resolve::{TaskIndex, bind_parameters, select_task_variant};

pub(crate) type BoundTask<'a> = (&'a TaskAst, HashMap<String, String>);

pub(crate) fn expand_execution_order<'a>(
    root: &'a TaskAst,
    root_bindings: &HashMap<String, String>,
    tasks: &TaskIndex<'a>,
) -> Result<Vec<BoundTask<'a>>, PlanError> {
    let mut ordered = Vec::new();
    let mut visited = HashSet::new();
    let mut visiting = Vec::new();
    visit_task(
        root,
        Some(root_bindings),
        tasks,
        &mut visited,
        &mut visiting,
        &mut ordered,
    )?;
    Ok(ordered)
}

fn visit_task<'a>(
    task: &'a TaskAst,
    root_bindings: Option<&HashMap<String, String>>,
    tasks: &TaskIndex<'a>,
    visited: &mut HashSet<String>,
    visiting: &mut Vec<String>,
    ordered: &mut Vec<BoundTask<'a>>,
) -> Result<(), PlanError> {
    let qualified_name = task.qualified_name().to_string();
    if visited.contains(&qualified_name) {
        return Ok(());
    }

    if visiting.contains(&qualified_name) {
        visiting.push(qualified_name);
        return Err(PlanError::CyclicDependency(visiting.join(" -> ")));
    }

    visiting.push(task.qualified_name().to_string());

    for dependency in &task.dependencies {
        if let Some(dependency_task) = tasks
            .get(dependency.name.as_str())
            .and_then(|variants| select_task_variant(variants))
        {
            visit_task(dependency_task, None, tasks, visited, visiting, ordered)?;
        }
    }

    visiting.pop();
    visited.insert(task.qualified_name().to_string());
    let bindings = bind_parameters(task, root_bindings)?;
    ordered.push((task, bindings));
    Ok(())
}
