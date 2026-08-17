use only_lsp::{DocumentSnapshot, LspSemanticTokenKind, semantic_tokens};

#[test]
fn classifies_engineering_syntax() {
    let source = concat!(
        "!version 0.3\n",
        "!var profile = \"release\"\n",
        "[dev]\n",
        "build(target=\"x\") ? @env(\"CI\") & prepare shell=bash:\n",
        "    echo {{target}}\n",
        "[/dev]\n",
    );
    let tokens = semantic_tokens(&DocumentSnapshot::new("file:///Onlyfile", 1, source));

    assert!(
        tokens
            .iter()
            .any(|token| token.kind == LspSemanticTokenKind::Variable)
    );
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == LspSemanticTokenKind::Namespace)
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
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == LspSemanticTokenKind::Shell)
    );
}
