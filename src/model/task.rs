use super::{Guard, ShellKind, SourceSpan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDefinition {
    pub signature: TaskSignature,
    pub doc: Option<String>,
    pub commands: Vec<CommandLine>,
    pub span: SourceSpan,
}

impl TaskDefinition {
    /// Returns the fully-qualified task name used in diagnostics.
    ///
    /// Args:
    /// namespace: Optional namespace prefix.
    ///
    /// Returns:
    /// Fully-qualified task display name.
    pub fn display_name(&self, namespace: Option<&str>) -> String {
        match namespace {
            Some(namespace) => format!("{namespace}.{}", self.signature.name),
            None => self.signature.name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSignature {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub guard: Option<Guard>,
    pub dependencies: Vec<String>,
    pub shell: Option<ShellKind>,
    pub shell_fallback: bool,
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
