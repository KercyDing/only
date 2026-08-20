use only_syntax::{ShellKind, ShellOperator, TaskStepNode, snapshot};

#[test]
fn exposes_typed_top_level_nodes() {
    let parsed = only_syntax::parse(
        "!shell deno\n[help] Developer tasks.\ngroup dev {\n    serve(port=\"3000\"):\n        echo {{port}}\n}\n",
    );
    let document = parsed.document();

    let directive = document
        .directives()
        .next()
        .expect("directive should exist");
    assert_eq!(directive.name().as_deref(), Some("shell"));
    assert_eq!(directive.value().as_deref(), Some("deno"));

    let doc = document.metadata().next().expect("metadata should exist");
    assert_eq!(doc.text().as_deref(), Some("Developer tasks."));

    let namespace = document.namespaces().next().expect("group should exist");
    assert_eq!(namespace.name().as_deref(), Some("dev"));

    let task = document.tasks().next().expect("task should exist");
    assert_eq!(task.name().as_deref(), Some("serve"));
    assert_eq!(task.header_text().as_deref(), Some("serve(port=\"3000\")"));
    assert_eq!(task.commands().collect::<Vec<_>>(), vec!["echo {{port}}"]);
    assert!(!task.range().is_empty());
}

#[test]
fn exposes_structured_doc_comment_fields() {
    let source = "[desc] Extra details\nrun():\n    true\n";
    let syntax = snapshot(source);
    let comment = syntax
        .document()
        .metadata()
        .next()
        .expect("metadata comment should exist");

    assert_eq!(
        comment
            .field()
            .as_ref()
            .map(|(name, value)| (name.as_str(), value.as_str())),
        Some(("desc", "Extra details"))
    );
    let tag = comment
        .tag_range()
        .expect("metadata tag should have a range");
    assert_eq!(
        &source[usize::from(tag.start())..usize::from(tag.end())],
        "[desc]"
    );
}

#[test]
fn snapshot_exposes_typed_document_root() {
    let syntax = snapshot("build():\n    cargo build\n");
    let document = syntax.document();
    let task = document.tasks().next().expect("task should exist");

    assert_eq!(task.name().as_deref(), Some("build"));
    assert_eq!(task.commands().collect::<Vec<_>>(), vec!["cargo build"]);
}

#[test]
fn skips_task_body_comment_lines() {
    let syntax = snapshot("check():\n    # comment\n    cargo check\n");
    let document = syntax.document();
    let task = document.tasks().next().expect("task should exist");

    assert_eq!(task.commands().collect::<Vec<_>>(), vec!["cargo check"]);
}

#[test]
fn keeps_hash_command() {
    let syntax = snapshot("clippy():\n    # Nothing here\n    cargo clippy --all-targets\n");
    let task = syntax.document().tasks().next().expect("task should exist");

    assert_eq!(
        task.commands().collect::<Vec<_>>(),
        vec!["cargo clippy --all-targets"]
    );
}

#[test]
fn keeps_full_command() {
    let syntax = snapshot(
        "!version 0.4\n!var cargo_flags = \"--all-targets\"\n\n[help] Lint code\nclippy():\n    # Nothing here\n    cargo clippy {{cargo_flags}} -- -D warnings\n",
    );
    let tasks = syntax.document().tasks().collect::<Vec<_>>();
    assert_eq!(tasks.len(), 1);
    assert_eq!(
        tasks[0].commands().collect::<Vec<_>>(),
        vec!["cargo clippy {{cargo_flags}} -- -D warnings"]
    );
}

#[test]
fn repo_clippy_command() {
    let syntax = snapshot(include_str!("../../../Onlyfile"));
    let task = syntax
        .document()
        .tasks()
        .find(|task| task.name().as_deref() == Some("clippy"))
        .expect("clippy task should exist");
    assert_eq!(
        task.commands().collect::<Vec<_>>(),
        vec![
            "cargo clippy {{cargo_flags}} -- -D warnings",
            "echo \"Done\""
        ]
    );
}

#[test]
fn top_level_doc_comment_ends_previous_task_body() {
    let syntax = snapshot(
        "# Build release artifacts.\nbuild():\n    cargo build\n# Run checks.\ncheck():\n    cargo check\n",
    );
    let tasks = syntax.document().tasks().collect::<Vec<_>>();

    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].name().as_deref(), Some("build"));
    assert_eq!(tasks[0].commands().collect::<Vec<_>>(), vec!["cargo build"]);
    assert_eq!(tasks[1].name().as_deref(), Some("check"));
    assert_eq!(tasks[1].commands().collect::<Vec<_>>(), vec!["cargo check"]);
}

