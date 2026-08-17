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
fn reports_close_without_namespace() {
    let compiled = compile_document("!version 0.3\n}\n");

    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "namespace.close-without-open")
    );
}

#[test]
fn requires_namespace_close_in_version_0_3() {
    let compiled = compile_document("!version 0.3\n[front] {\nrun():\n    true\n");

    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "namespace.missing-close")
    );
}

#[test]
fn requires_namespace_open_brace_in_version_0_3() {
    let compiled = compile_document("!version 0.3\n[front]\nrun():\n    true\n");

    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "namespace.missing-open-brace")
    );
}

#[test]
fn rejects_implicit_namespace_switch() {
    let compiled = compile_document(
        "!version 0.3\n[front] {\ncheck():\n    true\n[back] {\ncheck():\n    true\n}\n",
    );
    let missing_close = compiled
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code.as_str() == "namespace.missing-close")
        .collect::<Vec<_>>();

    assert_eq!(missing_close.len(), 1);
    assert!(missing_close[0].message.contains("'front'"));
}

#[test]
fn keeps_legacy_namespaces_compatible_before_version_0_3() {
    let compiled = compile_document("[front]\nrun():\n    true\n");

    assert!(compiled.diagnostics.is_empty());
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
fn rejects_unknown_shell_fallback() {
    let compiled = compile_document("!version 0.3\nbuild() shell~=pw:\n    true\n");

    let diagnostic = compiled
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "semantic.invalid-shell-fallback")
        .expect("unknown fallback shell should be rejected");
    assert!(diagnostic.message.contains("shell 'pw' has no fallback"));
    assert!(diagnostic.message.contains("'pwsh' -> 'powershell'"));
    let start = "!version 0.3\nbuild() ".len();
    assert_eq!(
        usize::from(diagnostic.primary_range.start()),
        start,
        "diagnostic should start at the shell clause"
    );
}

#[test]
fn rejects_unknown_guard() {
    let source = "install() ? @s(\"windows\"):\n    true\n";
    let compiled = compile_document(source);

    let diagnostic = compiled
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "semantic.unknown-guard")
        .expect("unknown guard should be rejected");
    assert_eq!(
        diagnostic.message,
        "guard '@s' is not supported; use '@os', '@arch', '@env', or '@has'"
    );
    assert_eq!(
        usize::from(diagnostic.primary_range.start()),
        source.find('?').expect("guard should exist")
    );
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
