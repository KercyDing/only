use std::process::ExitCode;

use only::{CliInput, build_execution_plan, parse_onlyfile, run_plan};

fn cli(task_path: &[&str]) -> CliInput {
    CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        top_level_help_requested: false,
        task_path: task_path.iter().map(|s| s.to_string()).collect(),
        parameter_overrides: vec![],
    }
}

#[test]
fn runs_successful_plan() {
    let document = parse_onlyfile(
        "hello():
    true
",
    )
    .expect("document should parse");

    let plan = build_execution_plan(&document, &cli(&["hello"])).expect("plan should build");

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn propagates_command_failure() {
    let document = parse_onlyfile(
        "fail():
    false
",
    )
    .expect("document should parse");

    let plan = build_execution_plan(&document, &cli(&["fail"])).expect("plan should build");

    let error = run_plan(&plan).expect_err("runtime should return contextual error");
    assert_eq!(
        error.to_string(),
        "task 'fail' failed while running `false` with exit code ExitCode(unix_exit_status(1))"
    );
}

#[test]
fn binds_default_parameter_values() {
    let document = parse_onlyfile(
        r#"hello(name="world"):
    test "{{name}}" = "world"
"#,
    )
    .expect("document should parse");

    let plan = build_execution_plan(&document, &cli(&["hello"])).expect("plan should build");

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn applies_cli_parameter_overrides() {
    let document = parse_onlyfile(
        r#"hello(name="world"):
    test "{{name}}" = "alice"
"#,
    )
    .expect("document should parse");

    let input = CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        top_level_help_requested: false,
        task_path: vec!["hello".into()],
        parameter_overrides: vec![("name".into(), "alice".into())],
    };

    let plan = build_execution_plan(&document, &input).expect("plan should build");

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn rejects_missing_required_parameter() {
    let document = parse_onlyfile(
        "hello(name):
    echo {{name}}
",
    )
    .expect("document should parse");

    let error = build_execution_plan(&document, &cli(&["hello"]))
        .expect_err("missing parameter should fail planning");

    assert_eq!(error.to_string(), "missing required parameter '{{name}}'");
}

#[test]
fn rejects_unknown_parameter_override() {
    let document = parse_onlyfile(
        r#"hello(name="world"):
    echo {{name}}
"#,
    )
    .expect("document should parse");

    let input = CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        top_level_help_requested: false,
        task_path: vec!["hello".into()],
        parameter_overrides: vec![("other".into(), "alice".into())],
    };

    let error = build_execution_plan(&document, &input)
        .expect_err("unknown parameter should fail planning");

    assert_eq!(
        error.to_string(),
        "unknown parameter 'other' for task 'hello'"
    );
}

#[test]
fn rejects_duplicate_parameter_overrides() {
    let document = parse_onlyfile(
        r#"hello(name="world"):
    echo {{name}}
"#,
    )
    .expect("document should parse");

    let input = CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        top_level_help_requested: false,
        task_path: vec!["hello".into()],
        parameter_overrides: vec![
            ("name".into(), "alice".into()),
            ("name".into(), "bob".into()),
        ],
    };

    let error = build_execution_plan(&document, &input)
        .expect_err("duplicate override should fail planning");

    assert_eq!(error.to_string(), "duplicate parameter override 'name'");
}

#[test]
fn runs_verbose_plan_successfully() {
    let document = parse_onlyfile(
        "!verbose true
hello():
    true
",
    )
    .expect("document should parse");

    let plan = build_execution_plan(&document, &cli(&["hello"])).expect("plan should build");

    let code = run_plan(&plan).expect("verbose runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn binds_positional_arguments_for_global_task() {
    let document = parse_onlyfile(
        r#"run(task):
    test "{{task}}" = "hello"
"#,
    )
    .expect("document should parse");

    let plan = build_execution_plan(&document, &cli(&["run", "hello"])).expect("plan should build");

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn binds_positional_arguments_for_namespaced_task() {
    let document = parse_onlyfile(
        "[frontend]
build(profile):
    test \"{{profile}}\" = \"prod\"
",
    )
    .expect("document should parse");

    let plan = build_execution_plan(&document, &cli(&["frontend", "build", "prod"]))
        .expect("plan should build");

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}