#[test]
fn exposes_structured_task_header_sections() {
    let source =
        "build(tag=\"v1\") ? @env(\"CI\") & install & bootstrap shell~=bash:\n    echo {{tag}}\n";
    let syntax = snapshot(source);
    let task = syntax.document().tasks().next().expect("task should exist");
    let header = task.header_info();

    assert_eq!(header.params.as_deref(), Some("tag=\"v1\""));
    assert_eq!(header.guard.as_deref(), Some("@env(\"CI\")"));
    let guard = &header.guards[0];
    assert_eq!(
        &source[usize::from(guard.name_range.start())..usize::from(guard.name_range.end())],
        "@env"
    );
    assert_eq!(header.dependencies.as_deref(), Some("install & bootstrap"));
    let shell = header.shell.expect("shell should exist");
    assert_eq!(shell.selection.kind, ShellKind::Bash);
    assert_eq!(shell.selection.operator, ShellOperator::Fallback);

    let dependency_refs = header.dependency_refs;
    assert_eq!(dependency_refs.len(), 2);
    assert_eq!(dependency_refs[0].name.as_str(), "install");
    assert_eq!(dependency_refs[0].stage, 0);
    assert_eq!(dependency_refs[1].name.as_str(), "bootstrap");
    assert_eq!(dependency_refs[1].stage, 1);
}

#[test]
fn exposes_exact_shell_assignment_in_task_header() {
    let syntax = snapshot("probe() shell=powershell:\n    Write-Output ok\n");
    let task = syntax.document().tasks().next().expect("task should exist");
    let header = task.header_info();

    let shell = header.shell.expect("shell should exist");
    assert_eq!(shell.selection.kind, ShellKind::Powershell);
    assert_eq!(shell.selection.operator, ShellOperator::Required);
}

#[test]
fn exposes_dependency_ranges_for_hover_and_diagnostics() {
    let source = "ci() & (fmt, dev.build) & test shell~=bash:\n    echo ok\n";
    let syntax = snapshot(source);
    let task = syntax.document().tasks().next().expect("task should exist");
    let dependency_refs = task.header_info().dependency_refs;

    assert_eq!(dependency_refs.len(), 3);
    assert_eq!(dependency_refs[0].name.as_str(), "fmt");
    assert_eq!(dependency_refs[0].stage, 0);
    assert_eq!(
        &source[usize::from(dependency_refs[0].range.start())
            ..usize::from(dependency_refs[0].range.end())],
        "fmt"
    );
    assert_eq!(dependency_refs[1].name.as_str(), "dev.build");
    assert_eq!(dependency_refs[1].stage, 0);
    assert_eq!(
        &source[usize::from(dependency_refs[1].range.start())
            ..usize::from(dependency_refs[1].range.end())],
        "dev.build"
    );
    assert_eq!(dependency_refs[2].name.as_str(), "test");
    assert_eq!(dependency_refs[2].stage, 1);
}

#[test]
fn exposes_dependency_arguments_and_ranges() {
    let source = "ci() & build(\"dev\", \"fast\"):\n    true\n";
    let syntax = snapshot(source);
    let task = syntax.document().tasks().next().expect("task should exist");
    let dependency = task
        .header_info()
        .dependency_refs
        .into_iter()
        .next()
        .expect("dependency should exist");

    assert_eq!(dependency.name.as_str(), "build");
    assert_eq!(dependency.arguments.len(), 2);
    assert_eq!(dependency.arguments[0].value.as_str(), "dev");
    assert_eq!(dependency.arguments[1].value.as_str(), "fast");
    assert_eq!(
        &source[usize::from(dependency.range.start())..usize::from(dependency.range.end())],
        "build"
    );
    assert_eq!(
        &source[usize::from(dependency.invocation_range.start())
            ..usize::from(dependency.invocation_range.end())],
        "build(\"dev\", \"fast\")"
    );
}

#[test]
fn exposes_parameter_name_ranges_for_hover() {
    let source = "build(tag=\"v1\", args.., shell):\n    echo ok\n";
    let syntax = snapshot(source);
    let task = syntax.document().tasks().next().expect("task should exist");
    let param_refs = task.header_info().param_refs;

    assert_eq!(param_refs.len(), 3);
    for (reference, expected) in param_refs.iter().zip(["tag", "args", "shell"]) {
        assert_eq!(reference.name.as_str(), expected);
        assert_eq!(
            &source[usize::from(reference.range.start())..usize::from(reference.range.end())],
            expected
        );
    }
}

#[test]
fn groups_consecutive_block_lines() {
    let source = "task():\n    | if true; then\n    |     echo ok\n    | fi\n    echo after\n";
    let syntax = snapshot(source);
    let task = syntax.document().tasks().next().expect("task should exist");
    let steps = task.steps().collect::<Vec<_>>();

    assert_eq!(steps.len(), 2);
    let TaskStepNode::CommandBlock(block) = &steps[0] else {
        panic!("first step should be a block");
    };
    assert_eq!(block.source, "if true; then\n    echo ok\nfi\n");
    assert_eq!(block.line_ranges.len(), 3);
    assert_eq!(block.marker_ranges.len(), 3);
    assert_eq!(
        &source[usize::from(block.range.start())..usize::from(block.range.end())],
        "    | if true; then\n    |     echo ok\n    | fi\n"
    );
    let TaskStepNode::Command(command) = &steps[1] else {
        panic!("second step should be a command");
    };
    assert_eq!(command.text, "echo after");
}

