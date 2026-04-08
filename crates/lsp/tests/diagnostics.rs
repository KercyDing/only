use only_lsp::{DocumentSnapshot, LspDiagnosticSeverity, diagnostics};

#[test]
fn maps_semantic_diagnostics_into_lsp_values() {
    let snapshot = DocumentSnapshot::new(
        "file:///workspace/Onlyfile",
        1,
        "deploy() & build:\n    echo {{target}}\n",
    );

    let diagnostics = diagnostics(&snapshot);

    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "semantic.undefined-dependency")
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity == LspDiagnosticSeverity::Error)
    );
}
