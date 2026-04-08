use text_size::TextRange;

use crate::DocumentSnapshot;

/// Host-facing document symbol kind used by the LSP crate.
///
/// Args:
/// None.
///
/// Returns:
/// Stable symbol categories for editor protocol conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspDocumentSymbolKind {
    Namespace,
    Task,
}

/// Host-facing document symbol entry for one source item.
///
/// Args:
/// None.
///
/// Returns:
/// Symbol metadata detached from semantic internals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDocumentSymbol {
    pub name: String,
    pub detail: Option<String>,
    pub kind: LspDocumentSymbolKind,
    pub range: TextRange,
    pub container_name: Option<String>,
}

/// Builds document symbols from one in-memory document snapshot.
///
/// Args:
/// snapshot: In-memory document snapshot with semantic analysis.
///
/// Returns:
/// LSP host-facing symbol list in source order.
pub fn symbols(snapshot: &DocumentSnapshot) -> Vec<LspDocumentSymbol> {
    only_semantic::document_symbols(&snapshot.semantic)
        .into_iter()
        .map(|symbol| LspDocumentSymbol {
            name: symbol.name.to_string(),
            detail: symbol.detail.map(|detail| detail.to_string()),
            kind: match symbol.kind {
                only_semantic::DocumentSymbolKind::Namespace => LspDocumentSymbolKind::Namespace,
                only_semantic::DocumentSymbolKind::Task => LspDocumentSymbolKind::Task,
            },
            range: symbol.range,
            container_name: symbol.container_name.map(|name| name.to_string()),
        })
        .collect()
}
