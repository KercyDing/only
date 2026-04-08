use only_semantic::{compile_document, compile_syntax};
use only_syntax::snapshot;

#[test]
fn lowers_cst_and_collects_symbols() {
    let compiled = compile_document("build(name=\"dev\"):\n    echo {{name}}\n");

    assert!(compiled.diagnostics.is_empty());
    assert_eq!(compiled.document.tasks.len(), 1);
    assert_eq!(compiled.document.tasks[0].params.len(), 1);
    assert_eq!(compiled.symbols.tasks[0].name, "build");
}

#[test]
fn reports_undefined_dependency_and_variable() {
    let compiled = compile_document("deploy() & build:\n    echo {{target}}\n");
    let messages: Vec<_> = compiled
        .diagnostics
        .iter()
        .map(|diag| diag.message.as_str())
        .collect();

    assert!(
        messages
            .iter()
            .any(|msg| msg.contains("undefined dependency 'build'"))
    );
    assert!(
        messages
            .iter()
            .any(|msg| msg.contains("undefined variable 'target'"))
    );
}

#[test]
fn lowers_directives_and_namespaced_tasks() {
    let compiled = compile_document(concat!(
        "!verbose true\n",
        "% Developer commands.\n",
        "[dev]\n",
        "% Start the app.\n",
        "serve(port=\"3000\") & build:\n",
        "    echo {{port}}\n",
        "build():\n",
        "    cargo build\n",
    ));

    assert!(compiled.diagnostics.is_empty());
    assert_eq!(compiled.document.directives.len(), 1);
    assert_eq!(compiled.document.namespaces.len(), 1);
    assert_eq!(compiled.document.namespaces[0].name, "dev");
    assert_eq!(
        compiled.document.namespaces[0].doc.as_deref(),
        Some("Developer commands.")
    );
    assert_eq!(compiled.document.tasks.len(), 2);
    assert_eq!(compiled.document.tasks[0].namespace.as_deref(), Some("dev"));
    assert_eq!(compiled.document.tasks[0].name, "serve");
    assert_eq!(
        compiled.document.tasks[0].doc.as_deref(),
        Some("Start the app.")
    );
    assert_eq!(compiled.symbols.namespaces.len(), 1);
    assert_eq!(compiled.symbols.namespaces[0].name, "dev");
    assert_eq!(compiled.symbols.tasks[0].name, "dev.serve");
}

#[test]
fn resolves_local_namespace_dependencies() {
    let compiled = compile_document(concat!(
        "[dev]\n",
        "build():\n",
        "    cargo build\n",
        "serve() & build:\n",
        "    echo ok\n",
    ));

    assert!(compiled.diagnostics.is_empty());
    assert_eq!(compiled.document.tasks.len(), 2);
    assert_eq!(compiled.document.tasks[1].namespace.as_deref(), Some("dev"));
    assert_eq!(compiled.document.tasks[1].dependencies[0].name, "dev.build");
}

#[test]
fn reports_namespace_conflict_with_global_task() {
    let compiled = compile_document(concat!(
        "build():\n",
        "    cargo build\n",
        "[build]\n",
        "serve():\n",
        "    cargo run\n",
    ));
    let messages: Vec<_> = compiled
        .diagnostics
        .iter()
        .map(|diag| diag.message.as_str())
        .collect();

    assert!(
        messages.iter().any(|msg| msg
            == &"conflict: global task 'build' and namespace 'build' cannot coexist")
    );
}

#[test]
fn reports_duplicate_parameter_names() {
    let compiled = compile_document("build(tag, tag):\n    echo {{tag}}\n");
    let messages: Vec<_> = compiled
        .diagnostics
        .iter()
        .map(|diag| diag.message.as_str())
        .collect();

    assert!(
        messages
            .iter()
            .any(|msg| msg == &"duplicate parameter 'tag' in task 'build'")
    );
}

#[test]
fn lowers_parameter_defaults_guard_and_shell() {
    let compiled =
        compile_document("build(tag=\"v1\") ? @env(\"CI\") shell?=bash:\n    echo {{tag}}\n");

    assert!(compiled.diagnostics.is_empty());
    let task = &compiled.document.tasks[0];
    assert_eq!(task.params[0].name, "tag");
    assert_eq!(task.params[0].default_value.as_deref(), Some("v1"));
    assert_eq!(
        task.guard.as_ref().map(|guard| guard.kind.as_str()),
        Some("env")
    );
    assert_eq!(
        task.guard.as_ref().map(|guard| guard.argument.as_str()),
        Some("CI")
    );
    assert_eq!(task.shell.as_deref(), Some("bash"));
    assert!(task.shell_fallback);
}

#[test]
fn reports_duplicate_task_definition() {
    let compiled = compile_document(concat!(
        "build(tag=\"v1\"):\n",
        "    echo one\n",
        "build(tag=\"v1\"):\n",
        "    echo two\n",
    ));
    let messages: Vec<_> = compiled
        .diagnostics
        .iter()
        .map(|diag| diag.message.as_str())
        .collect();

    assert!(
        messages
            .iter()
            .any(|msg| msg == &"duplicate task definition: 'build' conflicts with 'build'")
    );
}

#[test]
fn reports_ambiguous_guard_overlap() {
    let compiled = compile_document(concat!(
        "build() ? @env(\"CI\"):\n",
        "    echo one\n",
        "build() ? @env(\"CI\"):\n",
        "    echo two\n",
    ));
    let messages: Vec<_> = compiled
        .diagnostics
        .iter()
        .map(|diag| diag.message.as_str())
        .collect();

    assert!(
        messages
            .iter()
            .any(|msg| msg == &"ambiguous guard: 'build' conflicts with 'build'")
    );
}

#[test]
fn compiles_from_existing_syntax_snapshot() {
    let syntax = snapshot("build(name=\"dev\"):\n    echo {{name}}\n");
    let compiled = compile_syntax(&syntax);

    assert!(compiled.diagnostics.is_empty());
    assert_eq!(compiled.document.tasks[0].name, "build");
    assert_eq!(compiled.document.tasks[0].params[0].name, "name");
}
