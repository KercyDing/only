use crate::SyntaxKind;

pub(crate) fn consume_line(input: &mut &[SyntaxKind]) {
    while let Some(kind) = input.first().copied() {
        advance(input);
        if matches!(kind, SyntaxKind::Newline | SyntaxKind::Eof) {
            break;
        }
    }
}

pub(crate) fn starts_top_level_item(current: SyntaxKind) -> bool {
    if current == SyntaxKind::Indent {
        return false;
    }
    matches!(
        current,
        SyntaxKind::Bang
            | SyntaxKind::Percent
            | SyntaxKind::LBracket
            | SyntaxKind::Ident
            | SyntaxKind::Eof
    )
}

pub(crate) fn advance(input: &mut &[SyntaxKind]) {
    let (_, rest) = input
        .split_first()
        .expect("advance should only be called with non-empty input");
    *input = rest;
}
