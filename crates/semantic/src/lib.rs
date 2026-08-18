mod analysis;
mod ast;
mod document_symbols;
mod folding;
mod hover;
mod interpolation;
mod lower;
mod names;
mod semantic_snapshot;
mod symbols;
mod validate;

pub use analysis::{
    compile_document, compile_document_for_runner, compile_document_with_runner, compile_syntax,
};
pub use ast::{
    CommandAst, CommandBlockAst, DependencyAst, DirectiveAst, DocumentAst, GuardAst,
    InterpolationAst, NamespaceAst, ParamAst, ShellAst, TaskAst, TaskMetadataAst, TaskStepAst,
};
pub use document_symbols::{DocumentSymbol, DocumentSymbolKind, document_symbols};
pub use folding::{FoldingRange, FoldingRangeKind, folding_ranges};
pub use hover::{HoverInfo, HoverKind, hover_at};
pub use interpolation::interpolation_name_ranges;
pub use only_syntax::{
    DirectiveKind, GuardKind, MetadataKind, ShellKind, ShellOperator, ShellSelection, TaskShellRef,
};
pub use semantic_snapshot::SemanticSnapshot;
pub use symbols::{NamespaceSymbol, SymbolIndex, TaskSymbol};
