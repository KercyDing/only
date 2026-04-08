use only_lsp::DocumentSnapshot;

#[test]
fn reparses_from_memory_snapshot() {
    let snapshot = DocumentSnapshot::new(
        "file:///workspace/Onlyfile",
        7,
        "build(name=\"dev\"):\n    echo {{name}}\n",
    );

    assert_eq!(snapshot.uri, "file:///workspace/Onlyfile");
    assert_eq!(snapshot.version, 7);
    assert_eq!(snapshot.syntax.tokens[0].text.as_str(), "build");
    assert!(snapshot.semantic.diagnostics.is_empty());
    assert_eq!(snapshot.semantic.document.tasks[0].name, "build");
}

#[test]
fn keeps_uri_when_reparsing_new_version() {
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 7, "build():\n    true\n");
    let reparsed = snapshot.reparse(8, "deploy() & build:\n    true\n");

    assert_eq!(reparsed.uri, "file:///workspace/Onlyfile");
    assert_eq!(reparsed.version, 8);
    assert!(
        reparsed
            .semantic
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("undefined dependency 'build'"))
    );
}
