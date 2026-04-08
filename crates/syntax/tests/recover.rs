use only_syntax::{DiagnosticCode, ParseResultExt, SyntaxKind, parse};

#[test]
fn inserts_error_node_for_malformed_task_header_and_recovers() {
    let parsed = parse("build(name:\n    echo broken\nnext():\n    echo next\n");
    let kinds: Vec<_> = parsed.root_children().map(|node| node.kind()).collect();

    assert!(kinds.contains(&SyntaxKind::Error));
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::TaskDecl)
            .count(),
        1
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diag| diag.code == DiagnosticCode::new("parse.malformed-task-header"))
    );
}

#[test]
fn inserts_error_node_for_malformed_directive_and_recovers() {
    let parsed = parse("!\nbuild():\n    cargo build\n");
    let kinds: Vec<_> = parsed.root_children().map(|node| node.kind()).collect();

    assert!(kinds.contains(&SyntaxKind::Error));
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::TaskDecl)
            .count(),
        1
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diag| diag.code == DiagnosticCode::new("parse.malformed-directive"))
    );
}

#[test]
fn recovers_across_blank_lines_and_comments_after_error() {
    let parsed = parse("build(name:\n# keep going\n\nnext():\n    echo next\n");
    let kinds: Vec<_> = parsed.root_children().map(|node| node.kind()).collect();

    assert!(kinds.contains(&SyntaxKind::Error));
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::TaskDecl)
            .count(),
        1
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|diag| diag.code == DiagnosticCode::new("parse.malformed-task-header"))
    );
}
