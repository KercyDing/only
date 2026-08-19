use std::collections::{HashMap, HashSet};

use only_semantic::{DependencyAst, TaskAst};

use crate::planner::PlanError;
use crate::resolve::{TaskIndex, bind_parameters, merge_parameter_inputs, select_task_variant};

pub(crate) type BoundTask<'a> = (usize, &'a TaskAst, HashMap<String, String>);

pub(crate) struct ExpandedExecution<'a> {
    pub ordered: Vec<BoundTask<'a>>,
    pub successors: Vec<Vec<usize>>,
}

#[derive(Default)]
struct ExecutionGraph<'a> {
    nodes: HashMap<String, (&'a TaskAst, HashMap<String, String>)>,
    registration_order: Vec<String>,
    dependencies: HashMap<String, Vec<String>>,
    edges: HashMap<String, HashSet<String>>,
}

pub(crate) fn expand_execution_order<'a>(
    root: &'a TaskAst,
    root_bindings: &HashMap<String, String>,
    tasks: &TaskIndex<'a>,
    globals: &HashMap<String, String>,
) -> Result<ExpandedExecution<'a>, PlanError> {
    let mut graph = ExecutionGraph::default();
    let mut visiting = Vec::new();
    let root_bindings = bind_parameters(root, Some(root_bindings), globals)?;
    collect_task(
        root,
        root_bindings,
        tasks,
        globals,
        &mut visiting,
        &mut graph,
    )?;
    build_execution_order(graph, &root.qualified_name())
}

fn collect_task<'a>(
    task: &'a TaskAst,
    bindings: HashMap<String, String>,
    tasks: &TaskIndex<'a>,
    globals: &HashMap<String, String>,
    visiting: &mut Vec<String>,
    graph: &mut ExecutionGraph<'a>,
) -> Result<(), PlanError> {
    let qualified_name = task.qualified_name().to_string();
    if visiting.contains(&qualified_name) {
        visiting.push(qualified_name);
        return Err(PlanError::CyclicDependency(visiting.join(" -> ")));
    }
    if let Some((_, existing_bindings)) = graph.nodes.get(&qualified_name) {
        if existing_bindings != &bindings {
            return Err(PlanError::ConflictingDependencyArguments(qualified_name));
        }
        return Ok(());
    }

    visiting.push(qualified_name.clone());
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
                let dependency_bindings = bind_dependency(dependency, dependency_task, globals)?;
                collect_task(
                    dependency_task,
                    dependency_bindings,
                    tasks,
                    globals,
                    visiting,
                    graph,
                )?;
                graph
                    .edges
                    .entry(dependency_name.clone())
                    .or_default()
                    .insert(qualified_name.clone());
                let dependencies = graph
                    .dependencies
                    .entry(qualified_name.clone())
                    .or_default();
                if !dependencies.contains(&dependency_name) {
                    dependencies.push(dependency_name.clone());
                }
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

fn bind_dependency(
    dependency: &DependencyAst,
    task: &TaskAst,
    globals: &HashMap<String, String>,
) -> Result<HashMap<String, String>, PlanError> {
    let positional_args = dependency
        .arguments
        .iter()
        .map(|argument| argument.value.as_str())
        .collect();
    let inputs = merge_parameter_inputs(positional_args, Vec::new(), task, globals)?;
    bind_parameters(task, Some(&inputs), globals).map_err(|error| match error {
        PlanError::MissingRequiredParameter(parameter) => {
            PlanError::DependencyMissingRequiredParameter {
                dependency: dependency.name.to_string(),
                parameter,
            }
        }
        other => other,
    })
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

fn build_execution_order<'a>(
    graph: ExecutionGraph<'a>,
    root_name: &str,
) -> Result<ExpandedExecution<'a>, PlanError> {
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
    let mut stages = HashMap::new();
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
            stages.insert(name, stage);
        }

        stage += 1;
    }

    let presentation_order = expand_presentation_order(root_name, &graph.dependencies);
    let indices = presentation_order
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect::<HashMap<_, _>>();
    let mut successors = vec![Vec::new(); presentation_order.len()];
    for (name, dependents) in &graph.edges {
        let source = indices[name];
        for dependent in dependents {
            successors[source].push(indices[dependent]);
        }
        successors[source].sort_unstable();
    }

    Ok(ExpandedExecution {
        ordered: presentation_order
            .into_iter()
            .map(|name| {
                let (task, bindings) = graph
                    .nodes
                    .get(&name)
                    .expect("presentation node should exist in graph");
                let stage = stages[&name];
                (stage, *task, bindings.clone())
            })
            .collect(),
        successors,
    })
}

fn expand_presentation_order(
    root_name: &str,
    dependencies: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    visit_dependencies(root_name, dependencies, &mut visited, &mut order);
    order
}

fn visit_dependencies(
    task_name: &str,
    dependencies: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    order: &mut Vec<String>,
) {
    if !visited.insert(task_name.to_string()) {
        return;
    }

    if let Some(task_dependencies) = dependencies.get(task_name) {
        for dependency in task_dependencies {
            visit_dependencies(dependency, dependencies, visited, order);
        }
    }
    order.push(task_name.to_string());
}
