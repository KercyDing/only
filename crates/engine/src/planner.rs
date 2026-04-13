use std::fmt;
use std::path::PathBuf;

use only_semantic::DocumentAst;

use crate::dag::expand_execution_order;
use crate::resolve::{
    build_execution_nodes, build_task_index, document_echo, document_shell, merge_parameter_inputs,
    resolve_root_task,
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
    pub commands: Vec<String>,
    pub params: Vec<PlanParam>,
    pub shell: Option<String>,
    pub shell_fallback: bool,
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
    pub echo: bool,
    pub shell: Option<String>,
    pub working_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    UnknownTask(String),
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
            Self::UnknownTask(task) => write!(f, "task '{task}' is not defined"),
            Self::TaskUnavailable(task) => {
                write!(f, "task '{task}' is not available for this environment")
            }
            Self::MissingRequiredParameter(name) => {
                write!(f, "missing required parameter '{{{{{name}}}}}'")
            }
            Self::UnknownParameter { task, name } => {
                write!(f, "unknown parameter '{name}' for task '{task}'")
            }
            Self::DuplicateOverride(name) => write!(f, "duplicate parameter override '{name}'"),
            Self::CyclicDependency(path) => write!(f, "cyclic dependency detected: {path}"),
            Self::TooManyArguments {
                task,
                expected,
                got,
            } => write!(
                f,
                "too many arguments for task '{task}'; expected at most {expected}, got {got}"
            ),
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
    let overrides = merge_parameter_inputs(args, overrides, root)?;
    let ordered = expand_execution_order(root, &overrides, &tasks)?;

    Ok(ExecutionPlan {
        nodes: build_execution_nodes(ordered),
        echo: document_echo(document),
        shell: document_shell(document),
        working_dir,
    })
}
