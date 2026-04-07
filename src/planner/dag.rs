#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPlan {
    pub nodes: Vec<ExecutionNode>,
    pub verbose: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionNode {
    pub qualified_name: String,
    pub commands: Vec<String>,
}
