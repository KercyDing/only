use only_syntax::{DiagnosticCode, ParseResultExt, SyntaxKind, parse};

#[test]
fn parses_document_with_directive_task_and_group() {
    let parsed =
        parse("!shell deno\nbuild():\n    echo hi\ngroup dev {\nserve():\n    cargo run\n}\n");
    let kinds: Vec<_> = parsed.root_children().map(|node| node.kind()).collect();

    assert!(kinds.contains(&SyntaxKind::Directive));
    assert!(kinds.contains(&SyntaxKind::TaskDecl));
    assert!(kinds.contains(&SyntaxKind::NamespaceBlock));
    assert!(parsed.diagnostics().is_empty());
}

#[test]
fn multiline_task_header_inside_group() {
    let parsed = parse(concat!(
        "!version 0.4\n",
        "group back {\n",
        "    ci()\n",
        "        & fmt\n",
        "        & check\n",
        "        & clippy\n",
        "        & test\n",
        "    :\n",
        "        echo done\n",
        "}\n",
    ));

    assert_eq!(
        parsed
            .root_children()
            .filter(|node| node.kind() == SyntaxKind::TaskDecl)
            .count(),
        1
    );
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
fn parses_task_without_colon() {
    let parsed = parse(
        "_prepare():\n    true\nci() & _prepare & (check, test)\ncheck():\n    true\ntest():\n    true\n",
    );

    assert!(
        parsed.diagnostics().is_empty(),
        "{:?}",
        parsed.diagnostics()
    );
    let task = parsed
        .document()
        .tasks()
        .find(|task| task.header().and_then(|header| header.name()).as_deref() == Some("ci"))
        .expect("dependency-only task should be present");
    assert!(task.header().expect("task header").terminator().is_none());
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
    let parsed = parse("@\ngroup dev {\nserve():\n    cargo run\n}\n");
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
    let parsed = parse("# docs\n\nbuild():\n    cargo build\n");
    let kinds: Vec<_> = parsed.root_children().map(|node| node.kind()).collect();

    assert!(kinds.contains(&SyntaxKind::TaskDecl));
    assert!(parsed.diagnostics().is_empty());
}

#[test]
fn parses_task_metadata_without_group_conflicts() {
    let parsed = parse(
        "[help] Build the project\n[unknown] ignored text\nbuild():\n    true\ngroup dev {\n}\n",
    );
    let kinds = parsed
        .root_children()
        .map(|node| node.kind())
        .collect::<Vec<_>>();

    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::MetadataComment)
            .count(),
        2
    );
    assert!(kinds.contains(&SyntaxKind::NamespaceBlock));
    assert!(parsed.diagnostics().is_empty());
}

#[test]
fn top_level_comment_ends_previous_task_body() {
    let parsed = parse("build():\n    cargo build\n# comment\ncheck():\n    cargo check\n");
    let task_count = parsed
        .root_children()
        .filter(|node| node.kind() == SyntaxKind::TaskDecl)
        .count();

    assert_eq!(task_count, 2);
    assert!(parsed.diagnostics().is_empty());
}

#[test]
fn treats_indented_group_braces_as_structure() {
    let parsed =
        parse("!version 0.4\n    group dev {\nrun():\n    true\n    }\nroot():\n    true\n");
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
    let parsed = parse("!shell bash # comment\nbuild():\n    cargo build\n");
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
    let parsed = parse("build(): # comment\nnext():\n    echo next\n");
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
fn rejects_inline_task_command() {
    let parsed = parse("build(): cargo build\nnext():\n    cargo check\n");
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
    assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code
        == DiagnosticCode::new("parse.malformed-task-header")));
}

#[test]
fn rejects_inline_command() {
    let parsed = parse("build() cargo build\nnext():\n    cargo check\n");

    assert_eq!(
        parsed
            .root_children()
            .filter(|node| node.kind() == SyntaxKind::TaskDecl)
            .count(),
        1
    );
    assert!(parsed.diagnostics().iter().any(|diagnostic| diagnostic.code
        == DiagnosticCode::new("parse.malformed-task-header")));
}

#[test]
fn accepts_task_header_at_eof() {
    let parsed = parse("build():");

    assert!(parsed.diagnostics().is_empty());
    assert!(
        parsed
            .root_children()
            .any(|node| node.kind() == SyntaxKind::TaskDecl)
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
fn parses_dependency_arguments() {
    let parsed = parse("build(profile):\n    echo {{profile}}\nci() & build(\"dev\"):\n    true\n");

    assert!(parsed.diagnostics().is_empty());
}

#[test]
fn parses_arguments_in_parallel_dependencies() {
    let parsed = parse(concat!(
        "build(profile):\n    echo {{profile}}\n",
        "test(profile):\n    echo {{profile}}\n",
        "ci() & (build(\"dev\"), test(\"ci\")):\n    true\n",
    ));

    assert!(parsed.diagnostics().is_empty());
}

#[test]
fn rejects_non_string_dependency_arguments() {
    let parsed = parse("build(profile):\n    true\nci() & build(dev):\n    true\n");

    assert!(parsed
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::new("parse.malformed-task-header")));
}
