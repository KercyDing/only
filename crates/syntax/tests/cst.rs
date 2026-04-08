use only_syntax::snapshot;

#[test]
fn exposes_typed_top_level_nodes() {
    let parsed = only_syntax::parse(
        "!verbose true\n% Developer tasks.\n[dev]\nserve(port=\"3000\"):\n    echo {{port}}\n",
    );
    let document = parsed.document();

    let directive = document
        .directives()
        .next()
        .expect("directive should exist");
    assert_eq!(directive.name().as_deref(), Some("verbose"));
    assert_eq!(directive.value().as_deref(), Some("true"));

    let doc = document
        .doc_comments()
        .next()
        .expect("doc comment should exist");
    assert_eq!(doc.text().as_deref(), Some("Developer tasks."));

    let namespace = document
        .namespaces()
        .next()
        .expect("namespace should exist");
    assert_eq!(namespace.name().as_deref(), Some("dev"));

    let task = document.tasks().next().expect("task should exist");
    assert_eq!(task.name().as_deref(), Some("serve"));
    assert_eq!(task.header_text().as_deref(), Some("serve(port=\"3000\")"));
    assert_eq!(task.commands().collect::<Vec<_>>(), vec!["echo {{port}}"]);
    assert!(!task.range().is_empty());
}

#[test]
fn snapshot_exposes_typed_document_root() {
    let syntax = snapshot("build():\n    cargo build\n");
    let document = syntax.document();
    let task = document.tasks().next().expect("task should exist");

    assert_eq!(task.name().as_deref(), Some("build"));
    assert_eq!(task.commands().collect::<Vec<_>>(), vec!["cargo build"]);
}

#[test]
fn exposes_structured_task_header_sections() {
    let syntax = snapshot(
        "build(tag=\"v1\") ? @env(\"CI\") & install & bootstrap shell?=bash:\n    echo {{tag}}\n",
    );
    let task = syntax.document().tasks().next().expect("task should exist");

    assert_eq!(task.params_text().as_deref(), Some("tag=\"v1\""));
    assert_eq!(task.guard_text().as_deref(), Some("@env(\"CI\")"));
    assert_eq!(
        task.dependencies_text().as_deref(),
        Some("install & bootstrap")
    );
    assert_eq!(task.shell_name().as_deref(), Some("bash"));
    assert!(task.shell_fallback());
}
