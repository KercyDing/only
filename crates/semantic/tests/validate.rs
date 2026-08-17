use only_semantic::compile_document;

#[test]
fn reports_validation_errors_for_dependencies_and_variables() {
    let compiled = compile_document("deploy() & build:\n    echo {{target}}\n");
    let messages: Vec<_> = compiled
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect();

    assert!(
        messages
            .iter()
            .any(|message| message.contains("depends on missing task 'build'"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("variable 'target' is not defined"))
    );
}

#[test]
fn reports_duplicate_directives() {
    let compiled = compile_document("!shell bash\n!shell deno\n");
    let messages: Vec<_> = compiled
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect();

    assert!(messages.contains(&"`!shell` is used more than once"));
}

#[test]
fn reports_duplicate_global_variables() {
    let compiled = compile_document("!version 0.3\n!var mode = \"one\"\n!var mode = \"two\"\n");

    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "variable.duplicate")
    );
}

#[test]
fn reports_namespace_close_mismatch() {
    let compiled = compile_document("!version 0.3\n[front]\n[/back]\n");

    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "namespace.close-mismatch")
    );
}

#[test]
fn fallback_shell_requires_version_0_3() {
    let unversioned = compile_document("build() shell~=bash:\n    true\n");
    let versioned = compile_document("!version 0.3\nbuild() shell~=bash:\n    true\n");

    assert!(
        unversioned
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "semantic.engineering-version")
    );
    assert!(versioned.diagnostics.is_empty());
}

#[test]
fn reports_slice_parameter_before_final_position() {
    let compiled = compile_document("run(args.., tail):\n    echo {{args}} {{tail}}\n");
    let messages: Vec<_> = compiled
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect();

    assert!(messages.contains(&"slice parameter 'args..' must be last in task 'run'"));
}

#[test]
fn reports_slice_parameter_default_value() {
    let compiled = compile_document("run(args..=\"fetch\"):\n    echo {{args}}\n");
    let messages: Vec<_> = compiled
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect();

    assert!(messages.contains(&"slice parameter 'args..' cannot have a default"));
}
