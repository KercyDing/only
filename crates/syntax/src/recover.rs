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
            | SyntaxKind::Comment
            | SyntaxKind::GroupKw
            | SyntaxKind::LBracket
            | SyntaxKind::RBrace
            | SyntaxKind::Ident
            | SyntaxKind::Eof
    )
}

pub(crate) fn starts_indented_namespace_boundary(input: &[SyntaxKind]) -> bool {
    let mut index = 0;
    if input.get(index) != Some(&SyntaxKind::Indent) {
        return false;
    }
    while matches!(
        input.get(index),
        Some(SyntaxKind::Indent | SyntaxKind::Whitespace)
    ) {
        index += 1;
    }

    if input.get(index) == Some(&SyntaxKind::RBrace) {
        index += 1;
        while input.get(index) == Some(&SyntaxKind::Whitespace) {
            index += 1;
        }
        return matches!(
            input.get(index),
            Some(SyntaxKind::Newline | SyntaxKind::Eof)
        );
    }

    starts_braced_namespace(&input[index..])
}

pub(crate) fn starts_indented_namespace_member(input: &[SyntaxKind]) -> bool {
    if starts_indented_namespace_boundary(input) {
        return true;
    }

    let mut index = 0;
    if input.get(index) != Some(&SyntaxKind::Indent) {
        return false;
    }
    while matches!(
        input.get(index),
        Some(SyntaxKind::Indent | SyntaxKind::Whitespace)
    ) {
        index += 1;
    }

    if matches!(
        input.get(index),
        Some(SyntaxKind::Bang | SyntaxKind::GroupKw | SyntaxKind::Comment | SyntaxKind::LBracket)
    ) {
        return true;
    }
    if input.get(index) != Some(&SyntaxKind::Ident) {
        return false;
    }
    index += 1;
    while input.get(index) == Some(&SyntaxKind::Whitespace) {
        index += 1;
    }
    input.get(index) == Some(&SyntaxKind::LParen)
}

fn starts_braced_namespace(input: &[SyntaxKind]) -> bool {
    let mut index = 0;
    if input.get(index) != Some(&SyntaxKind::GroupKw) {
        return false;
    }
    index += 1;

    while input.get(index) == Some(&SyntaxKind::Whitespace) {
        index += 1;
    }
    if input.get(index) != Some(&SyntaxKind::Ident) {
        return false;
    }
    index += 1;
    while input.get(index) == Some(&SyntaxKind::Whitespace) {
        index += 1;
    }
    if input.get(index) != Some(&SyntaxKind::LBrace) {
        return false;
    }
    index += 1;
    while input.get(index) == Some(&SyntaxKind::Whitespace) {
        index += 1;
    }
    matches!(
        input.get(index),
        Some(SyntaxKind::Newline | SyntaxKind::Eof)
    )
}

pub(crate) fn advance(input: &mut &[SyntaxKind]) {
    let (_, rest) = input
        .split_first()
        .expect("advance should only be called with non-empty input");
    *input = rest;
}
