use rowan::{GreenNodeBuilder, Language, SyntaxKind as RowanSyntaxKind};

use crate::SyntaxKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OnlyLanguage {}

impl Language for OnlyLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: RowanSyntaxKind) -> Self::Kind {
        match raw.0 {
            x if x == SyntaxKind::Ident as u16 => SyntaxKind::Ident,
            x if x == SyntaxKind::String as u16 => SyntaxKind::String,
            x if x == SyntaxKind::Comment as u16 => SyntaxKind::Comment,
            x if x == SyntaxKind::Whitespace as u16 => SyntaxKind::Whitespace,
            x if x == SyntaxKind::Newline as u16 => SyntaxKind::Newline,
            x if x == SyntaxKind::Indent as u16 => SyntaxKind::Indent,
            x if x == SyntaxKind::Bang as u16 => SyntaxKind::Bang,
            x if x == SyntaxKind::Percent as u16 => SyntaxKind::Percent,
            x if x == SyntaxKind::Colon as u16 => SyntaxKind::Colon,
            x if x == SyntaxKind::Question as u16 => SyntaxKind::Question,
            x if x == SyntaxKind::Amp as u16 => SyntaxKind::Amp,
            x if x == SyntaxKind::Eq as u16 => SyntaxKind::Eq,
            x if x == SyntaxKind::At as u16 => SyntaxKind::At,
            x if x == SyntaxKind::LParen as u16 => SyntaxKind::LParen,
            x if x == SyntaxKind::RParen as u16 => SyntaxKind::RParen,
            x if x == SyntaxKind::LBracket as u16 => SyntaxKind::LBracket,
            x if x == SyntaxKind::RBracket as u16 => SyntaxKind::RBracket,
            x if x == SyntaxKind::ShellKw as u16 => SyntaxKind::ShellKw,
            x if x == SyntaxKind::ShellFallbackKw as u16 => SyntaxKind::ShellFallbackKw,
            x if x == SyntaxKind::Unknown as u16 => SyntaxKind::Unknown,
            x if x == SyntaxKind::Eof as u16 => SyntaxKind::Eof,
            x if x == SyntaxKind::Document as u16 => SyntaxKind::Document,
            x if x == SyntaxKind::Directive as u16 => SyntaxKind::Directive,
            x if x == SyntaxKind::DocComment as u16 => SyntaxKind::DocComment,
            x if x == SyntaxKind::NamespaceBlock as u16 => SyntaxKind::NamespaceBlock,
            x if x == SyntaxKind::TaskDecl as u16 => SyntaxKind::TaskDecl,
            x if x == SyntaxKind::Error as u16 => SyntaxKind::Error,
            _ => SyntaxKind::Unknown,
        }
    }

    fn kind_to_raw(kind: Self::Kind) -> RowanSyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<OnlyLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<OnlyLanguage>;

pub fn builder() -> GreenNodeBuilder<'static> {
    GreenNodeBuilder::new()
}
