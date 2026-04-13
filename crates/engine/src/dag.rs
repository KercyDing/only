use std::collections::{HashMap, HashSet};

use only_semantic::{DependencyAst, TaskAst};

use crate::planner::PlanError;
use crate::resolve::{TaskIndex, bind_parameters, select_task_variant};

pub(crate) type BoundTask<'a> = (usize, &'a TaskAst, HashMap<String, String>);

#[derive(Default)]
struct ExecutionGraph<'a> {
    nodes: HashMap<String, (&'a TaskAst, HashMap<String, String>)>,
    registration_order: Vec<String>,
    edges: HashMap<String, HashSet<String>>,
}

pub(crate) fn expand_execution_order<'a>(
    root: &'a TaskAst,
    root_bindings: &HashMap<String, String>,
    tasks: &TaskIndex<'a>,
) -> Result<Vec<BoundTask<'a>>, PlanError> {
    let mut graph = ExecutionGraph::default();
    let mut visiting = Vec::new();
    collect_task(root, Some(root_bindings), tasks, &mut visiting, &mut graph)?;
    build_staged_order(graph)
}

fn collect_task<'a>(
    task: &'a TaskAst,
    root_bindings: Option<&HashMap<String, String>>,
    tasks: &TaskIndex<'a>,
    visiting: &mut Vec<String>,
    graph: &mut ExecutionGraph<'a>,
) -> Result<(), PlanError> {
    let qualified_name = task.qualified_name().to_string();
    if visiting.contains(&qualified_name) {
        visiting.push(qualified_name);
        return Err(PlanError::CyclicDependency(visiting.join(" -> ")));
    }
    if graph.nodes.contains_key(&qualified_name) {
        return Ok(());
    }

    visiting.push(qualified_name.clone());
    let bindings = bind_parameters(task, root_bindings)?;
    graph.nodes.insert(qualified_name.clone(), (task, bindings));
    graph.registration_order.push(qualified_name.clone());

    let dependency_groups = group_dependencies(task);
    let mut previous_group: Vec<String> = Vec::new();

    for group in dependency_groups {
        let mut current_group = Vec::new();
        for dependency in group {
            if let Some(dependency_task) = tasks
                .get(dependency.name.as_str())
                .and_then(|variants| select_task_variant(variants))
            {
                let dependency_name = dependency_task.qualified_name().to_string();
                collect_task(dependency_task, None, tasks, visiting, graph)?;
                graph
                    .edges
                    .entry(dependency_name.clone())
                    .or_default()
                    .insert(qualified_name.clone());
                current_group.push(dependency_name);
            }
        }

        for previous in &previous_group {
            for current in &current_group {
                graph
                    .edges
                    .entry(previous.clone())
                    .or_default()
                    .insert(current.clone());
            }
        }

        previous_group = current_group;
    }

    visiting.pop();
    Ok(())
}

fn group_dependencies(task: &TaskAst) -> Vec<Vec<&DependencyAst>> {
    let mut groups = Vec::new();
    let mut current_stage = None;

    for dependency in &task.dependencies {
        if current_stage != Some(dependency.stage) {
            groups.push(Vec::new());
            current_stage = Some(dependency.stage);
        }
        groups
            .last_mut()
            .expect("dependency group should exist")
            .push(dependency);
    }

    groups
}

fn build_staged_order<'a>(graph: ExecutionGraph<'a>) -> Result<Vec<BoundTask<'a>>, PlanError> {
    let mut indegree = graph
        .registration_order
        .iter()
        .cloned()
        .map(|name| (name, 0usize))
        .collect::<HashMap<_, _>>();

    for dependents in graph.edges.values() {
        for dependent in dependents {
            *indegree
                .get_mut(dependent)
                .expect("dependent node should exist in graph") += 1;
        }
    }

    let mut scheduled = HashSet::new();
    let mut ordered = Vec::new();
    let mut stage = 0usize;

    while scheduled.len() < graph.registration_order.len() {
        let ready = graph
            .registration_order
            .iter()
            .filter(|name| !scheduled.contains(*name) && indegree.get(*name) == Some(&0))
            .cloned()
            .collect::<Vec<_>>();

        if ready.is_empty() {
            return Err(PlanError::CyclicDependency("execution graph".to_string()));
        }

        for name in &ready {
            scheduled.insert(name.clone());
        }

        for name in &ready {
            if let Some(dependents) = graph.edges.get(name) {
                for dependent in dependents {
                    *indegree
                        .get_mut(dependent)
                        .expect("dependent node should exist in graph") -= 1;
                }
            }
        }

        for name in ready {
            let (task, bindings) = graph
                .nodes
                .get(&name)
                .expect("registered node should exist in graph");
            ordered.push((stage, *task, bindings.clone()));
        }

        stage += 1;
    }

    Ok(ordered)
}
