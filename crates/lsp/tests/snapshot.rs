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
fn keeps_doc_comment_hover_separate_from_following_task() {
    let source =
        "// section header\n\n# macOS-only task.\nbuild-macos(target=\"debug\"):\n    echo ok\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(source.find("macOS-only").expect("doc text should exist") as u32);

    let hover = hover(&snapshot, offset).expect("hover should exist");

    assert_eq!(hover.kind, LspHoverKind::DocComment);
    assert_eq!(hover.docs.as_deref(), Some("macOS-only task."));
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
    let source = "!version 0.3\n!var profile = \"release\"\n";
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
fn returns_shell_operator_hover() {
    let source = "!version 0.3\nbuild() shell~=pwsh:\n    echo ok\n";
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
fn hides_invalid_fallback_hover() {
    let source = "!version 0.3\nbuild() shell~=sh:\n    echo ok\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(source.find("shell~=").expect("operator should exist") as u32);

    assert!(hover(&snapshot, offset).is_none());
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
}

#[test]
fn returns_command_block_hover_with_shell() {
    let source = "!version 0.2\n!shell bash\nbuild():\n    | echo ok\n    | echo done\n";
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let offset = TextSize::from(source.find("| echo ok").expect("block should exist") as u32);

    let info = hover(&snapshot, offset).expect("block hover should exist");

    assert_eq!(info.kind, LspHoverKind::CommandBlock);
    assert_eq!(info.signature, "block (bash)");
}

#[test]
fn returns_dependency_hover_for_serial_chain_entries() {
    let source = concat!(
        "# Formatting task.\n",
        "fmt():\n",
        "    cargo fmt\n",
        "# CI wrapper.\n",
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
    assert!(hover(&snapshot, amp_offset).is_none());
}

#[test]
fn resolves_local_namespace_dependency_hover() {
    let source = concat!(
        "[dev]\n",
        "# Build assets.\n",
        "build():\n",
        "    cargo build\n",
        "ci() & build:\n",
        "    echo done\n",
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
fn returns_braced_namespace_hover() {
    let source = concat!(
        "!version 0.3\n",
        "# Development tasks.\n",
        "[dev] {\n",
        "    run():\n",
        "        true\n",
        "}\n",
    );
    let snapshot = DocumentSnapshot::new("file:///workspace/Onlyfile", 1, source);
    let open_offset = TextSize::from(source.find("dev").expect("namespace should exist") as u32);
    let close_offset = TextSize::from(source.rfind('}').expect("close brace should exist") as u32);

    let open = hover(&snapshot, open_offset).expect("namespace hover should exist");
    let close = hover(&snapshot, close_offset).expect("close hover should exist");

    assert_eq!(open.kind, LspHoverKind::Namespace);
    assert_eq!(open.name, "dev");
    assert_eq!(open.signature, "namespace [dev] {");
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
