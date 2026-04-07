use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPlan {
    pub nodes: Vec<ExecutionNode>,
    pub verbose: bool,
    pub working_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionNode {
    pub qualified_name: String,
    pub commands: Vec<String>,
    pub parameters: Vec<(String, String)>,
}
