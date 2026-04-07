use super::{Guard, SourceSpan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDefinition {
    pub signature: TaskSignature,
    pub doc: Option<String>,
    pub commands: Vec<CommandLine>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSignature {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub guard: Option<Guard>,
    pub dependencies: Vec<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub default_value: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    pub text: String,
    pub span: SourceSpan,
}
