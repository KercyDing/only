use super::SourceSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    Verbose { value: bool, span: SourceSpan },
}
