use smol_str::SmolStr;
use text_size::TextRange;

use crate::{GuardKind, ShellKind, ShellSelection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentAst {
    pub directives: Vec<DirectiveAst>,
    pub namespaces: Vec<NamespaceAst>,
    pub tasks: Vec<TaskAst>,
    pub uses_braced_namespaces: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectiveAst {
    Version {
        major: u64,
        minor: u64,
        range: TextRange,
    },
    Shell {
        shell: ShellKind,
        range: TextRange,
    },
    Variable {
        name: SmolStr,
        value: SmolStr,
        name_range: TextRange,
        range: TextRange,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceAst {
    pub name: SmolStr,
    pub doc: Option<SmolStr>,
    pub metadata: TaskMetadataAst,
    pub range: TextRange,
    pub close_range: Option<TextRange>,
    pub is_braced: bool,
    pub is_group: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAst {
    pub name: SmolStr,
    pub namespace: Option<SmolStr>,
    /// Legacy alias for the task help text.
    pub doc: Option<SmolStr>,
    pub metadata: TaskMetadataAst,
    pub params: Vec<ParamAst>,
    pub guards: Vec<GuardAst>,
    pub dependencies: Vec<DependencyAst>,
    pub shell: Option<ShellAst>,
    pub steps: Vec<TaskStepAst>,
    pub range: TextRange,
    pub uses_multiline_header: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskMetadataAst {
    pub help: Option<SmolStr>,
    pub help_count: usize,
    pub desc: Option<SmolStr>,
    pub pass: Option<SmolStr>,
    pub fail: Option<SmolStr>,
    pub unknown_fields: Vec<SmolStr>,
    pub has_structured_fields: bool,
}

impl TaskAst {
    pub fn qualified_name(&self) -> SmolStr {
        match &self.namespace {
            Some(namespace) => SmolStr::from(format!("{namespace}.{}", self.name)),
            None => self.name.clone(),
        }
    }

    pub fn signature(&self) -> SmolStr {
        let mut signature = self.name.to_string();
        signature.push('(');
        signature.push_str(
            &self
                .params
                .iter()
                .map(
                    |parameter| match (parameter.is_slice, &parameter.default_value) {
                        (true, Some(default)) => format!("{}..=\"{default}\"", parameter.name),
                        (true, None) => format!("{}..", parameter.name),
                        (false, Some(default)) => format!("{}=\"{default}\"", parameter.name),
                        (false, None) => parameter.name.to_string(),
                    },
                )
                .collect::<Vec<_>>()
                .join(", "),
        );
        signature.push(')');
        SmolStr::from(signature)
    }

    /// Returns whether the task is a helper task hidden from normal CLI listings.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// `true` when the task name starts with `_`.
    pub fn is_helper(&self) -> bool {
        self.name.starts_with('_')
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamAst {
    pub name: SmolStr,
    pub default_value: Option<SmolStr>,
    pub is_slice: bool,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardAst {
    pub kind: GuardKind,
    pub argument: SmolStr,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellAst {
    pub selection: ShellSelection,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyAst {
    pub name: SmolStr,
    pub range: TextRange,
    pub arguments: Vec<DependencyArgumentAst>,
    pub invocation_range: TextRange,
    pub stage: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyArgumentAst {
    pub value: SmolStr,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandAst {
    pub text: SmolStr,
    pub interpolations: Vec<InterpolationAst>,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStepAst {
    Command(CommandAst),
    CommandBlock(CommandBlockAst),
}

impl TaskStepAst {
    pub fn source(&self) -> &str {
        match self {
            Self::Command(command) => command.text.as_str(),
            Self::CommandBlock(block) => block.source.as_str(),
        }
    }

    pub fn interpolations(&self) -> &[InterpolationAst] {
        match self {
            Self::Command(command) => &command.interpolations,
            Self::CommandBlock(block) => &block.interpolations,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandBlockAst {
    pub source: SmolStr,
    pub interpolations: Vec<InterpolationAst>,
    pub range: TextRange,
    pub line_ranges: Vec<TextRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpolationAst {
    pub name: SmolStr,
    pub range: TextRange,
}
