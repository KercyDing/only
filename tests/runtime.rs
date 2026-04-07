use std::process::ExitCode;

use only::{CliInput, build_execution_plan, parse_onlyfile, run_plan};

#[test]
fn runs_successful_plan() {
    let document = parse_onlyfile(
        "hello():
    true
",
    )
    .expect("document should parse");

    let plan = build_execution_plan(
        &document,
        &CliInput {
            onlyfile_path: None,
            print_discovered_path: false,
            task: Some("hello".into()),
            subtask: None,
            parameter_overrides: vec![],
        },
    )
    .expect("plan should build");

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

    let plan = build_execution_plan(
        &document,
        &CliInput {
            onlyfile_path: None,
            print_discovered_path: false,
            task: Some("fail".into()),
            subtask: None,
            parameter_overrides: vec![],
        },
    )
    .expect("plan should build");

    let code = run_plan(&plan).expect("runtime should return exit code");
    assert_ne!(code, ExitCode::SUCCESS);
}

#[test]
fn binds_default_parameter_values() {
    let document = parse_onlyfile(
        "hello(name=\"world\"):
    test \"{{name}}\" = \"world\"
",
    )
    .expect("document should parse");

    let plan = build_execution_plan(
        &document,
        &CliInput {
            onlyfile_path: None,
            print_discovered_path: false,
            task: Some("hello".into()),
            subtask: None,
            parameter_overrides: vec![],
        },
    )
    .expect("plan should build");

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn applies_cli_parameter_overrides() {
    let document = parse_onlyfile(
        "hello(name=\"world\"):
    test \"{{name}}\" = \"alice\"
",
    )
    .expect("document should parse");

    let plan = build_execution_plan(
        &document,
        &CliInput {
            onlyfile_path: None,
            print_discovered_path: false,
            task: Some("hello".into()),
            subtask: None,
            parameter_overrides: vec![("name".into(), "alice".into())],
        },
    )
    .expect("plan should build");

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

    let error = build_execution_plan(
        &document,
        &CliInput {
            onlyfile_path: None,
            print_discovered_path: false,
            task: Some("hello".into()),
            subtask: None,
            parameter_overrides: vec![],
        },
    )
    .expect_err("missing parameter should fail planning");

    assert_eq!(error.to_string(), "missing required parameter '{{name}}'");
}
