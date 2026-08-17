mod ast_view;
mod builder;
mod builtin;
mod cst;
mod cursor;
mod formatter;
mod kind;
mod lex;
mod parse;
mod recover;
mod syntax_snapshot;
mod token;
mod trivia;
mod version;

pub use ast_view::{
    DependencyClauseNode, DirectiveNode, DocCommentNode, DocumentNode, GuardClauseNode,
    HeaderTerminatorNode, NamespaceNode, ParameterListNode, ParameterNode, ShellClauseNode,
    TaskCommandBlockNode, TaskCommandNode, TaskDependencyRef, TaskGuardRef, TaskHeaderInfo,
    TaskHeaderNode, TaskNode, TaskParamRef, TaskStepNode,
};
pub use builtin::{
    DirectiveKind, GuardKind, ShellKind, ShellOperator, ShellSelection, TaskShellRef,
};
pub use cst::{SyntaxNode, SyntaxToken};
pub use formatter::{format_range, format_source};
pub use kind::SyntaxKind;
pub use lex::lex;
pub use only_diagnostic::DiagnosticCode;
pub use parse::{ParseResult, ParseResultExt, parse};
pub use syntax_snapshot::{SyntaxSnapshot, snapshot};
pub use token::LexToken;
pub use version::{
    BootstrapHeader, VersionRequirement, bootstrap, check_version_compatibility,
    parse_version_requirement, scan_bootstrap_header,
};
