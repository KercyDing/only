use super::{SourceSpan, TaskDefinition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namespace {
    pub name: String,
    pub span: SourceSpan,
    pub tasks: Vec<TaskDefinition>,
}
