#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPlan {
    pub nodes: Vec<ExecutionNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionNode {
    pub qualified_name: String,
    pub command_count: usize,
}
