use only_lsp::{DocumentSnapshot, LspHoverKind, hover};
use text_size::TextSize;

#[test]
fn reparses_from_memory_snapshot() {
    let snapshot = DocumentSnapshot::new(
        "file:///workspace/Onlyfile",
        7,
        "build(name=\"dev\"):\n    echo {{name}}\n",
    );

    assert_eq!(snapshot.uri, "file:///workspace/Onlyfile");
    assert_eq!(snapshot.version, 7);
    assert_eq!(snapshot.syntax.tokens[0].text.as_str(), "build");
    assert!(snapshot.semantic.diagnostics.is_empty());
    assert_eq!(snapshot.semantic.document.tasks[0].name, "build");
}

#[test]
fn keeps_uri_when_reparsing_new_version() {
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 7, "build():\n    true\n");
    let reparsed = snapshot.reparse(8, "deploy() & build:\n    true\n");

    assert_eq!(reparsed.uri, "file:///workspace/Onlyfile");
    assert_eq!(reparsed.version, 8);
    assert!(reparsed.semantic.diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("depends on missing task 'build'")
    }));
}

#[test]
fn ordinary_comment_has_no_hover() {
    let source =
        "// section header\n\n# macOS-only task.\nbuild-macos(target=\"debug\"):\n    echo ok\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(source.find("macOS-only").expect("doc text should exist") as u32);

    assert!(hover(&snapshot, offset).is_none());
}

#[test]
fn returns_metadata_hover() {
    let source = "!version 0.4\n[help] Build the project\nbuild():\n    true\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(source.find("help").expect("metadata field should exist") as u32);

    let hover = hover(&snapshot, offset).expect("metadata hover should exist");

    assert_eq!(hover.kind, LspHoverKind::Metadata);
    assert_eq!(hover.signature, "[help]");
    assert!(
        hover
            .docs
            .as_deref()
            .unwrap_or_default()
            .contains("Build the project")
    );
}

#[test]
fn returns_directive_hover_for_keyword_only() {
    let source = "!shell deno\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(source.find("shell").expect("directive should exist") as u32);
    let value_offset =
        TextSize::from(source.find("deno").expect("directive value should exist") as u32);

    let info = hover(&snapshot, offset).expect("hover should exist");

    assert_eq!(info.kind, LspHoverKind::Directive);
    assert_eq!(info.signature, "!shell");
    assert!(
        info.docs
            .as_deref()
            .is_some_and(|docs| docs.contains("Current value: `deno`"))
    );
    assert!(hover(&snapshot, value_offset).is_none());
}

#[test]
fn describes_global_variable() {
    let source = "!version 0.4\n!var profile = \"release\"\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(source.find("var").expect("directive should exist") as u32);

    let info = hover(&snapshot, offset).expect("hover should exist");

    assert_eq!(info.signature, "!var");
    assert_eq!(
        info.docs.as_deref(),
        Some("Defines a global string value.\n\nCurrent value: `profile = \"release\"`")
    );
}

#[test]
fn returns_guard_probe_hover() {
    let source = "build() ? @os(\"macos\"):\n    echo ok\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(source.find("@os").expect("probe should exist") as u32);
    let argument_offset =
        TextSize::from(source.find("macos").expect("probe argument should exist") as u32);

    let info = hover(&snapshot, offset).expect("hover should exist");

    assert_eq!(info.kind, LspHoverKind::GuardProbe);
    assert_eq!(info.signature, "@os(\"macos\")");
    assert!(
        info.docs
            .as_deref()
            .is_some_and(|docs| docs.contains("Current argument: `macos`"))
    );
    assert!(hover(&snapshot, argument_offset).is_none());
}

#[test]
fn describes_condition_operator() {
    let source = "build() ? @os(\"macos\"):\n    echo ok\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(source.find('?').expect("condition should exist") as u32);

    let info = hover(&snapshot, offset).expect("condition hover should exist");

    assert_eq!(info.kind, LspHoverKind::ConditionOperator);
    assert_eq!(info.signature, "?");
    assert_eq!(
        info.docs.as_deref(),
        Some("Runs the task only when the guard returns true.")
    );
}

#[test]
fn returns_shell_operator_hover() {
    let source = "!version 0.4\nbuild() shell~=pwsh:\n    echo ok\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let operator_offset =
        TextSize::from(source.find("shell~=").expect("shell operator should exist") as u32);
    let value_offset = TextSize::from(source.find("pwsh").expect("shell should exist") as u32);

    let operator = hover(&snapshot, operator_offset).expect("operator hover should exist");
    let value = hover(&snapshot, value_offset).expect("shell hover should exist");

    assert_eq!(operator.kind, LspHoverKind::ShellOperator);
    assert_eq!(operator.signature, "shell~=pwsh");
    assert_eq!(operator.range, value.range);
    assert_eq!(
        operator.docs.as_deref(),
        Some("Prefers pwsh and falls back to powershell when unavailable.")
    );
}

