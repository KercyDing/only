use only_lsp::{DocumentSnapshot, LspSemanticTokenKind, semantic_tokens};

#[test]
fn classifies_engineering_syntax() {
    let source = concat!(
        "!version 0.4\n",
        "!var profile = \"release\"\n",
        "group dev {\n",
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
        .expect("group name should have group highlighting");
    assert_eq!(
        &source[usize::from(namespace.range.start())..usize::from(namespace.range.end())],
        "dev"
    );
    let guard = tokens
        .iter()
        .find(|token| token.kind == LspSemanticTokenKind::Guard)
        .expect("guard name should have guard highlighting");
    assert_eq!(
        &source[usize::from(guard.range.start())..usize::from(guard.range.end())],
        "@env"
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

#[test]
fn classifies_task_metadata_fields() {
    let source = "!version 0.4\n[help] Deploy\n[pass] Done\ndeploy():\n    true\n";
    let tokens = semantic_tokens(&DocumentSnapshot::new("file:///Onlyfile", 1, source));
    let tags = tokens
        .iter()
        .filter(|token| token.kind == LspSemanticTokenKind::Metadata)
        .map(|token| &source[usize::from(token.range.start())..usize::from(token.range.end())])
        .collect::<Vec<_>>();

    assert_eq!(tags, vec!["[help]", "[pass]"]);
}

#[test]
fn uses_variable_tokens_for_interpolation_names() {
    let source = concat!(
        "!version 0.4\n",
        "!var cargo_flags = \"--all-targets\"\n",
        "[desc] Build with {{cargo_flags}}\n",
        "build():\n",
        "    echo {{cargo_flags}}\n",
    );
    let tokens = semantic_tokens(&DocumentSnapshot::new("file:///Onlyfile", 1, source));
    let variables = tokens
        .iter()
        .filter(|token| token.kind == LspSemanticTokenKind::Variable)
        .map(|token| &source[usize::from(token.range.start())..usize::from(token.range.end())])
        .collect::<Vec<_>>();

    assert_eq!(variables, vec!["cargo_flags", "cargo_flags", "cargo_flags"]);
}

#[test]
fn classifies_block_markers() {
    let source = "!version 0.4\ninstall():\n    | # Resolve paths.\n    | echo ok\n    |\n";
    let tokens = semantic_tokens(&DocumentSnapshot::new("file:///Onlyfile", 1, source));
    let markers = tokens
        .iter()
        .filter(|token| token.kind == LspSemanticTokenKind::BlockMarker)
        .map(|token| &source[usize::from(token.range.start())..usize::from(token.range.end())])
        .collect::<Vec<_>>();

    assert_eq!(markers, vec!["|", "|", "|"]);
}
