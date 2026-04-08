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

pub use analysis::{compile_document, compile_syntax};
pub use ast::{
    CommandAst, DependencyAst, DirectiveAst, DocumentAst, GuardAst, InterpolationAst, NamespaceAst,
    ParamAst, TaskAst,
};
pub use document_symbols::{DocumentSymbol, DocumentSymbolKind, document_symbols};
pub use folding::{FoldingRange, FoldingRangeKind, folding_ranges};
pub use hover::{HoverInfo, HoverKind, hover_at};
pub use semantic_snapshot::SemanticSnapshot;
pub use symbols::{NamespaceSymbol, SymbolIndex, TaskSymbol};
