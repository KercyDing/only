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
            .any(|message| message.contains("undefined dependency 'build'"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("undefined variable 'target'"))
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

    assert!(messages.contains(&"duplicate directive '!shell'"));
}

#[test]
fn reports_slice_parameter_before_final_position() {
    let compiled = compile_document("run(args.., tail):\n    echo {{args}} {{tail}}\n");
    let messages: Vec<_> = compiled
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect();

    assert!(
        messages.contains(&"slice parameter 'args..' must be the final parameter in task 'run'")
    );
}

#[test]
fn reports_slice_parameter_default_value() {
    let compiled = compile_document("run(args..=\"fetch\"):\n    echo {{args}}\n");
    let messages: Vec<_> = compiled
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect();

    assert!(messages.contains(&"slice parameter 'args..' cannot have a default value"));
}