#[test]
fn recognizes_only_delimited_block_markers() {
    let syntax = snapshot(
        "task():\n    | first\n    |\tsecond\n    |\n    |  indented\n    |tee output\n    || fallback\n",
    );
    let task = syntax.document().tasks().next().expect("task should exist");
    let steps = task.steps().collect::<Vec<_>>();

    assert_eq!(steps.len(), 3);
    let TaskStepNode::CommandBlock(block) = &steps[0] else {
        panic!("first step should be a block");
    };
    assert_eq!(block.source, "first\nsecond\n\n indented\n");
    assert!(matches!(&steps[1], TaskStepNode::Command(command) if command.text == "|tee output"));
    assert!(matches!(&steps[2], TaskStepNode::Command(command) if command.text == "|| fallback"));
}

#[test]
fn blank_lines_and_comments_split_blocks() {
    let syntax = snapshot("task():\n    | first\n\n    | second\n    # separator\n    | third\n");
    let task = syntax.document().tasks().next().expect("task should exist");
    let steps = task.steps().collect::<Vec<_>>();

    assert_eq!(steps.len(), 3);
    assert!(
        steps
            .iter()
            .all(|step| matches!(step, TaskStepNode::CommandBlock(_)))
    );
}

#[test]
fn normalizes_crlf_block_newlines() {
    let syntax = snapshot("task():\r\n    | echo one\r\n    | echo two\r\n");
    let task = syntax.document().tasks().next().expect("task should exist");
    let steps = task.steps().collect::<Vec<_>>();
    let TaskStepNode::CommandBlock(block) = &steps[0] else {
        panic!("step should be a block");
    };

    assert_eq!(block.source, "echo one\necho two\n");
    assert_eq!(block.line_ranges.len(), 2);
}

#[test]
fn exposes_structured_multiline_header_nodes() {
    let source = "release(\n    channel = \"nightly\",\n    target = \"x86_64,static\",\n)\n    ? @has(\"cargo\")\n    ? @env(\"CI\")\n    & build\n    & (sign, package)\n    shell=bash\n:\n    echo done\n";
    let syntax = snapshot(source);
    assert!(
        syntax.diagnostics().is_empty(),
        "{:?}",
        syntax.diagnostics()
    );
    let task = syntax.document().tasks().next().expect("task should exist");
    let header = task.header().expect("header should exist");

    let parameters = header
        .parameter_list()
        .expect("parameters should exist")
        .parameters()
        .collect::<Vec<_>>();
    assert_eq!(parameters.len(), 2);
    assert_eq!(parameters[0].name().as_deref(), Some("channel"));
    assert_eq!(
        parameters[1].default_value().as_deref(),
        Some("x86_64,static")
    );
    assert_eq!(header.conditions().count(), 2);
    assert_eq!(header.dependencies().count(), 2);
    assert_eq!(
        header.shell().expect("shell should exist").text(),
        "shell=bash"
    );
    assert!(header.terminator().is_some());
}

#[test]
fn multiline_and_single_line_headers_match() {
    let single = snapshot(
        "test(target=\"all\") ? @has(\"cargo\") ? @env(\"CI\") & build shell~=bash:\n    true\n",
    );
    let multiline = snapshot(
        "test(\n    target = \"all\",\n)\n    ? @has(\"cargo\")\n    ? @env(\"CI\")\n    & build\n    shell~=bash\n:\n    true\n",
    );

    let single_info = single
        .document()
        .tasks()
        .next()
        .expect("task")
        .header_info();
    let multiline_info = multiline
        .document()
        .tasks()
        .next()
        .expect("task")
        .header_info();
    assert_eq!(
        single_info.param_refs,
        multiline_info
            .param_refs
            .iter()
            .cloned()
            .map(|mut parameter| {
                parameter.range = single_info.param_refs[0].range;
                parameter
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        single_info
            .guards
            .iter()
            .map(|guard| (&guard.kind, &guard.argument))
            .collect::<Vec<_>>(),
        multiline_info
            .guards
            .iter()
            .map(|guard| (&guard.kind, &guard.argument))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        single_info
            .dependency_refs
            .iter()
            .map(|dependency| (&dependency.name, dependency.stage))
            .collect::<Vec<_>>(),
        multiline_info
            .dependency_refs
            .iter()
            .map(|dependency| (&dependency.name, dependency.stage))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        single_info.shell.map(|shell| shell.selection),
        multiline_info.shell.map(|shell| shell.selection)
    );
}
