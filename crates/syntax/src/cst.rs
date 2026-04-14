use rowan::{GreenNodeBuilder, Language, SyntaxKind as RowanSyntaxKind};

use crate::SyntaxKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OnlyLanguage {}

impl Language for OnlyLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: RowanSyntaxKind) -> Self::Kind {
        SyntaxKind::from(raw)
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
