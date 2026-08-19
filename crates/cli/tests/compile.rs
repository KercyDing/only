use only::{CliInput, compile_for_cli, compile_for_cli_input, parse_onlyfile};

#[test]
fn compiles_in_memory_source_without_fs() {
    let compiled = compile_for_cli("check():\n    echo ok\n");
    assert!(compiled.diagnostics.is_empty());
    assert_eq!(compiled.plan.nodes[0].name, "check");
}

#[test]
fn skips_helper_task_when_picking_default_in_memory_target() {
    let compiled = compile_for_cli("_prepare():\n    echo helper\ncheck():\n    echo ok\n");
    assert!(compiled.diagnostics.is_empty());
    assert_eq!(compiled.plan.nodes[0].name, "check");
}

#[test]
fn compiles_group_first_task_into_plan() {
    let compiled = compile_for_cli("group dev {\nserve():\n    echo ok\n}\n");
    assert!(compiled.diagnostics.is_empty());
    assert_eq!(compiled.plan.nodes[0].name, "dev.serve");
}

#[test]
fn compiles_selected_group_task_with_positional_arg() {
    let cli = CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        dry_run: false,
        dry_run_full: false,
        quiet: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        top_level_upgrade_requested: false,
        format_requested: false,
        format_check: false,
        task_path: vec!["dev".into(), "serve".into(), "true".into()],
        parameter_overrides: vec![],
    };
    let compiled = compile_for_cli_input("group dev {\nserve(flag):\n    {{flag}}\n}\n", &cli)
        .expect("semantic CLI compile should succeed");

    assert!(compiled.diagnostics.is_empty());
    assert_eq!(compiled.plan.nodes[0].name, "dev.serve");
    assert_eq!(
        compiled.plan.nodes[0].params[0].value.as_deref(),
        Some("true")
    );
}

#[test]
fn compiles_selected_global_task_with_named_override() {
    let cli = CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        dry_run: false,
        dry_run_full: false,
        quiet: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        top_level_upgrade_requested: false,
        format_requested: false,
        format_check: false,
        task_path: vec!["build".into()],
        parameter_overrides: vec![("profile".into(), "release".into())],
    };
    let compiled = compile_for_cli_input("build(profile=\"dev\"):\n    {{profile}}\n", &cli)
        .expect("semantic CLI compile should succeed");

    assert!(compiled.diagnostics.is_empty());
    assert_eq!(compiled.plan.nodes[0].name, "build");
    assert_eq!(
        compiled.plan.nodes[0].params[0].value.as_deref(),
        Some("release")
    );
}

#[test]
fn rejects_direct_invocation_of_helper_task_for_semantic_cli_compile() {
    let cli = CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        dry_run: false,
        dry_run_full: false,
        quiet: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        top_level_upgrade_requested: false,
        format_requested: false,
        format_check: false,
        task_path: vec!["_prepare".into()],
        parameter_overrides: vec![],
    };
    let error = compile_for_cli_input("_prepare():\n    echo helper\n", &cli)
        .expect_err("helper target should fail semantic CLI compile");

    assert_eq!(
        error.to_string(),
        "helper task '_prepare' cannot be run directly"
    );
}

#[test]
fn rejects_group_without_task_target_for_semantic_cli_compile() {
    let cli = CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        dry_run: false,
        dry_run_full: false,
        quiet: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        top_level_upgrade_requested: false,
        format_requested: false,
        format_check: false,
        task_path: vec!["dev".into()],
        parameter_overrides: vec![],
    };
    let error = compile_for_cli_input("group dev {\nserve():\n    echo ok\n}\n", &cli)
        .expect_err("group target should fail");

    assert_eq!(error.to_string(), "choose a task in group 'dev'");
}

#[test]
fn rejects_error_diagnostics_before_planning() {
    let cli = CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        dry_run: false,
        dry_run_full: false,
        quiet: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        top_level_upgrade_requested: false,
        format_requested: false,
        format_check: false,
        task_path: vec!["deploy".into()],
        parameter_overrides: vec![],
    };
    let error = compile_for_cli_input("deploy() & build:\n    echo deploy\n", &cli)
        .expect_err("semantic errors should stop CLI compilation");

    assert_eq!(
        error.to_string(),
        "task 'deploy' depends on missing task 'build'"
    );
}

#[test]
fn shows_only_the_first_error_stage() {
    let error = parse_onlyfile("@\n@\ndeploy() & build:\n    echo deploy\n")
        .expect_err("parse errors should hide later semantic errors");

    assert_eq!(
        error.to_string(),
        "unexpected text\nnote: this file has no `!version` line\nhelp: check the syntax; it may need a newer version of only"
    );
}
