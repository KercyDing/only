use only_syntax::{SyntaxKind, lex};

#[test]
fn lexes_directive_task_and_trivia() {
    let tokens = lex("!shell deno\nbuild():\n    echo hi\n");
    let kinds: Vec<_> = tokens.iter().map(|token| token.kind).collect();

    assert!(kinds.contains(&SyntaxKind::Bang));
    assert!(kinds.contains(&SyntaxKind::Ident));
    assert!(kinds.contains(&SyntaxKind::Newline));
    assert!(kinds.contains(&SyntaxKind::Indent));
    assert!(tokens.iter().any(|token| token.text.as_str() == "build"));
}

#[test]
fn keeps_comment_and_unknown_tokens() {
    let tokens = lex("# doc\n@\n");
    assert_eq!(
        tokens
            .iter()
            .filter(|token| token.kind == SyntaxKind::Comment)
            .count(),
        1
    );
    assert!(tokens.iter().any(|token| token.kind == SyntaxKind::At));
}

#[test]
fn lexes_crlf_as_newlines_without_unknown_tokens() {
    let tokens = lex("!shell deno\r\n\r\nbuild():\r\n    echo hi\r\n");
    let kinds: Vec<_> = tokens.iter().map(|token| token.kind).collect();

    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::Newline)
            .count(),
        4
    );
    assert!(!kinds.contains(&SyntaxKind::Unknown));
}

#[test]
fn accepts_bom_only_at_file_start() {
    let leading = lex("\u{feff}!shell deno\n");
    let interior = lex("!shell deno\n\u{feff}build():\n    true\n");

    assert_eq!(leading[0].kind, SyntaxKind::Bom);
    assert!(
        interior
            .iter()
            .any(|token| token.kind == SyntaxKind::Unknown)
    );
}

#[test]
fn lexes_fallback_shell_operator() {
    let tokens = lex("build() shell~=bash:\n    true\n");

    assert!(
        tokens
            .iter()
            .any(|token| { token.kind == SyntaxKind::ShellFallbackKw && token.text == "shell~=" })
    );
    assert!(
        !lex("build() shell?=bash:\n    true\n")
            .iter()
            .any(|token| token.kind == SyntaxKind::ShellFallbackKw)
    );
}

#[test]
fn lexes_group_braces() {
    let tokens = lex("group dev {\n}\n");
    let kinds = tokens.iter().map(|token| token.kind).collect::<Vec<_>>();

    assert!(kinds.contains(&SyntaxKind::LBrace));
    assert!(kinds.contains(&SyntaxKind::RBrace));
    assert!(!kinds.contains(&SyntaxKind::Unknown));
}
