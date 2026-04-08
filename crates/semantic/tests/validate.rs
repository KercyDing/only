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
