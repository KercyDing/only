use crate::trivia::is_trivia;
use crate::{LexToken, SyntaxKind};

pub(crate) struct TokenCursor<'a> {
    tokens: &'a [LexToken],
    kinds: &'a [SyntaxKind],
    index: usize,
}

impl<'a> TokenCursor<'a> {
    pub(crate) fn new(tokens: &'a [LexToken], kinds: &'a [SyntaxKind]) -> Self {
        Self {
            tokens,
            kinds,
            index: 0,
        }
    }

    pub(crate) fn skip_trivia(&mut self) -> &'a [LexToken] {
        let start = self.index;
        while self.current_kind().is_some_and(is_trivia) {
            self.index += 1;
        }
        &self.tokens[start..self.index]
    }

    pub(crate) fn current(&self) -> Option<&'a LexToken> {
        self.tokens.get(self.index)
    }

    pub(crate) fn remaining(&self) -> &'a [SyntaxKind] {
        &self.kinds[self.index..]
    }

    pub(crate) fn consume(&mut self, count: usize) -> &'a [LexToken] {
        let start = self.index;
        let end = start + count;
        self.index = end;
        &self.tokens[start..end]
    }

    fn current_kind(&self) -> Option<SyntaxKind> {
        self.kinds.get(self.index).copied()
    }
}
