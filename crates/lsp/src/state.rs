use std::collections::BTreeMap;

use text_size::TextSize;

use crate::{
    DocumentSnapshot, LspDiagnostic, LspDocumentSymbol, LspFoldingRange, LspHover, diagnostics,
    folding, hover, symbols,
};

/// In-memory document state for one LSP session.
///
/// Args:
/// None.
///
/// Returns:
/// URI-keyed immutable snapshots built from editor text, not filesystem reads.
#[derive(Debug, Default)]
pub struct WorkspaceState {
    documents: BTreeMap<String, DocumentSnapshot>,
}

impl WorkspaceState {
    /// Creates an empty workspace state.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Empty URI-keyed snapshot store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces one in-memory document snapshot.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    /// version: Monotonic document version.
    /// source: Current in-memory source text.
    ///
    /// Returns:
    /// None.
    pub fn upsert(&mut self, uri: &str, version: i32, source: &str) {
        self.documents
            .insert(uri.to_owned(), DocumentSnapshot::new(uri, version, source));
    }

    /// Removes one document snapshot from the in-memory store.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    ///
    /// Returns:
    /// Previous snapshot when present.
    pub fn remove(&mut self, uri: &str) -> Option<DocumentSnapshot> {
        self.documents.remove(uri)
    }

    /// Returns the current snapshot for one URI.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    ///
    /// Returns:
    /// Borrowed immutable document snapshot when present.
    pub fn snapshot(&self, uri: &str) -> Option<&DocumentSnapshot> {
        self.documents.get(uri)
    }

    /// Returns diagnostics from the current snapshot.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    ///
    /// Returns:
    /// Borrowed semantic diagnostics for the document.
    pub fn diagnostics(&self, uri: &str) -> Option<Vec<LspDiagnostic>> {
        self.snapshot(uri).map(diagnostics::diagnostics)
    }

    /// Returns hover information from the current snapshot.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    /// offset: Source offset queried by the editor host.
    ///
    /// Returns:
    /// Hover information for the matching source item.
    pub fn hover(&self, uri: &str, offset: TextSize) -> Option<LspHover> {
        self.snapshot(uri)
            .and_then(|snapshot| hover::hover(snapshot, offset))
    }

    /// Returns document symbols from the current snapshot.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    ///
    /// Returns:
    /// Flat document symbols for the current source snapshot.
    pub fn document_symbols(&self, uri: &str) -> Option<Vec<LspDocumentSymbol>> {
        self.snapshot(uri).map(symbols::symbols)
    }

    /// Returns folding ranges from the current snapshot.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    ///
    /// Returns:
    /// Folding ranges for the current source snapshot.
    pub fn folding_ranges(&self, uri: &str) -> Option<Vec<LspFoldingRange>> {
        self.snapshot(uri).map(folding::folding_ranges)
    }
}
