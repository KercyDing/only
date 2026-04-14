use only_syntax::snapshot;

#[test]
fn exposes_typed_top_level_nodes() {
    let parsed = only_syntax::parse(
        "!echo true\n% Developer tasks.\n[dev]\nserve(port=\"3000\"):\n    echo {{port}}\n",
    );
    let document = parsed.document();

    let directive = document
        .directives()
        .next()
        .expect("directive should exist");
    assert_eq!(directive.name().as_deref(), Some("echo"));
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
    let header = task.header_info();

    assert_eq!(header.params.as_deref(), Some("tag=\"v1\""));
    assert_eq!(header.guard.as_deref(), Some("@env(\"CI\")"));
    assert_eq!(header.dependencies.as_deref(), Some("install & bootstrap"));
    assert_eq!(header.shell.as_deref(), Some("bash"));
    assert!(header.shell_fallback);

    let dependency_refs = header.dependency_refs;
    assert_eq!(dependency_refs.len(), 2);
    assert_eq!(dependency_refs[0].name.as_str(), "install");
    assert_eq!(dependency_refs[0].stage, 0);
    assert_eq!(dependency_refs[1].name.as_str(), "bootstrap");
    assert_eq!(dependency_refs[1].stage, 1);
}

#[test]
fn exposes_dependency_ranges_for_hover_and_diagnostics() {
    let source = "ci() & (fmt, dev.build) & test shell?=bash:\n    echo ok\n";
    let syntax = snapshot(source);
    let task = syntax.document().tasks().next().expect("task should exist");
    let dependency_refs = task.header_info().dependency_refs;

    assert_eq!(dependency_refs.len(), 3);
    assert_eq!(dependency_refs[0].name.as_str(), "fmt");
    assert_eq!(dependency_refs[0].stage, 0);
    assert_eq!(
        &source[usize::from(dependency_refs[0].range.start())
            ..usize::from(dependency_refs[0].range.end())],
        "fmt"
    );
    assert_eq!(dependency_refs[1].name.as_str(), "dev.build");
    assert_eq!(dependency_refs[1].stage, 0);
    assert_eq!(
        &source[usize::from(dependency_refs[1].range.start())
            ..usize::from(dependency_refs[1].range.end())],
        "dev.build"
    );
    assert_eq!(dependency_refs[2].name.as_str(), "test");
    assert_eq!(dependency_refs[2].stage, 1);
}

#[test]
fn preserves_multiple_install_task_variants_in_repo_onlyfile() {
    let syntax = snapshot(include_str!("../../../Onlyfile"));
    let install_count = syntax
        .document()
        .tasks()
        .filter(|task| task.name().as_deref() == Some("install"))
        .count();

    assert_eq!(install_count, 2);
}
