use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use only::cli::app;
use only::{CliInput, ShellKind, build_execution_plan, parse_onlyfile, run_plan, run_with};

fn cli(task_path: &[&str]) -> CliInput {
    CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
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
    let rendered = error.to_string();
    assert!(rendered.contains("task 'fail' failed at step [1/1]"));
    assert!(rendered.contains("while running `false`"));
    assert!(rendered.contains("with exit code"));
}

#[test]
fn binds_default_parameter_values() {
    let document = parse_onlyfile(
        r#"hello(name="true"):
    {{name}}
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
        r#"hello(name="false"):
    {{name}}
"#,
    )
    .expect("document should parse");

    let input = CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        task_path: vec!["hello".into()],
        parameter_overrides: vec![("name".into(), "true".into())],
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
        top_level_version_requested: false,
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
        top_level_version_requested: false,
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

#[cfg(unix)]
#[test]
fn runs_verbose_plan_successfully_with_sh() {
    let document = parse_onlyfile(
        "!verbose true
!shell sh
hello():
    true
",
    )
    .expect("document should parse");

    let plan = build_execution_plan(&document, &cli(&["hello"])).expect("plan should build");
    assert!(plan.verbose);
    assert_eq!(plan.shell, ShellKind::Sh);

    let code = run_plan(&plan).expect("verbose runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[cfg(windows)]
#[test]
fn runs_verbose_plan_successfully_with_powershell() {
    let document = parse_onlyfile(
        "!verbose true
!shell powershell
hello():
    exit 0
",
    )
    .expect("document should parse");

    let plan = build_execution_plan(&document, &cli(&["hello"])).expect("plan should build");
    assert!(plan.verbose);
    assert_eq!(plan.shell, ShellKind::PowerShell);

    let code = run_plan(&plan).expect("verbose runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn uses_deno_task_shell_by_default() {
    let document = parse_onlyfile(
        "hello():
    true
",
    )
    .expect("document should parse");

    let plan = build_execution_plan(&document, &cli(&["hello"])).expect("plan should build");
    assert_eq!(plan.shell, ShellKind::Deno);
}

#[test]
fn binds_positional_arguments_for_global_task() {
    let document = parse_onlyfile(
        r#"run(task):
    {{task}}
"#,
    )
    .expect("document should parse");

    let plan = build_execution_plan(&document, &cli(&["run", "true"])).expect("plan should build");

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn binds_positional_arguments_for_namespaced_task() {
    let document = parse_onlyfile(
        r#"[frontend]
build(profile):
    {{profile}}
"#,
    )
    .expect("document should parse");

    let plan = build_execution_plan(&document, &cli(&["frontend", "build", "true"]))
        .expect("plan should build");

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn accepts_named_override_for_required_parameter_through_dynamic_cli() {
    let document = parse_onlyfile(
        r#"run(task):
    {{task}}
"#,
    )
    .expect("document should parse");

    let matches = app::build(&document)
        .try_get_matches_from(["only", "--set", "task=true", "run"])
        .expect("dynamic CLI should accept named override without positional argument");
    let input = CliInput::from_matches(matches.clone())
        .expect("matches should normalize")
        .with_task_path(matches, &document);

    let plan = build_execution_plan(&document, &input).expect("plan should build");
    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[cfg(windows)]
#[test]
fn detects_windows_has_probe_via_pathext() {
    let document = parse_onlyfile(
        r#"probe() ? @has("powershell"):
    true

probe():
    false
"#,
    )
    .expect("document should parse");

    let plan = build_execution_plan(&document, &cli(&["probe"])).expect("plan should build");
    let code = run_plan(&plan).expect("guarded task should be selected");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn runs_tasks_from_onlyfile_base_dir() {
    let root = temp_case_dir("only-runtime-base-dir");
    let onlyfile_path = root.join("Onlyfile");
    fs::write(
        &onlyfile_path,
        "check():
    echo marker > marker.txt
",
    )
    .expect("Onlyfile should be written");

    let input = CliInput {
        onlyfile_path: Some(onlyfile_path),
        print_discovered_path: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        task_path: vec!["check".into()],
        parameter_overrides: vec![],
    };

    let code = run_with(input).expect("runtime should use the Onlyfile base directory");
    assert_eq!(code, ExitCode::SUCCESS);
    assert!(root.join("marker.txt").exists());

    fs::remove_dir_all(root).expect("temp tree should be removed");
}

fn temp_case_dir(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).expect("existing temp tree should be removed");
    }

    fs::create_dir_all(&root).expect("temp tree should be created");
    root
}
