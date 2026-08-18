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

    /// Emits a task with a structured header and lossless body tokens.
    pub fn push_task(&mut self, tokens: &[LexToken]) {
        self.inner.start_node(SyntaxKind::TaskDecl.into());
        let header_end = tokens
            .iter()
            .position(|token| token.kind == SyntaxKind::Colon)
            .map_or(tokens.len(), |index| index + 1);

        self.inner.start_node(SyntaxKind::TaskHeader.into());
        self.push_task_header_tokens(&tokens[..header_end]);
        self.inner.finish_node();
        self.emit_tokens(&tokens[header_end..]);
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
        self.emit_tokens(tokens);
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

    fn push_task_header_tokens(&mut self, tokens: &[LexToken]) {
        let mut index = 0usize;
        let mut saw_name = false;

        while index < tokens.len() {
            let token = &tokens[index];
            if !saw_name && token.kind == SyntaxKind::Ident {
                self.emit_node(SyntaxKind::TaskName, &tokens[index..index + 1]);
                saw_name = true;
                index += 1;
                continue;
            }

            let (kind, end) = match token.kind {
                SyntaxKind::LParen if saw_name => {
                    let end = matching_paren_end(tokens, index);
                    self.push_parameter_list(&tokens[index..end]);
                    index = end;
                    continue;
                }
                SyntaxKind::Question => (
                    SyntaxKind::ConditionClause,
                    clause_end(tokens, index + 1, ClauseBoundary::Condition),
                ),
                SyntaxKind::Amp => (
                    SyntaxKind::DependencyClause,
                    clause_end(tokens, index + 1, ClauseBoundary::Dependency),
                ),
                SyntaxKind::ShellKw | SyntaxKind::ShellFallbackKw => (
                    SyntaxKind::ShellClause,
                    clause_end(tokens, index + 1, ClauseBoundary::Shell),
                ),
                SyntaxKind::Colon => (SyntaxKind::HeaderTerminator, index + 1),
                _ => {
                    self.emit_tokens(&tokens[index..index + 1]);
                    index += 1;
                    continue;
                }
            };

            self.emit_node(kind, &tokens[index..end]);
            index = end;
        }
    }

    fn push_parameter_list(&mut self, tokens: &[LexToken]) {
        self.inner.start_node(SyntaxKind::ParameterList.into());
        let mut index = 0usize;

        while index < tokens.len() {
            let token = &tokens[index];
            if is_parameter_start(token) {
                let end = parameter_end(tokens, index + 1);
                self.emit_node(SyntaxKind::Parameter, &tokens[index..end]);
                index = end;
            } else {
                self.emit_tokens(&tokens[index..index + 1]);
                index += 1;
            }
        }

        self.inner.finish_node();
    }

    fn emit_node(&mut self, kind: SyntaxKind, tokens: &[LexToken]) {
        self.inner.start_node(kind.into());
        self.emit_tokens(tokens);
        self.inner.finish_node();
    }

    fn emit_tokens(&mut self, tokens: &[LexToken]) {
        for token in tokens {
            self.inner.token(token.kind.into(), token.text.as_str());
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ClauseBoundary {
    Condition,
    Dependency,
    Shell,
}

fn matching_paren_end(tokens: &[LexToken], start: usize) -> usize {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match token.kind {
            SyntaxKind::LParen => depth += 1,
            SyntaxKind::RParen => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return index + 1;
                }
            }
            _ => {}
        }
    }
    tokens.len()
}

fn clause_end(tokens: &[LexToken], start: usize, boundary: ClauseBoundary) -> usize {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match token.kind {
            SyntaxKind::LParen => depth += 1,
            SyntaxKind::RParen => depth = depth.saturating_sub(1),
            SyntaxKind::Colon if depth == 0 => return index,
            SyntaxKind::Newline if depth == 0 => return index,
            SyntaxKind::Question if depth == 0 => return index,
            SyntaxKind::Amp if depth == 0 => return index,
            SyntaxKind::ShellKw | SyntaxKind::ShellFallbackKw if depth == 0 => return index,
            _ => {}
        }

        if matches!(boundary, ClauseBoundary::Condition)
            && depth == 0
            && token.kind == SyntaxKind::RParen
        {
            return index + 1;
        }
    }
    tokens.len()
}

fn is_parameter_start(token: &LexToken) -> bool {
    matches!(token.kind, SyntaxKind::Ident | SyntaxKind::ShellKw)
}

fn parameter_end(tokens: &[LexToken], start: usize) -> usize {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match token.kind {
            SyntaxKind::LParen | SyntaxKind::LBracket => depth += 1,
            SyntaxKind::RParen | SyntaxKind::RBracket if depth > 0 => depth -= 1,
            SyntaxKind::RParen if depth == 0 => return index,
            SyntaxKind::Comma if depth == 0 => return index,
            _ => {}
        }
    }
    tokens.len()
}
