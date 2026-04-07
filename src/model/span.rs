#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceSpan {
    pub offset: usize,
    pub length: usize,
}

impl SourceSpan {
    pub const fn new(offset: usize, length: usize) -> Self {
        Self { offset, length }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: SourceSpan,
}

impl<T> Spanned<T> {
    pub const fn new(value: T, span: SourceSpan) -> Self {
        Self { value, span }
    }
}
