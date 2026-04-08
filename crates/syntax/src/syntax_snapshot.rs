use only_diagnostic::Diagnostic;

use crate::ast_view::DocumentNode;
use crate::parse::parse_tokens;
use crate::{LexToken, ParseResult, ParseResultExt, SyntaxNode, lex};

/// Immutable syntax snapshot shared by semantic analysis and editor hosts.
///
/// Args:
/// None.
///
/// Returns:
/// Token stream and parsed CST for a single in-memory source snapshot.
#[derive(Debug, Clone)]
pub struct SyntaxSnapshot {
    pub tokens: Vec<LexToken>,
    pub parse: ParseResult,
}

impl SyntaxSnapshot {
    /// Returns the typed document CST root.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Typed document wrapper for the snapshot root.
    pub fn document(&self) -> DocumentNode {
        self.parse.document()
    }

    /// Returns the CST root node for this snapshot.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Parsed rowan root node.
    pub fn root(&self) -> &SyntaxNode {
        &self.parse.root
    }

    /// Returns collected syntax diagnostics.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Borrowed parse diagnostics for this snapshot.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        self.parse.diagnostics()
    }
}

/// Lexes and parses source text into an immutable syntax snapshot.
///
/// Args:
/// source: Raw Onlyfile source text.
///
/// Returns:
/// Snapshot containing tokens, CST and syntax diagnostics.
pub fn snapshot(source: &str) -> SyntaxSnapshot {
    let tokens = lex(source);
    let parse = parse_tokens(&tokens);

    SyntaxSnapshot { tokens, parse }
}
