use rowan::SyntaxKind as RowanSyntaxKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum SyntaxKind {
    Ident,
    String,
    Comment,
    Whitespace,
    Newline,
    Indent,
    Bang,
    Percent,
    Colon,
    Question,
    Amp,
    Eq,
    At,
    LParen,
    RParen,
    LBracket,
    RBracket,
    ShellKw,
    ShellFallbackKw,
    Unknown,
    Eof,
    Document,
    Directive,
    DocComment,
    NamespaceBlock,
    TaskDecl,
    Error,
}

impl From<SyntaxKind> for RowanSyntaxKind {
    fn from(value: SyntaxKind) -> Self {
        RowanSyntaxKind(value as u16)
    }
}
