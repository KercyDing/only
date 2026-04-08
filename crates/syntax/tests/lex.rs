use only_syntax::{SyntaxKind, lex};

#[test]
fn lexes_directive_task_and_trivia() {
    let tokens = lex("!verbose true\nbuild():\n    echo hi\n");
    let kinds: Vec<_> = tokens.iter().map(|token| token.kind).collect();

    assert!(kinds.contains(&SyntaxKind::Bang));
    assert!(kinds.contains(&SyntaxKind::Ident));
    assert!(kinds.contains(&SyntaxKind::Newline));
    assert!(kinds.contains(&SyntaxKind::Indent));
    assert!(tokens.iter().any(|token| token.text.as_str() == "build"));
}

#[test]
fn keeps_comment_and_unknown_tokens() {
    let tokens = lex("% doc\n# tail\n@\n");
    assert!(tokens.iter().any(|token| token.kind == SyntaxKind::Percent));
    assert!(tokens.iter().any(|token| token.kind == SyntaxKind::Comment));
    assert!(tokens.iter().any(|token| token.kind == SyntaxKind::At));
}
