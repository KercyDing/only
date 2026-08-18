use std::fmt;
use std::path::PathBuf;

use only_semantic::{DirectiveAst, DocumentAst, ShellKind, ShellSelection, TaskAst};

use crate::dag::expand_execution_order;
use crate::resolve::{
    build_execution_nodes, build_task_index, document_shell, merge_parameter_inputs,
    resolve_root_task, resolve_root_task_in_document,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation<'a> {
    Task {
        target: &'a str,
        args: Vec<&'a str>,
        overrides: Vec<(&'a str, &'a str)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionNode {
    pub stage: usize,
    pub name: String,
    pub steps: Vec<ExecutionStep>,
    pub params: Vec<PlanParam>,
    pub result_params: Vec<PlanParam>,
    pub pass: Option<String>,
    pub fail: Option<String>,
    pub shell: Option<ShellSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStep {
    Command(String),
    CommandBlock { source: String, line_count: usize },
}

impl ExecutionStep {
    pub fn source(&self) -> &str {
        match self {
            Self::Command(command) => command,
            Self::CommandBlock { source, .. } => source,
        }
    }

    pub fn is_block(&self) -> bool {
        matches!(self, Self::CommandBlock { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanParam {
    pub name: String,
    pub default_value: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExecutionPlan {
    pub nodes: Vec<ExecutionNode>,
    pub shell: Option<ShellKind>,
    pub working_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    UnknownTask(String),
    HelperTask(String),
    TaskUnavailable(String),
    MissingRequiredParameter(String),
    UnknownParameter {
        task: String,
        name: String,
    },
    DuplicateOverride(String),
    CyclicDependency(String),
    TooManyArguments {
        task: String,
        expected: usize,
        got: usize,
    },
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTask(task) => write!(f, "task '{task}' does not exist"),
            Self::HelperTask(task) => {
                write!(f, "helper task '{task}' cannot be run directly")
            }
            Self::TaskUnavailable(task) => {
                write!(f, "task '{task}' is not available on this system")
            }
            Self::MissingRequiredParameter(name) => {
                write!(f, "parameter '{{{{{name}}}}}' is required")
            }
            Self::UnknownParameter { task, name } => {
                write!(f, "task '{task}' has no parameter named '{name}'")
            }
            Self::DuplicateOverride(name) => {
                write!(f, "parameter '{name}' was given more than once")
            }
            Self::CyclicDependency(path) => write!(f, "dependency loop: {path}"),
            Self::TooManyArguments {
                task,
                expected,
                got,
            } => match expected {
                0 => write!(f, "task '{task}' accepts no arguments, but got {got}"),
                1 => write!(f, "task '{task}' accepts 1 argument, but got {got}"),
                _ => write!(
                    f,
                    "task '{task}' accepts {expected} arguments, but got {got}"
                ),
            },
        }
    }
}

impl std::error::Error for PlanError {}

/// Builds a dependency-ordered execution plan from semantic AST.
///
/// Args:
/// document: Semantic AST used by the runtime.
/// invocation: Requested task target.
///
/// Returns:
/// Dependency-expanded execution plan in DAG order.
pub fn build_execution_plan(document: &DocumentAst, invocation: Invocation<'_>) -> ExecutionPlan {
    let working_dir = std::env::current_dir().unwrap_or_default();
    try_build_execution_plan_in_dir(document, invocation, working_dir).unwrap_or_default()
}

/// Builds a dependency-ordered execution plan from semantic AST and returns planner errors.
///
/// Args:
/// document: Semantic AST used by the runtime.
/// invocation: Requested task target plus input bindings.
///
/// Returns:
/// Dependency-expanded execution plan in DAG order.
pub fn try_build_execution_plan(
    document: &DocumentAst,
    invocation: Invocation<'_>,
) -> Result<ExecutionPlan, PlanError> {
    let working_dir = std::env::current_dir().unwrap_or_default();
    try_build_execution_plan_in_dir(document, invocation, working_dir)
}

/// Builds a dependency-ordered execution plan from semantic AST for an explicit working directory.
///
/// Args:
/// document: Semantic AST used by the runtime.
/// invocation: Requested task target plus input bindings.
/// working_dir: Directory used by runtime execution.
///
/// Returns:
/// Dependency-expanded execution plan in DAG order.
pub fn try_build_execution_plan_in_dir(
    document: &DocumentAst,
    invocation: Invocation<'_>,
    working_dir: PathBuf,
) -> Result<ExecutionPlan, PlanError> {
    let Invocation::Task {
        target,
        args,
        overrides,
    } = invocation;

    let tasks = build_task_index(document);
    let root = resolve_root_task(&tasks, target)?;
    let globals = document
        .directives
        .iter()
        .filter_map(|directive| match directive {
            DirectiveAst::Variable { name, value, .. } => {
                Some((name.to_string(), value.to_string()))
            }
            _ => None,
        })
        .collect::<std::collections::HashMap<_, _>>();
    let overrides = merge_parameter_inputs(args, overrides, root, &globals)?;
    let mut effective_globals = globals.clone();
    for (name, value) in &overrides {
        if globals.contains_key(name)
            && !root.params.iter().any(|param| param.name.as_str() == name)
        {
            effective_globals.insert(name.clone(), value.clone());
        }
    }
    let ordered = expand_execution_order(root, &overrides, &tasks, &effective_globals)?;

    Ok(ExecutionPlan {
        nodes: build_execution_nodes(ordered),
        shell: document_shell(document),
        working_dir,
    })
}

/// Resolves the concrete root task variant selected for the current environment.
///
/// Args:
/// document: Semantic AST used by the runtime.
/// target: Fully qualified task target.
///
/// Returns:
/// The selected root task variant or a planner error when unavailable.
pub fn select_root_task_variant<'a>(
    document: &'a DocumentAst,
    target: &str,
) -> Result<&'a TaskAst, PlanError> {
    resolve_root_task_in_document(document, target)
}
