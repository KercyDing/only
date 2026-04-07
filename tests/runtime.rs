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
        },
    )
    .expect("plan should build");

    let code = run_plan(&plan).expect("runtime should return exit code");
    assert_ne!(code, ExitCode::SUCCESS);
}
