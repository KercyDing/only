use only_semantic::compile_document;

#[test]
fn lowers_task_header_and_commands_into_ast() {
    let compiled = compile_document("build(tag=\"v1\"):\n    echo {{tag}}\n");
    let task = &compiled.document.tasks[0];

    assert_eq!(task.name, "build");
    assert_eq!(task.params[0].name, "tag");
    assert_eq!(task.params[0].default_value.as_deref(), Some("v1"));
    assert_eq!(task.commands.len(), 1);
    assert_eq!(task.commands[0].text, "echo {{tag}}");
    assert_eq!(task.commands[0].interpolations[0].name, "tag");
}
