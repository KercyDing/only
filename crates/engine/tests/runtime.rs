use only_engine::{Invocation, build_execution_plan, run_plan};
#[cfg(unix)]
use only_engine::{RuntimeOptions, run_plan_with_options};
use only_semantic::compile_document;
#[cfg(unix)]
use std::fs;
use std::process::ExitCode;
#[cfg(unix)]
use std::time::{SystemTime, UNIX_EPOCH};

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
    assert!(rendered.contains("due to "));
    assert!(!rendered.contains("ExitCode("));
    assert!(!rendered.contains("command:"));
}

#[cfg(unix)]
#[test]
fn releases_ready_successors() {
    let directory = std::env::temp_dir().join(format!(
        "only-runtime-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&directory).expect("temporary directory should be created");
    let compiled = compile_document(
        "!version 0.4
f1() shell=sh:
    touch f1
f2() shell=sh:
    test -f f1
    touch f2
front() & f1 & f2:
    true
b1() shell=sh:
    sleep 0.2
    test -f f2
back() & b1:
    true
root() & (front, back):
    true
",
    );
    let plan = only_engine::try_build_execution_plan_in_dir(
        &compiled.document,
        Invocation::Task {
            target: "root",
            args: vec![],
            overrides: vec![],
        },
        directory.clone(),
    )
    .expect("plan should build");

    let code = run_plan(&plan).expect("ready successor should run before unrelated branch ends");
    assert_eq!(code, ExitCode::SUCCESS);
    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}

#[cfg(unix)]
#[test]
fn stops_scheduling_after_failure() {
    let directory = std::env::temp_dir().join(format!(
        "only-runtime-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&directory).expect("temporary directory should be created");
    let compiled = compile_document(
        "!version 0.4
fail() shell=sh:
    false
later() shell=sh:
    touch later
root() & (fail, later):
    true
",
    );
    let plan = only_engine::try_build_execution_plan_in_dir(
        &compiled.document,
        Invocation::Task {
            target: "root",
            args: vec![],
            overrides: vec![],
        },
        directory.clone(),
    )
    .expect("plan should build");

    let error = run_plan_with_options(
        &plan,
        RuntimeOptions {
            quiet: true,
            max_parallelism: Some(1),
        },
    )
    .expect_err("failure should stop further scheduling");
    assert!(error.to_string().contains("task 'fail' failed"));
    assert!(!directory.join("later").exists());
    fs::remove_dir_all(directory).expect("temporary directory should be removed");
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

#[cfg(unix)]
#[test]
fn keeps_state_inside_command_block() {
    let compiled = compile_document(
        "!version 0.4\n!shell sh\nstate(value=\"inside\"):\n    | value={{value}}\n    | test \"$value\" = inside\n    test -z \"$value\"\n",
    );
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "state",
            args: vec![],
            overrides: vec![],
        },
    );

    let code = run_plan(&plan).expect("block state should stay inside one shell process");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[cfg(unix)]
#[test]
fn uses_shell_exit_status_for_whole_block() {
    let compiled = compile_document("!version 0.4\n!shell sh\nstate():\n    | false\n    | true\n");
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "state",
            args: vec![],
            overrides: vec![],
        },
    );

    let code = run_plan(&plan).expect("only the final shell status should decide the result");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[cfg(unix)]
#[test]
fn reports_command_block_failure() {
    let compiled = compile_document("!version 0.4\n!shell sh\nfail():\n    | exit 7\n    true\n");
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "fail",
            args: vec![],
            overrides: vec![],
        },
    );

    let error = run_plan(&plan).expect_err("block should fail");
    let rendered = error.to_string();
    assert!(rendered.contains("step [1/2]"));
    assert!(rendered.contains("due to "));
    assert!(!rendered.contains("ExitCode("));
}

#[cfg(unix)]
#[test]
fn runs_bash_control_structure() {
    let compiled = compile_document(
        "!version 0.4\ncount() shell=bash:\n    | total=0\n    | for value in 1 2 3; do\n    |     total=$((total + value))\n    | done\n    | test \"$total\" = 6\n",
    );
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "count",
            args: vec![],
            overrides: vec![],
        },
    );

    let code = run_plan(&plan).expect("bash block should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[cfg(unix)]
#[test]
fn handles_signal_exit_from_block() {
    let compiled =
        compile_document("!version 0.4\nsignal() shell=sh:\n    | kill -INT $$\n    true\n");
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "signal",
            args: vec![],
            overrides: vec![],
        },
    );

    let error = run_plan(&plan).expect_err("signal should fail the block");
    let rendered = error.to_string();
    assert!(rendered.contains("task 'signal' failed at step [1/2]"));
    assert!(rendered.contains("due to "));
    assert!(!rendered.contains("ExitCode("));
}

#[cfg(unix)]
#[test]
fn runs_blocks_in_parallel_stage() {
    let compiled = compile_document(
        "!version 0.4\na() shell=sh:\n    | value=a\n    | test \"$value\" = a\nb() shell=sh:\n    | value=b\n    | test \"$value\" = b\nci() & (a, b):\n    true\n",
    );
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "ci",
            args: vec![],
            overrides: vec![],
        },
    );

    let code = run_plan(&plan).expect("parallel blocks should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn reports_block_shell_start_failure() {
    let compiled =
        compile_document("!version 0.4\nfail() shell=missing-shell:\n    | echo never\n");
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "fail",
            args: vec![],
            overrides: vec![],
        },
    );

    let error = run_plan(&plan).expect_err("unsupported shell should fail");
    let rendered = error.to_string();
    assert!(rendered.contains("could not start command block with shell 'missing-shell'"));
    assert!(rendered.contains("shell 'missing-shell' is not supported"));
}

#[cfg(windows)]
#[test]
fn keeps_powershell_state_inside_block() {
    let compiled = compile_document(
        "!version 0.4\nstate() shell=powershell:\n    | $value = \"inside\"\n    | if ($value -ne \"inside\") { exit 1 }\n    | exit 0\n    if ($null -ne $value) { exit 1 }\n",
    );
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "state",
            args: vec![],
            overrides: vec![],
        },
    );

    let code = run_plan(&plan).expect("PowerShell block should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}
