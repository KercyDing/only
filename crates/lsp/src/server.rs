use text_size::TextSize;

use crate::{
    DocumentSnapshot, LspDiagnostic, LspDocumentSymbol, LspFoldingRange, LspHover, WorkspaceState,
};

/// Thin LSP host facade around the in-memory workspace state.
///
/// Args:
/// None.
///
/// Returns:
/// Stable server-facing entry points for document sync and analysis queries.
#[derive(Debug, Default)]
pub struct LanguageServer {
    workspace: WorkspaceState,
}

impl LanguageServer {
    /// Creates an empty language server facade.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Empty server with an empty workspace state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens or replaces one in-memory document.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    /// version: Monotonic document version.
    /// source: Current in-memory source text.
    ///
    /// Returns:
    /// None.
    pub fn open_document(&mut self, uri: &str, version: i32, source: &str) {
        self.workspace.upsert(uri, version, source);
    }

    /// Applies an in-memory document change.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    /// version: Monotonic document version.
    /// source: Current in-memory source text.
    ///
    /// Returns:
    /// None.
    pub fn change_document(&mut self, uri: &str, version: i32, source: &str) {
        self.workspace.upsert(uri, version, source);
    }

    /// Closes one in-memory document.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    ///
    /// Returns:
    /// Previous document snapshot when present.
    pub fn close_document(&mut self, uri: &str) -> Option<DocumentSnapshot> {
        self.workspace.remove(uri)
    }

    /// Returns the current snapshot for one URI.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    ///
    /// Returns:
    /// Borrowed immutable document snapshot when present.
    pub fn snapshot(&self, uri: &str) -> Option<&DocumentSnapshot> {
        self.workspace.snapshot(uri)
    }

    /// Returns mapped diagnostics for one document.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    ///
    /// Returns:
    /// Host-facing diagnostics when the document exists.
    pub fn diagnostics(&self, uri: &str) -> Option<Vec<LspDiagnostic>> {
        self.workspace.diagnostics(uri)
    }

    /// Returns mapped hover information for one document and source offset.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    /// offset: Source offset queried by the editor host.
    ///
    /// Returns:
    /// Host-facing hover payload when one item matches the offset.
    pub fn hover(&self, uri: &str, offset: TextSize) -> Option<LspHover> {
        self.workspace.hover(uri, offset)
    }

    /// Returns mapped document symbols for one document.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    ///
    /// Returns:
    /// Host-facing document symbols when the document exists.
    pub fn document_symbols(&self, uri: &str) -> Option<Vec<LspDocumentSymbol>> {
        self.workspace.document_symbols(uri)
    }

    /// Returns mapped folding ranges for one document.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    ///
    /// Returns:
    /// Host-facing folding ranges when the document exists.
    pub fn folding_ranges(&self, uri: &str) -> Option<Vec<LspFoldingRange>> {
        self.workspace.folding_ranges(uri)
    }
}
