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
fn reports_duplicate_dependency() {
    let snapshot = DocumentSnapshot::new(
        "file:///workspace/Onlyfile",
        1,
        "prepare():\n    true\n\nci() & prepare & prepare:\n    true\n",
    );
    let diagnostics = diagnostics(&snapshot);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "semantic.duplicate-dependency");
    assert_eq!(
        diagnostics[0].message,
        "dependency 'prepare' is repeated in task 'ci'"
    );
}

#[test]
fn leaves_runner_version_checks_to_the_cli() {
    let snapshot = DocumentSnapshot::new(
        "file:///workspace/Onlyfile",
        1,
        "!version 0.4\nbuild():\n    true\n",
    );
    let diagnostics = diagnostics(&snapshot);

    assert!(diagnostics.is_empty());
    assert_eq!(snapshot.semantic.document.tasks.len(), 1);
}

#[test]
fn accepts_multiline_header_inside_group() {
    let snapshot = DocumentSnapshot::new(
        "file:///workspace/Onlyfile",
        1,
        concat!(
            "!version 0.4\n",
            "group back {\n",
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
        "!version 0.4\nprepare():\n    true\nci() & prepare\n",
    );

    assert!(
        diagnostics(&snapshot).is_empty(),
        "{:?}",
        diagnostics(&snapshot)
    );
    assert_eq!(snapshot.semantic.document.tasks.len(), 2);
}

#[test]
fn accepts_task_without_newline() {
    let snapshot = DocumentSnapshot::new(
        "file:///workspace/Onlyfile",
        1,
        "!version 0.4\n[help] Example\nabc()",
    );

    assert!(diagnostics(&snapshot).is_empty());
    assert_eq!(snapshot.semantic.document.tasks.len(), 1);
}

#[test]
fn accepts_indented_task_header() {
    let snapshot = DocumentSnapshot::new(
        "file:///workspace/Onlyfile",
        1,
        "!version 0.4\n[help] Example\nabc()\n    ",
    );

    assert!(diagnostics(&snapshot).is_empty());
    assert_eq!(snapshot.semantic.document.tasks.len(), 1);
}

#[test]
fn reports_missing_task_colon() {
    let source = "!version 0.4\n[help] Example\nabc()\n    echo example";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let diagnostics = diagnostics(&snapshot);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "parse.missing-task-colon");
    assert_eq!(diagnostics[0].message, "missing ':' before command");
}

#[test]
fn reports_empty_task_body() {
    let source = "!version 0.4\n[help] Example\nabc():";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let diagnostics = diagnostics(&snapshot);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "semantic.empty-task-body");
    assert_eq!(
        diagnostics[0].message,
        "task body is empty; add a command or remove ':'"
    );
    assert_eq!(
        &source[usize::from(diagnostics[0].range.start())..usize::from(diagnostics[0].range.end())],
        ":"
    );
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
