use smol_str::SmolStr;
use text_size::TextRange;

use crate::SemanticSnapshot;

/// Kind of document symbol produced for editor consumers.
///
/// Args:
/// None.
///
/// Returns:
/// Stable symbol categories for namespace and task items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentSymbolKind {
    Namespace,
    Task,
}

/// Flat document symbol entry for editor consumers.
///
/// Args:
/// None.
///
/// Returns:
/// Symbol name, container and text range for one source item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbol {
    pub name: SmolStr,
    pub detail: Option<SmolStr>,
    pub kind: DocumentSymbolKind,
    pub range: TextRange,
    pub container_name: Option<SmolStr>,
}

/// Builds flat document symbols from one semantic snapshot.
///
/// Args:
/// snapshot: Immutable semantic snapshot for one document version.
///
/// Returns:
/// Flat symbol list for namespaces and tasks in source order.
pub fn document_symbols(snapshot: &SemanticSnapshot) -> Vec<DocumentSymbol> {
    let namespaces = snapshot
        .document
        .namespaces
        .iter()
        .map(|namespace| DocumentSymbol {
            name: namespace.name.clone(),
            detail: namespace.doc.clone(),
            kind: DocumentSymbolKind::Namespace,
            range: namespace.range,
            container_name: None,
        });

    let tasks = snapshot.document.tasks.iter().map(|task| DocumentSymbol {
        name: task.name.clone(),
        detail: Some(task.signature()),
        kind: DocumentSymbolKind::Task,
        range: task.range,
        container_name: task.namespace.clone(),
    });

    namespaces.chain(tasks).collect()
}
