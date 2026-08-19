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
fn leaves_runner_version_checks_to_the_cli() {
    let snapshot = DocumentSnapshot::new(
        "file:///workspace/Onlyfile",
        1,
        "!version 0.3\nbuild():\n    true\n",
    );
    let diagnostics = diagnostics(&snapshot);

    assert!(diagnostics.is_empty());
    assert_eq!(snapshot.semantic.document.tasks.len(), 1);
}

#[test]
fn accepts_multiline_header_inside_namespace() {
    let snapshot = DocumentSnapshot::new(
        "file:///workspace/Onlyfile",
        1,
        concat!(
            "!version 0.3\n",
            "[back] {\n",
            "    fmt():\n",
            "        true\n",
            "    check():\n",
            "        true\n",
            "    ci()\n",
            "        & fmt\n",
            "        & check\n",
            "    :\n",
            "        true\n",
            "}\n",
        ),
    );

    assert!(diagnostics(&snapshot).is_empty());
}

#[test]
fn reports_inline_task_command() {
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, "build(): cargo build\n");

    let diagnostics = diagnostics(&snapshot);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "parse.malformed-task-header")
    );
}

#[test]
fn accepts_dependency_task() {
    let snapshot = DocumentSnapshot::new(
        "file:///workspace/Onlyfile",
        1,
        "!version 0.3\nprepare():\n    true\nci() & prepare\n",
    );

    assert!(
        diagnostics(&snapshot).is_empty(),
        "{:?}",
        diagnostics(&snapshot)
    );
    assert_eq!(snapshot.semantic.document.tasks.len(), 2);
}

#[test]
fn accepts_multiline_dependency_task() {
    let snapshot = DocumentSnapshot::new(
        "file:///workspace/Onlyfile",
        1,
        "!version 0.4\nprepare():\n    true\nci()\n    & prepare\n",
    );

    assert!(
        diagnostics(&snapshot).is_empty(),
        "{:?}",
        diagnostics(&snapshot)
    );
}
