use only_syntax::{DiagnosticCode, ParseResultExt, SyntaxKind, parse};

#[test]
fn parses_document_with_directive_task_and_namespace() {
    let parsed = parse("!echo true\nbuild():\n    echo hi\n[dev]\nserve():\n    cargo run\n");
    let kinds: Vec<_> = parsed.root_children().map(|node| node.kind()).collect();

    assert!(kinds.contains(&SyntaxKind::Directive));
    assert!(kinds.contains(&SyntaxKind::TaskDecl));
    assert!(kinds.contains(&SyntaxKind::NamespaceBlock));
    assert!(parsed.diagnostics().is_empty());
}

#[test]
fn recovers_after_broken_task_header() {
    let parsed = parse("broken(\nnext():\n    echo next\n");
    let task_count = parsed
        .root_children()
        .filter(|node| node.kind() == SyntaxKind::TaskDecl)
        .count();
    let error_count = parsed
        .root_children()
        .filter(|node| node.kind() == SyntaxKind::Error)
        .count();

    assert_eq!(task_count, 1);
    assert_eq!(error_count, 1);
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diag| diag.code == DiagnosticCode::new("parse.malformed-task-header"))
    );
}

#[test]
fn recovers_after_unexpected_top_level_token() {
    let parsed = parse("@\n[dev]\nserve():\n    cargo run\n");
    let kinds: Vec<_> = parsed.root_children().map(|node| node.kind()).collect();

    assert!(kinds.contains(&SyntaxKind::Error));
    assert!(kinds.contains(&SyntaxKind::NamespaceBlock));
    assert!(kinds.contains(&SyntaxKind::TaskDecl));
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diag| diag.code == DiagnosticCode::new("parse.unexpected-token"))
    );
}

#[test]
fn keeps_parsing_after_comments_and_blank_lines() {
    let parsed = parse("% docs\n\n# comment\nbuild():\n    cargo build\n");
    let kinds: Vec<_> = parsed.root_children().map(|node| node.kind()).collect();

    assert!(kinds.contains(&SyntaxKind::DocComment));
    assert!(kinds.contains(&SyntaxKind::TaskDecl));
    assert!(parsed.diagnostics().is_empty());
}

#[test]
fn reports_malformed_namespace_header_and_recovers() {
    let parsed = parse("[dev\nserve():\n    cargo run\n");
    let task_count = parsed
        .root_children()
        .filter(|node| node.kind() == SyntaxKind::TaskDecl)
        .count();

    assert_eq!(task_count, 1);
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diag| diag.code == DiagnosticCode::new("parse.malformed-namespace-header"))
    );
}

#[test]
fn reports_malformed_directive_and_recovers() {
    let parsed = parse("!\nbuild():\n    cargo build\n");
    let task_count = parsed
        .root_children()
        .filter(|node| node.kind() == SyntaxKind::TaskDecl)
        .count();

    assert_eq!(task_count, 1);
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diag| diag.code == DiagnosticCode::new("parse.malformed-directive"))
    );
}

#[test]
fn reports_malformed_task_params_and_recovers() {
    let parsed = parse("build(name:\n    echo broken\nnext():\n    echo next\n");
    let task_count = parsed
        .root_children()
        .filter(|node| node.kind() == SyntaxKind::TaskDecl)
        .count();
    let error_count = parsed
        .root_children()
        .filter(|node| node.kind() == SyntaxKind::Error)
        .count();

    assert_eq!(task_count, 1);
    assert_eq!(error_count, 1);
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diag| diag.code == DiagnosticCode::new("parse.malformed-task-header"))
    );
}

#[test]
fn reports_malformed_task_guard_and_recovers() {
    let parsed = parse("build() ? env(\"CI\"):\n    echo broken\nnext():\n    echo next\n");
    let task_count = parsed
        .root_children()
        .filter(|node| node.kind() == SyntaxKind::TaskDecl)
        .count();
    let error_count = parsed
        .root_children()
        .filter(|node| node.kind() == SyntaxKind::Error)
        .count();

    assert_eq!(task_count, 1);
    assert_eq!(error_count, 1);
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diag| diag.code == DiagnosticCode::new("parse.malformed-task-header"))
    );
}

#[test]
fn reports_nested_parallel_dependency_groups_as_malformed() {
    let parsed = parse("ci() & (fmt, (lint, test)):\n    echo broken\nnext():\n    echo next\n");
    let task_count = parsed
        .root_children()
        .filter(|node| node.kind() == SyntaxKind::TaskDecl)
        .count();
    let error_count = parsed
        .root_children()
        .filter(|node| node.kind() == SyntaxKind::Error)
        .count();

    assert_eq!(task_count, 1);
    assert_eq!(error_count, 1);
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diag| diag.code == DiagnosticCode::new("parse.malformed-task-header"))
    );
}
