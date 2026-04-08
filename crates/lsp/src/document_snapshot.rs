use only_semantic::{SemanticSnapshot, compile_syntax};
use only_syntax::{SyntaxSnapshot, snapshot};

/// In-memory LSP document snapshot with syntax and semantic compilation results.
///
/// Args:
/// None.
///
/// Returns:
/// Stable container for one document version and its derived compiler snapshots.
#[derive(Debug, Clone)]
pub struct DocumentSnapshot {
    pub uri: String,
    pub version: i32,
    pub source: String,
    pub syntax: SyntaxSnapshot,
    pub semantic: SemanticSnapshot,
}

impl DocumentSnapshot {
    /// Creates a new in-memory document snapshot for LSP consumers.
    ///
    /// Args:
    /// uri: Stable document identifier from the editor.
    /// version: Monotonic document version from the editor.
    /// source: Current in-memory source text.
    ///
    /// Returns:
    /// Snapshot containing source text plus matching syntax and semantic results.
    pub fn new(uri: &str, version: i32, source: &str) -> Self {
        let syntax = snapshot(source);
        let semantic = compile_syntax(&syntax);

        Self {
            uri: uri.to_owned(),
            version,
            source: source.to_owned(),
            syntax,
            semantic,
        }
    }

    /// Reparses the same document URI with a newer in-memory version.
    ///
    /// Args:
    /// version: Monotonic document version from the editor.
    /// source: Current in-memory source text.
    ///
    /// Returns:
    /// New snapshot bound to the same URI.
    pub fn reparse(&self, version: i32, source: &str) -> Self {
        Self::new(&self.uri, version, source)
    }
}
