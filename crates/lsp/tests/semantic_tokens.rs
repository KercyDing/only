use only_lsp::{DocumentSnapshot, LspSemanticTokenKind, semantic_tokens};

#[test]
fn classifies_engineering_syntax() {
    let source = concat!(
        "!version 0.3\n",
        "!var profile = \"release\"\n",
        "[dev] {\n",
        "build(target=\"x\") ? @env(\"CI\") & prepare shell=bash:\n",
        "    echo {{target}}\n",
        "}\n",
    );
    let tokens = semantic_tokens(&DocumentSnapshot::new("file:///Onlyfile", 1, source));

    let variable = tokens
        .iter()
        .find(|token| token.kind == LspSemanticTokenKind::Variable)
        .expect("variable name should have variable highlighting");
    assert_eq!(
        &source[usize::from(variable.range.start())..usize::from(variable.range.end())],
        "profile"
    );
    let directives = tokens
        .iter()
        .filter(|token| token.kind == LspSemanticTokenKind::Directive)
        .map(|token| &source[usize::from(token.range.start())..usize::from(token.range.end())])
        .collect::<Vec<_>>();
    assert!(directives.contains(&"!version"));
    assert!(directives.contains(&"!var"));
    let namespace = tokens
        .iter()
        .find(|token| token.kind == LspSemanticTokenKind::Namespace)
        .expect("namespace name should have namespace highlighting");
    assert_eq!(
        &source[usize::from(namespace.range.start())..usize::from(namespace.range.end())],
        "dev"
    );
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == LspSemanticTokenKind::Guard)
    );
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == LspSemanticTokenKind::Dependency)
    );
    let shell = tokens
        .iter()
        .find(|token| token.kind == LspSemanticTokenKind::Shell)
        .expect("shell clause should have shell highlighting");
    assert_eq!(
        &source[usize::from(shell.range.start())..usize::from(shell.range.end())],
        "shell=bash"
    );
}
