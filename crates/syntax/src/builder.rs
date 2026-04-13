use crate::cst::SyntaxNode;
use crate::{LexToken, SyntaxKind};

/// Thin rowan builder wrapper used by the syntax parser.
///
/// Args:
/// None.
///
/// Returns:
/// Structured node/token emission helpers for CST construction.
pub struct ParseTreeBuilder {
    inner: rowan::GreenNodeBuilder<'static>,
}

impl ParseTreeBuilder {
    /// Creates a new document builder.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Builder positioned at the document root.
    pub fn new() -> Self {
        let mut inner = crate::cst::builder();
        inner.start_node(SyntaxKind::Document.into());
        Self { inner }
    }

    /// Emits one node wrapping the provided token slice.
    ///
    /// Args:
    /// kind: CST node kind to emit.
    /// tokens: Token slice copied into the node.
    ///
    /// Returns:
    /// None.
    pub fn push_node(&mut self, kind: SyntaxKind, tokens: &[LexToken]) {
        self.inner.start_node(kind.into());
        for token in tokens {
            self.inner.token(token.kind.into(), token.text.as_str());
        }
        self.inner.finish_node();
    }

    /// Emits raw tokens directly under the current parent node.
    ///
    /// Args:
    /// tokens: Token slice copied directly into the current parent.
    ///
    /// Returns:
    /// None.
    pub fn push_tokens(&mut self, tokens: &[LexToken]) {
        for token in tokens {
            self.inner.token(token.kind.into(), token.text.as_str());
        }
    }

    /// Finalizes the builder into a rowan root node.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Final CST root node.
    pub fn finish(mut self) -> SyntaxNode {
        self.inner.finish_node();
        SyntaxNode::new_root(self.inner.finish())
    }
}
