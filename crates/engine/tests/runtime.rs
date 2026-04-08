use only_engine::{Invocation, build_execution_plan, run_plan};
use only_semantic::compile_document;
use std::process::ExitCode;

#[test]
fn runs_plan_with_default_parameter_interpolation() {
    let compiled = compile_document("hello(name=\"true\"):\n    {{name}}\n");
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "hello",
            args: vec![],
            overrides: vec![],
        },
    );

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn reports_command_failure_with_context() {
    let compiled = compile_document("fail():\n    false\n");
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "fail",
            args: vec![],
            overrides: vec![],
        },
    );

    let error = run_plan(&plan).expect_err("runtime should fail");
    let rendered = error.to_string();
    assert!(rendered.contains("task 'fail' failed at step [1/1]"));
    assert!(rendered.contains("while running `false`"));
}

#[cfg(unix)]
#[test]
fn runs_plan_with_explicit_sh_shell() {
    let compiled = compile_document("!shell sh\nhello():\n    true\n");
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "hello",
            args: vec![],
            overrides: vec![],
        },
    );

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}
