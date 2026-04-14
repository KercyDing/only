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

impl SyntaxKind {
    const MAX: u16 = SyntaxKind::Error as u16;
}

impl From<SyntaxKind> for RowanSyntaxKind {
    fn from(value: SyntaxKind) -> Self {
        RowanSyntaxKind(value as u16)
    }
}

impl From<RowanSyntaxKind> for SyntaxKind {
    fn from(raw: RowanSyntaxKind) -> Self {
        if raw.0 <= Self::MAX {
            // SAFETY: SyntaxKind is #[repr(u16)] and raw.0 is within the
            // valid discriminant range [0, MAX].
            unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
        } else {
            SyntaxKind::Unknown
        }
    }
}
