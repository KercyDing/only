use only_semantic::compile_document;

#[test]
fn lowers_directives_with_crlf_line_endings() {
    let compiled = compile_document("!shell bash\r\nbuild():\r\n    echo ok\r\n");

    assert!(compiled.diagnostics.is_empty());
    assert!(matches!(
        compiled.document.directives[0],
        only_semantic::DirectiveAst::Shell { .. }
    ));
}

#[test]
fn lowers_task_header_and_commands_into_ast() {
    let compiled = compile_document("build(tag=\"v1\"):\n    echo {{tag}}\n");
    let task = &compiled.document.tasks[0];

    assert_eq!(task.name, "build");
    assert_eq!(task.params[0].name, "tag");
    assert!(!task.params[0].is_slice);
    assert_eq!(task.params[0].default_value.as_deref(), Some("v1"));
    assert_eq!(task.commands.len(), 1);
    assert_eq!(task.commands[0].text, "echo {{tag}}");
    assert_eq!(task.commands[0].interpolations[0].name, "tag");
}

#[test]
fn lowers_slice_parameter_suffix() {
    let compiled = compile_document("run(args..):\n    echo {{args}}\n");
    let task = &compiled.document.tasks[0];

    assert_eq!(task.params[0].name, "args");
    assert!(task.params[0].is_slice);
    assert_eq!(task.params[0].default_value, None);
    assert_eq!(task.signature(), "run(args..)");
}

#[test]
fn skips_escaped_interpolation_markers() {
    let compiled = compile_document("build(tag=\"v1\"):\n    echo \\{{tag\\}} {{tag}}\n");
    let task = &compiled.document.tasks[0];

    assert_eq!(task.commands.len(), 1);
    assert_eq!(task.commands[0].interpolations.len(), 1);
    assert_eq!(task.commands[0].interpolations[0].name, "tag");
}

#[test]
fn keeps_even_backslashes_before_real_interpolation() {
    let compiled = compile_document("build(tag=\"v1\"):\n    echo \\\\{{tag}}\n");
    let task = &compiled.document.tasks[0];

    assert_eq!(task.commands.len(), 1);
    assert_eq!(task.commands[0].interpolations.len(), 1);
    assert_eq!(task.commands[0].interpolations[0].name, "tag");
}
