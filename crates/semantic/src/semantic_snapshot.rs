use only_diagnostic::Diagnostic;

use crate::{DocumentAst, SymbolIndex};

#[derive(Debug, Clone)]
pub struct SemanticSnapshot {
    pub document: DocumentAst,
    pub diagnostics: Vec<Diagnostic>,
    pub symbols: SymbolIndex,
}
