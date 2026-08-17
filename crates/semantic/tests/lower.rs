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
    assert_eq!(task.steps.len(), 1);
    assert_eq!(task.steps[0].source(), "echo {{tag}}");
    assert_eq!(task.steps[0].interpolations()[0].name, "tag");
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

    assert_eq!(task.steps.len(), 1);
    assert_eq!(task.steps[0].interpolations().len(), 1);
    assert_eq!(task.steps[0].interpolations()[0].name, "tag");
}

#[test]
fn keeps_even_backslashes_before_real_interpolation() {
    let compiled = compile_document("build(tag=\"v1\"):\n    echo \\\\{{tag}}\n");
    let task = &compiled.document.tasks[0];

    assert_eq!(task.steps.len(), 1);
    assert_eq!(task.steps[0].interpolations().len(), 1);
    assert_eq!(task.steps[0].interpolations()[0].name, "tag");
}

#[test]
fn lowers_command_block_with_ranges_and_interpolation() {
    let source = "!version 0.2\ngreet(name):\n    | if [ -n \"{{name}}\" ]; then\n    |     echo \"{{name}}\"\n    | fi\n";
    let compiled = compile_document(source);
    let task = &compiled.document.tasks[0];

    assert!(compiled.diagnostics.is_empty());
    assert_eq!(task.steps.len(), 1);
    let only_semantic::TaskStepAst::CommandBlock(block) = &task.steps[0] else {
        panic!("step should be a command block");
    };
    assert_eq!(block.line_ranges.len(), 3);
    assert_eq!(block.interpolations.len(), 2);
    assert_eq!(block.interpolations[0].name, "name");
    assert_eq!(
        &source[usize::from(block.range.start())..usize::from(block.range.end())],
        "    | if [ -n \"{{name}}\" ]; then\n    |     echo \"{{name}}\"\n    | fi\n"
    );
}

#[test]
fn command_blocks_require_version_0_2() {
    for source in [
        "task():\n    | echo ok\n",
        "!version 0.1\ntask():\n    | echo ok\n",
    ] {
        let compiled = compile_document(source);
        assert!(
            compiled
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code.as_str() == "semantic.command-block-version" })
        );
    }
}
