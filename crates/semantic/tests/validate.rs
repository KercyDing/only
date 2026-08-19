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
fn rejects_duplicate_dependencies() {
    let source = "prepare():\n    true\n\nci() & prepare & prepare:\n    true\n";
    let compiled = compile_document(source);
    let diagnostic = compiled
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "semantic.duplicate-dependency")
        .expect("duplicate dependency should be rejected");

    assert_eq!(
        diagnostic.message,
        "dependency 'prepare' is repeated in task 'ci'"
    );
    assert_eq!(
        usize::from(diagnostic.primary_range.start()),
        source
            .rfind("prepare")
            .expect("second dependency should exist")
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
    let compiled = compile_document("!version 0.4\n!var mode = \"one\"\n!var mode = \"two\"\n");

    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "variable.duplicate")
    );
}

#[test]
fn reports_close_without_group() {
    let compiled = compile_document("!version 0.4\n}\n");

    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "namespace.close-without-open")
    );
}

#[test]
fn suggests_required_shell() {
    for shell in ["sh", "deno", "powershell"] {
        let source = format!("!version 0.4\nbuild() shell~={shell}:\n    true\n");
        let compiled = compile_document(&source);
        let diagnostic = compiled
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code.as_str() == "semantic.invalid-shell-fallback")
            .expect("fallback should be rejected");

        assert_eq!(
            diagnostic.message,
            format!("shell '{shell}' has no fallback; use `shell={shell}`")
        );
    }
}

#[test]
fn rejects_unknown_shell() {
    let compiled = compile_document("!version 0.4\nbuild() shell~=pw:\n    true\n");

    let diagnostic = compiled
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "semantic.unknown-shell")
        .expect("unknown shell should be rejected");
    assert!(diagnostic.message.contains("shell 'pw' is not supported"));
    assert!(diagnostic.message.contains("'pwsh'"));
    let start = "!version 0.4\nbuild() ".len();
    assert_eq!(
        usize::from(diagnostic.primary_range.start()),
        start,
        "diagnostic should start at the shell clause"
    );
}

#[test]
fn rejects_unknown_default_shell() {
    let compiled = compile_document("!shell custom\nbuild():\n    true\n");

    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "semantic.unknown-shell")
    );
}

#[test]
fn rejects_unknown_required_shell() {
    let compiled = compile_document("build() shell=custom:\n    true\n");

    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "semantic.unknown-shell")
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

#[test]
fn metadata_interpolation_only_uses_global_variables() {
    let compiled = compile_document(
        "!version 0.4\n!var output = \"dist\"\n[pass] wrote {{output}}\n[fail] {{name}} failed\nbuild(name):\n    true\n",
    );

    let diagnostics = compiled
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code.as_str() == "semantic.metadata-variable")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("name"));
}

#[test]
fn desc_can_omit_help() {
    let compiled = compile_document("!version 0.4\n[desc] Details\n_private():\n    true\n");

    assert!(
        compiled.diagnostics.is_empty(),
        "{:?}",
        compiled.diagnostics
    );
}

#[test]
fn rejects_duplicate_help_and_group_result_messages() {
    let source = "!version 0.4\n[help] One\n[help] Two\n[pass] Done\ngroup dev {\n    build():\n        true\n}\n";
    let compiled = compile_document(source);

    let duplicate_help = compiled
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "semantic.duplicate-help")
        .expect("duplicate help should be rejected");
    assert_eq!(
        &source[usize::from(duplicate_help.primary_range.start())
            ..usize::from(duplicate_help.primary_range.end())],
        "[help]"
    );
    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "semantic.group-result-metadata")
    );
}

#[test]
fn blank_line_detaches_metadata() {
    let compiled = compile_document("!version 0.4\n[help] Detached\n\nbuild():\n    true\n");

    assert!(compiled.document.tasks[0].metadata.help.is_none());
}