#[test]
fn describes_required_shell() {
    let source = "build() shell=bash:\n    echo ok\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(source.find("bash").expect("shell should exist") as u32);

    let info = hover(&snapshot, offset).expect("shell hover should exist");

    assert_eq!(info.signature, "shell=bash");
    assert_eq!(
        info.docs.as_deref(),
        Some("Uses bash. The task fails if it is unavailable.")
    );
}

#[test]
fn describes_invalid_fallback() {
    let source = "!version 0.4\nbuild() shell~=sh:\n    echo ok\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(source.find("shell~=").expect("operator should exist") as u32);

    let info = hover(&snapshot, offset).expect("shell hover should exist");

    assert_eq!(info.signature, "shell~=sh");
    assert_eq!(
        info.docs.as_deref(),
        Some("sh has no fallback. Use `shell=sh`.")
    );
}

#[test]
fn returns_interpolation_hover() {
    let source = "build(name=\"dev\"):\n    echo {{name}}\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset =
        TextSize::from(source.find("{{name}}").expect("interpolation should exist") as u32);

    let hover = hover(&snapshot, offset).expect("hover should exist");

    assert_eq!(hover.kind, LspHoverKind::Interpolation);
    assert_eq!(hover.signature, "{{name}}");
    assert_eq!(
        hover.docs.as_deref(),
        Some("Replaces this variable at runtime.\n\nDefault: `dev`")
    );
}

#[test]
fn shows_global_interpolation_default() {
    let source =
        "!var cargo_flags = \"--all-targets\"\nbuild():\n    cargo check {{cargo_flags}}\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(
        source
            .find("{{cargo_flags}}")
            .expect("interpolation should exist") as u32,
    );

    let hover = hover(&snapshot, offset).expect("hover should exist");

    assert_eq!(hover.kind, LspHoverKind::Interpolation);
    assert_eq!(
        hover.docs.as_deref(),
        Some("Replaces this variable at runtime.\n\nDefault: `--all-targets`")
    );
}

#[test]
fn returns_command_block_hover_with_shell() {
    let source = "!version 0.4\n!shell bash\nbuild():\n    | echo ok\n    | echo done\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(source.find("| echo ok").expect("block should exist") as u32);

    let info = hover(&snapshot, offset).expect("block hover should exist");

    assert_eq!(info.kind, LspHoverKind::CommandBlock);
    assert_eq!(info.signature, "block (bash)");
}

#[test]
fn returns_dependency_hover_for_serial_chain_entries() {
    let source = concat!(
        "!version 0.4\n",
        "[help] Formatting task.\n",
        "fmt():\n",
        "    cargo fmt\n",
        "[help] CI wrapper.\n",
        "ci() & fmt:\n",
        "    echo done\n",
    );
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let dependency_offset =
        TextSize::from(source.rfind("fmt").expect("dependency should exist") as u32);
    let amp_offset = TextSize::from(source.rfind('&').expect("ampersand should exist") as u32);

    let info = hover(&snapshot, dependency_offset).expect("hover should exist");

    assert_eq!(info.kind, LspHoverKind::Dependency);
    assert_eq!(info.name, "fmt");
    assert_eq!(info.signature, "fmt()");
    assert_eq!(info.docs.as_deref(), Some("Formatting task."));
    let operator = hover(&snapshot, amp_offset).expect("dependency operator hover should exist");
    assert_eq!(operator.kind, LspHoverKind::DependencyOperator);
    assert_eq!(operator.signature, "&");
    assert_eq!(
        operator.docs.as_deref(),
        Some("Runs the dependency before this task.")
    );
}

#[test]
fn returns_dependency_hover_for_invocation() {
    let source = concat!(
        "[help] Build the project.\n",
        "build(profile):\n    true\n",
        "ci() & build(\"dev\"):\n    true\n",
    );
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let dependency = source.rfind("build").expect("dependency should exist");

    let info =
        hover(&snapshot, TextSize::from(dependency as u32)).expect("dependency hover should exist");

    assert_eq!(info.kind, LspHoverKind::Dependency);
    assert_eq!(info.signature, "build(profile)");
    assert_eq!(info.docs.as_deref(), Some("Build the project."));
    assert_eq!(
        &source[usize::from(info.range.start())..usize::from(info.range.end())],
        "build"
    );
}

