use crate::SyntaxKind;

pub(crate) fn is_trivia(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Newline | SyntaxKind::Whitespace | SyntaxKind::Indent | SyntaxKind::Comment
    )
}

pub(crate) fn line_has_non_trivia(input: &[SyntaxKind]) -> bool {
    input
        .iter()
        .take_while(|kind| !matches!(kind, SyntaxKind::Newline | SyntaxKind::Eof))
        .copied()
        .any(|kind| !is_trivia(kind))
}

pub(crate) fn line_contains_kind(input: &[SyntaxKind], expected: SyntaxKind) -> bool {
    input
        .iter()
        .take_while(|kind| !matches!(kind, SyntaxKind::Newline | SyntaxKind::Eof))
        .copied()
        .any(|kind| kind == expected)
}
