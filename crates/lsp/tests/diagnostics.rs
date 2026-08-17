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

#[test]
fn reports_version_gate_diagnostic() {
    let snapshot = DocumentSnapshot::new(
        "file:///workspace/Onlyfile",
        1,
        "!version 0.1\nbuild():\n    true\n",
    );
    let diagnostics = diagnostics(&snapshot);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "version.incompatible");
    assert!(snapshot.semantic.document.tasks.is_empty());
}
