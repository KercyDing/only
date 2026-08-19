use only_semantic::{GuardKind, compile_document};

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
fn lowers_dependency_arguments() {
    let compiled = compile_document(concat!(
        "build(profile):\n    true\n",
        "ci() & build(\"dev\"):\n    true\n",
    ));
    let dependency = &compiled.document.tasks[1].dependencies[0];

    assert_eq!(dependency.name, "build");
    assert_eq!(dependency.arguments.len(), 1);
    assert_eq!(dependency.arguments[0].value, "dev");
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
    let source = "!version 0.4\ngreet(name):\n    | if [ -n \"{{name}}\" ]; then\n    |     echo \"{{name}}\"\n    | fi\n";
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
fn lowers_multiple_guards_in_order() {
    let compiled =
        compile_document("!version 0.4\ntest() ? @has(\"cargo\") ? @env(\"CI\"):\n    true\n");
    let guards = &compiled.document.tasks[0].guards;

    assert_eq!(guards.len(), 2);
    assert_eq!(guards[0].kind, GuardKind::Has);
    assert_eq!(guards[1].kind, GuardKind::Env);
    assert!(guards[0].range.start() < guards[1].range.start());
}

#[test]
fn lowers_multiline_parameter_list() {
    let compiled = compile_document(
        "!version 0.4\ndeploy(\n    env = \"production\",\n    region = \"global,primary\",\n):\n    true\n",
    );
    let params = &compiled.document.tasks[0].params;

    assert!(
        compiled.diagnostics.is_empty(),
        "{:?}",
        compiled.diagnostics
    );
    assert_eq!(params.len(), 2);
    assert_eq!(params[0].default_value.as_deref(), Some("production"));
    assert_eq!(params[1].default_value.as_deref(), Some("global,primary"));
}

#[test]
fn normalizes_help_fields_and_keeps_block_comments_in_commands() {
    let compiled = compile_document(
        "!version 0.4\n!var dist_dir = \"dist\"\n[help] Deploy artifacts\n[desc] Write to {{dist_dir}}\n[pass] Deploy complete\n[fail] Deploy failed\ndeploy():\n    | # shell comment\n    | echo {{dist_dir}}\n",
    );
    let task = &compiled.document.tasks[0];

    assert!(
        compiled.diagnostics.is_empty(),
        "{:?}",
        compiled.diagnostics
    );
    assert_eq!(task.doc.as_deref(), Some("Deploy artifacts"));
    assert_eq!(task.metadata.desc.as_deref(), Some("Write to {{dist_dir}}"));
    assert_eq!(task.metadata.pass.as_deref(), Some("Deploy complete"));
    assert_eq!(task.metadata.fail.as_deref(), Some("Deploy failed"));
    assert!(task.steps[0].source().contains("# shell comment"));
}

#[test]
fn inherits_variant_metadata() {
    let compiled = compile_document(
        "!version 0.4\n[help] Run tests\n[desc] Use the preferred runner.\n[desc] Keep output ordered.\n[pass] Tests passed.\n[pass] Reports are ready.\n[fail] Tests failed.\ntest() ? @os(\"not-a-real-os\"):\n    true\n\n[desc] Use Cargo.\n[desc] Run every target.\n[pass] Cargo tests passed.\ntest() ? @arch(\"not-a-real-arch\"):\n    true\n\n[fail] Fallback failed.\ntest():\n    true\n",
    );
    let cargo_variant = &compiled.document.tasks[1];
    let fallback = &compiled.document.tasks[2];

    assert!(
        compiled.diagnostics.is_empty(),
        "{:?}",
        compiled.diagnostics
    );
    assert_eq!(cargo_variant.metadata.help.as_deref(), Some("Run tests"));
    assert_eq!(
        cargo_variant.metadata.desc.as_deref(),
        Some("Use Cargo.\nRun every target.")
    );
    assert_eq!(
        cargo_variant.metadata.pass.as_deref(),
        Some("Cargo tests passed.")
    );
    assert_eq!(
        cargo_variant.metadata.fail.as_deref(),
        Some("Tests failed.")
    );

    assert_eq!(fallback.metadata.help.as_deref(), Some("Run tests"));
    assert_eq!(fallback.doc.as_deref(), Some("Run tests"));
    assert_eq!(
        fallback.metadata.desc.as_deref(),
        Some("Use the preferred runner.\nKeep output ordered.")
    );
    assert_eq!(
        fallback.metadata.pass.as_deref(),
        Some("Tests passed.\nReports are ready.")
    );
    assert_eq!(fallback.metadata.fail.as_deref(), Some("Fallback failed."));
}

#[test]
fn plain_comments_are_not_task_help() {
    let compiled =
        compile_document("# Run checks.\n# Use the local toolchain.\ncheck():\n    true\n");
    let task = &compiled.document.tasks[0];

    assert_eq!(task.doc, None);
    assert_eq!(task.metadata.help, None);
}

#[test]
fn rejects_unknown_metadata_fields() {
    let compiled = compile_document("[summary] Build everything\nbuild():\n    true\n");
    let task = &compiled.document.tasks[0];

    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "semantic.unknown-metadata-field")
    );
    assert_eq!(task.doc, None);
    assert_eq!(task.metadata.unknown_fields.len(), 1);
    assert_eq!(task.metadata.unknown_fields[0], "summary");
}