#[test]
fn describes_parallel_dependency_group() {
    let source = concat!(
        "check():\n    true\n",
        "test():\n    true\n",
        "ci() & (check, test):\n    true\n",
    );
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let group_start = source.rfind("(check").expect("parallel group should exist");
    let comma = source.rfind(',').expect("parallel separator should exist");
    let check = source
        .rfind("check")
        .expect("check dependency should exist");
    let test = source.rfind("test").expect("test dependency should exist");

    for offset in [group_start, comma] {
        let info = hover(&snapshot, TextSize::from(offset as u32))
            .expect("parallel group hover should exist");
        assert_eq!(info.kind, LspHoverKind::ParallelGroup);
        assert_eq!(info.signature, "(check, test)");
        assert_eq!(
            info.docs.as_deref(),
            Some("Runs these dependencies in parallel.")
        );
    }

    for offset in [check, test] {
        let info = hover(&snapshot, TextSize::from(offset as u32))
            .expect("dependency hover should remain available");
        assert_eq!(info.kind, LspHoverKind::Dependency);
    }
}

#[test]
fn resolves_local_group_dependency_hover() {
    let source = concat!(
        "!version 0.4\n",
        "group dev {\n",
        "[help] Build assets.\n",
        "build():\n",
        "    cargo build\n",
        "ci() & build:\n",
        "    echo done\n",
        "}\n",
    );
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let dependency_offset =
        TextSize::from(source.rfind("build").expect("dependency should exist") as u32);

    let info = hover(&snapshot, dependency_offset).expect("hover should exist");

    assert_eq!(info.kind, LspHoverKind::Dependency);
    assert_eq!(info.name, "build");
    assert_eq!(info.signature, "build()");
    assert_eq!(info.container_name.as_deref(), Some("dev"));
    assert_eq!(info.docs.as_deref(), Some("Build assets."));
}

#[test]
fn returns_group_hover() {
    let source = concat!(
        "!version 0.4\n",
        "[help] Development tasks.\n",
        "group dev {\n",
        "    run():\n",
        "        true\n",
        "}\n",
    );
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let open_offset = TextSize::from(source.find("dev").expect("group should exist") as u32);
    let close_offset = TextSize::from(source.rfind('}').expect("close brace should exist") as u32);

    let open = hover(&snapshot, open_offset).expect("group hover should exist");
    let close = hover(&snapshot, close_offset).expect("close hover should exist");

    assert_eq!(open.kind, LspHoverKind::Namespace);
    assert_eq!(open.name, "dev");
    assert_eq!(open.signature, "group dev {");
    assert_eq!(open.docs.as_deref(), Some("Development tasks."));
    assert_eq!(close.kind, LspHoverKind::Namespace);
    assert_eq!(close.name, "dev");
    assert_eq!(close.signature, "}");
}

#[test]
fn returns_task_and_parameter_hover() {
    let source = "build(name=\"dev\", args..):\n    echo {{name}}\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let name_offset = TextSize::from(source.find("build").expect("task name should exist") as u32);
    let param_offset = TextSize::from(source.find("name=").expect("param should exist") as u32);
    let default_offset = TextSize::from(source.find("dev").expect("default should exist") as u32);
    let slice_offset = TextSize::from(source.find("args..").expect("slice should exist") as u32);

    let task_hover = hover(&snapshot, name_offset).expect("hover should exist");
    let param_hover = hover(&snapshot, param_offset).expect("parameter hover should exist");
    let slice_hover = hover(&snapshot, slice_offset).expect("slice hover should exist");

    assert_eq!(task_hover.kind, LspHoverKind::Task);
    assert_eq!(task_hover.signature, "build(name=\"dev\", args..)");
    assert_eq!(task_hover.docs.as_deref(), Some("Default to `dev`."));
    assert_eq!(task_hover.range.start(), TextSize::from(0));
    assert_eq!(task_hover.range.end(), TextSize::from(5));
    assert_eq!(param_hover.kind, LspHoverKind::Parameter);
    assert_eq!(param_hover.signature, "name=\"dev\"");
    assert_eq!(
        param_hover.docs.as_deref(),
        Some("Task parameter.\n\nDefault: `dev`")
    );
    assert_eq!(param_hover.container_name.as_deref(), Some("build"));
    assert_eq!(slice_hover.kind, LspHoverKind::Parameter);
    assert_eq!(slice_hover.signature, "args..");
    assert!(
        slice_hover
            .docs
            .as_deref()
            .is_some_and(|docs| docs.contains("remaining positional arguments"))
    );
    assert!(hover(&snapshot, default_offset).is_none());
}

#[test]
fn shows_defaults_before_task_help() {
    let source = concat!(
        "[help] Build only-cli\n",
        "build(profile=\"debug\"):\n",
        "    cargo build\n",
    );
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(source.find("build").expect("task should exist") as u32);

    let info = hover(&snapshot, offset).expect("task hover should exist");

    assert_eq!(
        info.docs.as_deref(),
        Some("Default to `debug`.\n\nBuild only-cli")
    );
}
