mod directive;
mod namespace;
mod probe;
mod span;
mod task;

pub use directive::{Directive, ShellKind};
pub use namespace::Namespace;
pub use probe::{Guard, ProbeCall, ProbeKind};
pub use span::{SourceSpan, Spanned};
pub use task::{CommandLine, Parameter, TaskDefinition, TaskSignature};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Onlyfile {
    pub directives: Vec<Directive>,
    pub global_tasks: Vec<TaskDefinition>,
    pub namespaces: Vec<Namespace>,
}
