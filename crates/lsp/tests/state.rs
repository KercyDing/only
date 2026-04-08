use only_lsp::WorkspaceState;
use text_size::TextSize;

#[test]
fn stores_and_replaces_document_snapshots_by_uri() {
    let mut state = WorkspaceState::new();
    state.upsert("file:///workspace/Onlyfile", 1, "build():\n    true\n");
    state.upsert(
        "file:///workspace/Onlyfile",
        2,
        "deploy() & build:\n    true\n",
    );

    let snapshot = state
        .snapshot("file:///workspace/Onlyfile")
        .expect("snapshot should exist");

    assert_eq!(snapshot.version, 2);
    assert_eq!(snapshot.source, "deploy() & build:\n    true\n");
}

#[test]
fn reads_diagnostics_and_editor_analysis_from_same_snapshot() {
    let mut state = WorkspaceState::new();
    let uri = "file:///workspace/Onlyfile";
    let source = "% Start the app.\nserve(port=\"3000\"):\n    echo {{port}}\n";
    state.upsert(uri, 1, source);

    let diagnostics = state.diagnostics(uri).expect("diagnostics should exist");
    assert!(diagnostics.is_empty());

    let hover = state
        .hover(
            uri,
            TextSize::from(source.find("serve").expect("task name should exist") as u32),
        )
        .expect("hover should exist");
    assert_eq!(hover.name.as_str(), "serve");

    let symbols = state
        .document_symbols(uri)
        .expect("document symbols should exist");
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "serve");

    let folding = state.folding_ranges(uri).expect("folding should exist");
    assert_eq!(folding.len(), 1);
}
