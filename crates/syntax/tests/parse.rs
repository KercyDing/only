use only_syntax::{DiagnosticCode, ParseResultExt, SyntaxKind, parse};

#[test]
fn parses_document_with_directive_task_and_namespace() {
    let parsed = parse("!shell deno\nbuild():\n    echo hi\n[dev]\nserve():\n    cargo run\n");
    let kinds: Vec<_> = parsed.root_children().map(|node| node.kind()).collect();

    assert!(kinds.contains(&SyntaxKind::Directive));
    assert!(kinds.contains(&SyntaxKind::TaskDecl));
    assert!(kinds.contains(&SyntaxKind::NamespaceBlock));
    assert!(parsed.diagnostics().is_empty());
}

#[test]
fn parses_document_with_crlf_line_endings() {
    let parsed = parse("!shell deno\r\n\r\nbuild():\r\n    echo hi\r\n");
    let kinds: Vec<_> = parsed.root_children().map(|node| node.kind()).collect();

    assert!(kinds.contains(&SyntaxKind::Directive));
    assert!(kinds.contains(&SyntaxKind::TaskDecl));
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
    let parsed = parse("# docs\n\n// comment\nbuild():\n    cargo build\n");
    let kinds: Vec<_> = parsed.root_children().map(|node| node.kind()).collect();

    assert!(kinds.contains(&SyntaxKind::DocComment));
    assert!(kinds.contains(&SyntaxKind::TaskDecl));
    assert!(parsed.diagnostics().is_empty());
}

#[test]
fn top_level_comment_ends_previous_task_body() {
    let parsed = parse("build():\n    cargo build\n// comment\ncheck():\n    cargo check\n");
    let task_count = parsed
        .root_children()
        .filter(|node| node.kind() == SyntaxKind::TaskDecl)
        .count();

    assert_eq!(task_count, 2);
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
fn reports_inline_comment_in_namespace_header() {
    let parsed = parse("[dev] // comment\nserve():\n    cargo run\n");
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
fn treats_indented_namespace_braces_as_structure() {
    let parsed = parse("!version 0.3\n    [dev] {\nrun():\n    true\n    }\nroot():\n    true\n");
    let namespaces = parsed
        .root_children()
        .filter(|node| node.kind() == SyntaxKind::NamespaceBlock)
        .count();

    assert_eq!(namespaces, 2);
    assert!(parsed.diagnostics().is_empty());
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
fn reports_inline_comment_in_directive() {
    let parsed = parse("!shell bash // comment\nbuild():\n    cargo build\n");
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
fn reports_inline_comment_in_task_header() {
    let parsed = parse("build(): // comment\nnext():\n    echo next\n");
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

#[test]
fn rejects_old_fallback_shell_operator() {
    let parsed = parse("build() shell?=bash:\n    true\n");

    assert!(parsed
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::new("parse.malformed-task-header")));
}
