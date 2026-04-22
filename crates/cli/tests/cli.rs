use only::{
    CliInput, DirectiveAst, DocumentAst, ExecutionPlan, OnlyError, TaskAst, build_cli,
    compile_for_cli_input, discover_onlyfile, parse_onlyfile, run_plan, run_with, version_string,
};
use std::env;
use std::error::Error as _;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::ExitCode;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = env::temp_dir().join(format!("only-{prefix}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).expect("temp dir should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct CurrentDirGuard {
    original: PathBuf,
}

impl CurrentDirGuard {
    fn change_to(path: &Path) -> Self {
        let original = env::current_dir().expect("current dir should be available");
        env::set_current_dir(path).expect("current dir should be changed");
        Self { original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        env::set_current_dir(&self.original).expect("current dir should be restored");
    }
}

fn cwd_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("cwd test lock should not be poisoned")
}

fn cli(task_path: &[&str]) -> CliInput {
    CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        task_path: task_path.iter().map(|part| part.to_string()).collect(),
        parameter_overrides: vec![],
    }
}

fn compile_plan(source: &str, cli: &CliInput) -> ExecutionPlan {
    compile_for_cli_input(source, cli)
        .expect("plan should build")
        .plan
}

fn task<'a>(document: &'a DocumentAst, namespace: Option<&str>, name: &str) -> &'a TaskAst {
    document
        .tasks
        .iter()
        .find(|task| task.namespace.as_deref() == namespace && task.name == name)
        .expect("task should exist")
}

fn temp_case_dir(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).expect("existing temp tree should be removed");
    }

    fs::create_dir_all(&root).expect("temp tree should be created");
    root
}

fn cli_binary_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target");
    path.push("debug");
    path.push(if cfg!(windows) { "only.exe" } else { "only" });
    path
}

fn strip_ansi(input: &str) -> String {
    let mut stripped = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
            continue;
        }

        stripped.push(ch);
    }

    stripped
}

fn assert_same_canonical_path(actual: &Path, expected: &Path) {
    assert_eq!(
        fs::canonicalize(actual).expect("actual path should canonicalize"),
        fs::canonicalize(expected).expect("expected path should canonicalize")
    );
}

#[test]
fn discovers_onlyfile_from_parent_directory() {
    let _cwd_lock = cwd_lock();
    let temp_dir = TempDir::new("discover-parent");
    let nested_dir = temp_dir.path().join("workspace/nested");
    fs::create_dir_all(&nested_dir).expect("nested dir should be created");
    let onlyfile_path = temp_dir.path().join("Onlyfile");
    fs::write(&onlyfile_path, "check():\n    echo ok\n").expect("Onlyfile should be written");

    let _guard = CurrentDirGuard::change_to(&nested_dir);
    let discovered = discover_onlyfile(None).expect("Onlyfile should be discovered");

    assert_same_canonical_path(&discovered.path, &onlyfile_path);
    assert_same_canonical_path(&discovered.base_dir, temp_dir.path());
    assert_eq!(discovered.contents, "check():\n    echo ok\n");
}

#[test]
fn reads_explicit_onlyfile_path() {
    let _cwd_lock = cwd_lock();
    let temp_dir = TempDir::new("discover-explicit");
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).expect("config dir should be created");
    let onlyfile_path = config_dir.join("onlyfile");
    fs::write(&onlyfile_path, "build():\n    echo build\n").expect("onlyfile should be written");

    let discovered =
        discover_onlyfile(Some(&onlyfile_path)).expect("explicit Onlyfile should be loaded");

    assert_eq!(discovered.path, onlyfile_path);
    assert_eq!(discovered.base_dir, config_dir);
    assert_eq!(discovered.contents, "build():\n    echo build\n");
}

#[test]
fn returns_not_found_when_onlyfile_does_not_exist() {
    let _cwd_lock = cwd_lock();
    let temp_dir = TempDir::new("discover-missing");
    let empty_dir = temp_dir.path().join("empty");
    fs::create_dir_all(&empty_dir).expect("empty dir should be created");

    let _guard = CurrentDirGuard::change_to(&empty_dir);
    let error = discover_onlyfile(None).expect_err("missing Onlyfile should fail");

    match error {
        OnlyError::NotFound(message) => {
            assert_eq!(
                message,
                "No Onlyfile found in current directory or any parent."
            );
        }
        other => panic!("expected not found error, got {other:?}"),
    }
}

#[test]
fn renders_io_error_with_path() {
    let error = OnlyError::io_with_path(
        "failed to read Onlyfile",
        PathBuf::from("/tmp/Onlyfile"),
        std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
    );

    assert_eq!(
        error.to_string(),
        "failed to read Onlyfile: /tmp/Onlyfile: missing"
    );
    assert!(error.source().is_some());
}

#[test]
fn renders_not_found_error_message() {
    let error = OnlyError::not_found("No Onlyfile found.".to_string());
    assert_eq!(error.to_string(), "No Onlyfile found.");
    assert!(error.source().is_none());
}

#[test]
fn renders_unsupported_error_message() {
    let error = OnlyError::unsupported("unsupported shell");
    assert_eq!(error.to_string(), "unsupported shell");
    assert!(error.source().is_none());
}

#[test]
fn parses_empty_onlyfile() {
    let document = parse_onlyfile("").expect("empty Onlyfile should parse");
    assert!(document.directives.is_empty());
    assert!(document.tasks.is_empty());
    assert!(document.namespaces.is_empty());
}

#[test]
fn parses_minimal_document_shape() {
    let source =
        "!echo false\n!shell sh\nhello():\n    echo hello\n[tools]\nfmt():\n    cargo fmt\n";
    let document = parse_onlyfile(source).expect("minimal document should parse");

    assert!(matches!(
        document.directives[0],
        DirectiveAst::Echo { value: false, .. }
    ));
    assert!(matches!(
        document.directives[1],
        DirectiveAst::Shell { ref shell, .. } if shell == "sh"
    ));
    assert_eq!(
        task(&document, None, "hello").commands[0].text,
        "echo hello"
    );
    assert_eq!(document.namespaces[0].name, "tools");
    assert_eq!(task(&document, Some("tools"), "fmt").name, "fmt");
}

#[test]
fn rejects_ambiguous_guards() {
    let source = "build() ? @os(\"linux\"):
    echo one

build() ? @os(\"linux\"):
    echo two
";
    let error = parse_onlyfile(source).expect_err("ambiguous guards should fail");
    let rendered = error.to_string();
    assert!(rendered.contains("ambiguous guard: 'build' conflicts with 'build'"));
}

#[test]
fn rejects_duplicate_parameter_names() {
    let source = "build(tag, tag=\"v1\"):
    echo build
";
    let error = parse_onlyfile(source).expect_err("duplicate parameters should fail");
    assert_eq!(
        error.to_string(),
        "duplicate parameter 'tag' in task 'build'"
    );
}

#[test]
fn rejects_duplicate_directives() {
    let source = "!echo false\n!echo true\n!shell sh\n!shell bash\nbuild():\n    echo build\n";
    let error = parse_onlyfile(source).expect_err("duplicate directives should fail");
    let rendered = error.to_string();

    assert!(rendered.contains("duplicate directive '!echo'"));
    assert!(rendered.contains("duplicate directive '!shell'"));
}

#[test]
fn assigns_following_tasks_to_current_namespace() {
    let source = "[frontend]
build():
    npm run build

test():
    npm test

[backend]
serve():
    cargo run
";
    let document = parse_onlyfile(source).expect("namespaced tasks should parse");

    assert!(document.tasks.iter().all(|task| task.namespace.is_some()));
    assert_eq!(document.namespaces.len(), 2);
    assert_eq!(document.namespaces[0].name, "frontend");
    assert_eq!(document.namespaces[1].name, "backend");
    assert_eq!(
        document
            .tasks
            .iter()
            .filter(|task| task.namespace.as_deref() == Some("frontend"))
            .count(),
        2
    );
    assert_eq!(task(&document, Some("frontend"), "build").name, "build");
    assert_eq!(task(&document, Some("frontend"), "test").name, "test");
    assert_eq!(task(&document, Some("backend"), "serve").name, "serve");
}

#[test]
fn does_not_assign_namespace_doc_to_first_task() {
    let source = "% Developer workflow.\n[dev]\nsmoke():\n    echo smoke\n";
    let document = parse_onlyfile(source).expect("namespaced tasks should parse");

    assert_eq!(
        document.namespaces[0].doc.as_deref(),
        Some("Developer workflow.")
    );
    assert_eq!(task(&document, Some("dev"), "smoke").name, "smoke");
    assert!(task(&document, Some("dev"), "smoke").doc.is_none());
}

#[test]
fn rejects_undefined_dependency_during_parse_validation() {
    let source = "deploy() & build:\n    echo deploy\n";
    let error = parse_onlyfile(source).expect_err("undefined dependency should fail validation");
    assert_eq!(
        error.to_string(),
        "undefined dependency 'build' referenced from 'deploy'"
    );
}

#[test]
fn accepts_local_and_global_dependencies() {
    let source = "bootstrap():\n    echo bootstrap\n[frontend]\ninstall():\n    npm install\nbuild() & install & bootstrap:\n    npm run build\n";
    parse_onlyfile(source).expect("valid dependency graph should parse");
}

#[test]
fn compiles_parallel_dependency_groups_into_shared_stage() {
    let plan = compile_plan(
        "fmt():\n    true\nlint():\n    true\nbuild():\n    true\nci() & fmt & (lint, build):\n    true\n",
        &cli(&["ci"]),
    );

    assert_eq!(plan.nodes.len(), 4);
    assert_eq!(plan.nodes[0].name, "fmt");
    assert_eq!(plan.nodes[0].stage, 0);
    assert_eq!(plan.nodes[1].name, "lint");
    assert_eq!(plan.nodes[1].stage, 1);
    assert_eq!(plan.nodes[2].name, "build");
    assert_eq!(plan.nodes[2].stage, 1);
    assert_eq!(plan.nodes[3].name, "ci");
    assert_eq!(plan.nodes[3].stage, 2);
}

#[test]
fn rejects_namespace_without_task_target() {
    let error = compile_for_cli_input(
        "bootstrap():
    echo bootstrap

[frontend]
install():
    npm install
",
        &cli(&["frontend"]),
    )
    .expect_err("namespace should require explicit task");

    assert_eq!(
        error.to_string(),
        "namespace 'frontend' requires a task target"
    );
}

#[test]
fn detects_cyclic_dependencies() {
    let error = compile_for_cli_input(
        "a() & b:
    echo a

b() & a:
    echo b
",
        &cli(&["a"]),
    )
    .expect_err("cycle should fail");

    assert_eq!(error.to_string(), "cyclic dependency detected: a -> b -> a");
}

#[test]
fn runs_successful_plan() {
    let _cwd_lock = cwd_lock();
    let plan = compile_plan(
        "hello():
    true
",
        &cli(&["hello"]),
    );

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn propagates_command_failure() {
    let _cwd_lock = cwd_lock();
    let plan = compile_plan(
        "fail():
    false
",
        &cli(&["fail"]),
    );

    let error = run_plan(&plan).expect_err("runtime should return contextual error");
    let rendered = error.to_string();
    assert!(rendered.contains("task 'fail' failed at step [1/1]"));
    assert!(rendered.contains("while running `false`"));
    assert!(rendered.contains("with exit code"));
}

#[test]
fn binds_default_parameter_values() {
    let _cwd_lock = cwd_lock();
    let plan = compile_plan(
        r#"hello(name="true"):
    {{name}}
"#,
        &cli(&["hello"]),
    );

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn applies_cli_parameter_overrides() {
    let _cwd_lock = cwd_lock();
    let input = CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        task_path: vec!["hello".into()],
        parameter_overrides: vec![("name".into(), "true".into())],
    };

    let plan = compile_plan(
        r#"hello(name="false"):
    {{name}}
"#,
        &input,
    );

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn rejects_missing_required_parameter() {
    let error = compile_for_cli_input(
        "hello(name):
    echo {{name}}
",
        &cli(&["hello"]),
    )
    .expect_err("missing parameter should fail planning");

    assert_eq!(error.to_string(), "missing required parameter '{{name}}'");
}

#[test]
fn rejects_unknown_parameter_override() {
    let input = CliInput {
        onlyfile_path: None,
        print_discovered_path: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        task_path: vec!["hello".into()],
        parameter_overrides: vec![("other".into(), "alice".into())],
    };

    let error = compile_for_cli_input(
        r#"hello(name="world"):
    echo {{name}}
"#,
        &input,
    )
    .expect_err("unknown parameter should fail planning");

    assert_eq!(
        error.to_string(),
        "unknown parameter 'other' for task 'hello'"
    );
}

#[test]
fn rejects_duplicate_parameter_overrides() {
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

    let error = compile_for_cli_input(
        r#"hello(name="world"):
    echo {{name}}
"#,
        &input,
    )
    .expect_err("duplicate override should fail planning");

    assert_eq!(error.to_string(), "duplicate parameter override 'name'");
}

#[cfg(unix)]
#[test]
fn runs_echo_plan_successfully_with_sh() {
    let _cwd_lock = cwd_lock();
    let plan = compile_plan(
        "!echo true
!shell sh
hello():
    true
",
        &cli(&["hello"]),
    );
    assert!(plan.echo);
    assert_eq!(plan.shell.as_deref(), Some("sh"));

    let code = run_plan(&plan).expect("echo-enabled runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[cfg(windows)]
#[test]
fn runs_echo_plan_successfully_with_powershell() {
    let _cwd_lock = cwd_lock();
    let plan = compile_plan(
        "!echo true
!shell powershell
hello():
    exit 0
",
        &cli(&["hello"]),
    );
    assert!(plan.echo);
    assert_eq!(plan.shell.as_deref(), Some("powershell"));

    let code = run_plan(&plan).expect("echo-enabled runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn uses_deno_task_shell_by_default() {
    let _cwd_lock = cwd_lock();
    let plan = compile_plan(
        "hello():
    true
",
        &cli(&["hello"]),
    );
    assert_eq!(plan.shell.as_deref(), None);
}

#[test]
fn cli_run_prints_task_progress_by_default() {
    let _cwd_lock = cwd_lock();
    let temp_dir = TempDir::new("echo-cli-progress");
    let onlyfile_path = temp_dir.path().join("Onlyfile");
    fs::write(
        &onlyfile_path,
        r#"prepare():
    echo "prepare out"

check():
    echo "check out"

ci() & prepare & check:
    echo "ci out"
"#,
    )
    .expect("Onlyfile should be written");

    let output = Command::new(cli_binary_path())
        .arg("ci")
        .current_dir(temp_dir.path())
        .output()
        .expect("CLI process should run");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf-8");
    let plain_stdout = strip_ansi(&stdout);
    let plain_stderr = strip_ansi(&stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected CLI to succeed, stderr was: {stderr}"
    );
    assert!(plain_stdout.contains("[prepare] prepare out"));
    assert!(plain_stdout.contains("[check] check out"));
    assert!(plain_stdout.contains("[ci] ci out"));
    assert!(
        plain_stderr.contains("[task 1/3] prepare"),
        "expected first task progress in stderr, got: {stderr}"
    );
    assert!(
        plain_stderr.contains("[task 2/3] check"),
        "expected second task progress in stderr, got: {stderr}"
    );
    assert!(
        plain_stderr.contains("[task 3/3] ci"),
        "expected final task progress in stderr, got: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn echo_false_is_quiet_on_success() {
    let _cwd_lock = cwd_lock();
    let temp_dir = TempDir::new("echo-false-unix");
    let onlyfile_path = temp_dir.path().join("Onlyfile");
    fs::write(
        &onlyfile_path,
        r#"!echo false
!shell sh
prepare():
    printf 'prepare out\n'
    printf 'prepare err\n' >&2

ci() & prepare:
    printf 'ci out\n'
    printf 'ci err\n' >&2
"#,
    )
    .expect("Onlyfile should be written");

    let output = Command::new(cli_binary_path())
        .arg("ci")
        .current_dir(temp_dir.path())
        .output()
        .expect("CLI process should run");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf-8");
    let plain_stdout = strip_ansi(&stdout);
    let plain_stderr = strip_ansi(&stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected CLI to succeed, stderr was: {stderr}"
    );
    assert_eq!(plain_stdout, "");
    assert!(plain_stderr.contains("[task 1/2] prepare"));
    assert!(plain_stderr.contains("[task 2/2] ci"));
    assert!(plain_stderr.contains("Success"));
    assert!(!plain_stderr.contains("[prepare]"));
    assert!(!plain_stderr.contains("[ci]"));
    assert!(!plain_stderr.contains("prepare out"));
    assert!(!plain_stderr.contains("ci out"));
}

#[cfg(unix)]
#[test]
fn echo_false_replays_stderr_on_failure() {
    let _cwd_lock = cwd_lock();
    let temp_dir = TempDir::new("echo-false-failure-unix");
    let onlyfile_path = temp_dir.path().join("Onlyfile");
    fs::write(
        &onlyfile_path,
        r#"!echo false
!shell sh
fail():
    printf 'fail out\n'
    printf 'fail err\n' >&2
    exit 1
"#,
    )
    .expect("Onlyfile should be written");

    let output = Command::new(cli_binary_path())
        .arg("fail")
        .current_dir(temp_dir.path())
        .output()
        .expect("CLI process should run");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf-8");
    let plain_stdout = strip_ansi(&stdout);
    let plain_stderr = strip_ansi(&stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "expected CLI to fail, stderr was: {stderr}"
    );
    assert_eq!(plain_stdout, "");
    assert!(plain_stderr.contains("[task 1/1] fail"));
    assert!(plain_stderr.contains("[fail] fail err"));
    assert!(plain_stderr.contains("Fail"));
    assert!(plain_stderr.contains("Error:"));
    assert!(!plain_stderr.contains("fail out"));
}

#[test]
fn grouped_parallel_output_uses_task_prefixes() {
    let _cwd_lock = cwd_lock();
    let temp_dir = TempDir::new("grouped-parallel-output");
    let onlyfile_path = temp_dir.path().join("Onlyfile");
    fs::write(
        &onlyfile_path,
        r#"fmt():
    echo "fmt start"
    sleep 0.05
    echo "fmt end"

test():
    echo "test start"
    sleep 0.01
    echo "test end"

ci() & (fmt, test):
    echo "ci end"
"#,
    )
    .expect("Onlyfile should be written");

    let output = Command::new(cli_binary_path())
        .arg("ci")
        .current_dir(temp_dir.path())
        .output()
        .expect("CLI process should run");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    let plain_stdout = strip_ansi(&stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected CLI to succeed, stdout was: {stdout}"
    );
    assert!(plain_stdout.contains("[fmt] fmt start\n[fmt] fmt end\n"));
    assert!(plain_stdout.contains("[test] test start\n[test] test end\n"));
    assert!(plain_stdout.contains("[ci] ci end\n"));
    assert!(stdout.contains("\u{1b}["));
    assert!(
        plain_stdout
            .find("[fmt] fmt start")
            .expect("fmt output should exist")
            < plain_stdout
                .find("[test] test start")
                .expect("test output should exist")
    );
}

#[test]
fn grouped_parallel_output_streams_before_stage_completion() {
    let _cwd_lock = cwd_lock();
    let temp_dir = TempDir::new("grouped-parallel-streaming");
    let onlyfile_path = temp_dir.path().join("Onlyfile");
    fs::write(
        &onlyfile_path,
        r#"fmt():
    echo "fmt start"
    sleep 0.05
    echo "fmt end"

test():
    sleep 0.30
    echo "test end"

ci() & (fmt, test):
    echo "ci end"
"#,
    )
    .expect("Onlyfile should be written");

    let mut child = Command::new(cli_binary_path())
        .arg("ci")
        .current_dir(temp_dir.path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("CLI process should spawn");
    let stdout = child.stdout.take().expect("stdout should be piped");
    let mut reader = BufReader::new(stdout);
    let mut collected = String::new();
    let mut line = String::new();

    loop {
        line.clear();
        let read = reader
            .read_line(&mut line)
            .expect("stdout should remain readable");
        if read == 0 {
            break;
        }
        collected.push_str(&line);
        let plain = strip_ansi(&collected);
        if plain.contains("[fmt] fmt end\n") {
            assert!(
                child
                    .try_wait()
                    .expect("child status should be readable")
                    .is_none(),
                "child should still be running after first task output: {plain}"
            );
            break;
        }
    }

    let status = child.wait().expect("child should exit");
    assert_eq!(status.code(), Some(0));
}

#[test]
fn binds_positional_arguments_for_global_task() {
    let _cwd_lock = cwd_lock();
    let plan = compile_plan(
        r#"run(task):
    {{task}}
"#,
        &cli(&["run", "true"]),
    );

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn binds_positional_arguments_for_namespaced_task() {
    let _cwd_lock = cwd_lock();
    let plan = compile_plan(
        r#"[frontend]
build(profile):
    {{profile}}
"#,
        &cli(&["frontend", "build", "true"]),
    );

    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn accepts_named_override_for_required_parameter_through_dynamic_cli() {
    let _cwd_lock = cwd_lock();
    let document = parse_onlyfile(
        r#"run(task):
    {{task}}
"#,
    )
    .expect("document should parse");

    let matches = build_cli(&document)
        .try_get_matches_from(["only", "--set", "task=true", "run"])
        .expect("dynamic CLI should accept named override without positional argument");
    let input = CliInput::from_matches(matches.clone())
        .expect("matches should normalize")
        .with_task_path(matches, &document);

    let plan = compile_plan(
        r#"run(task):
    {{task}}
"#,
        &input,
    );
    let code = run_plan(&plan).expect("runtime should succeed");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[cfg(windows)]
#[test]
fn detects_windows_has_probe_via_pathext() {
    let _cwd_lock = cwd_lock();
    let plan = compile_plan(
        r#"probe() ? @has("powershell"):
    true

probe():
    false
"#,
        &cli(&["probe"]),
    );
    let code = run_plan(&plan).expect("guarded task should be selected");
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn runs_tasks_from_onlyfile_base_dir() {
    let _cwd_lock = cwd_lock();
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

#[test]
fn run_with_selects_guarded_task_for_current_environment() {
    let _cwd_lock = cwd_lock();
    let root = temp_case_dir("only-runtime-guarded-task");
    let onlyfile_path = root.join("Onlyfile");
    let current_os = std::env::consts::OS;
    let other_os = if current_os == "windows" {
        "linux"
    } else {
        "windows"
    };
    fs::write(
        &onlyfile_path,
        format!(
            "probe() ? @os(\"{current_os}\"):\n    echo guarded > guarded.txt\nprobe() ? @os(\"{other_os}\"):\n    echo skipped > skipped.txt\nprobe():\n    echo fallback > fallback.txt\n"
        ),
    )
    .expect("Onlyfile should be written");

    let input = CliInput {
        onlyfile_path: Some(onlyfile_path),
        print_discovered_path: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        task_path: vec!["probe".into()],
        parameter_overrides: vec![],
    };

    let code = run_with(input).expect("guarded runtime should select the matching variant");
    assert_eq!(code, ExitCode::SUCCESS);
    assert!(root.join("guarded.txt").exists());
    assert!(!root.join("skipped.txt").exists());
    assert!(!root.join("fallback.txt").exists());

    fs::remove_dir_all(root).expect("temp tree should be removed");
}

#[test]
fn run_with_reports_unavailable_guarded_task() {
    let _cwd_lock = cwd_lock();
    let root = temp_case_dir("only-runtime-unavailable-guard");
    let onlyfile_path = root.join("Onlyfile");
    let other_os = if std::env::consts::OS == "windows" {
        "linux"
    } else {
        "windows"
    };
    fs::write(
        &onlyfile_path,
        format!("probe() ? @os(\"{other_os}\"):\n    true\n"),
    )
    .expect("Onlyfile should be written");

    let input = CliInput {
        onlyfile_path: Some(onlyfile_path),
        print_discovered_path: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        task_path: vec!["probe".into()],
        parameter_overrides: vec![],
    };

    let error = run_with(input).expect_err("unavailable guarded task should fail");
    assert_eq!(
        error.to_string(),
        "task 'probe' is not available for this environment"
    );

    fs::remove_dir_all(root).expect("temp tree should be removed");
}

#[test]
fn run_with_rejects_error_diagnostics_before_execution() {
    let _cwd_lock = cwd_lock();
    let root = temp_case_dir("only-runtime-diagnostics");
    let onlyfile_path = root.join("Onlyfile");
    fs::write(
        &onlyfile_path,
        "deploy() & build:\n    echo ran > ran.txt\n",
    )
    .expect("Onlyfile should be written");

    let input = CliInput {
        onlyfile_path: Some(onlyfile_path),
        print_discovered_path: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        task_path: vec!["deploy".into()],
        parameter_overrides: vec![],
    };

    let error = run_with(input).expect_err("semantic errors should stop execution");
    assert_eq!(
        error.to_string(),
        "undefined dependency 'build' referenced from 'deploy'"
    );
    assert!(!root.join("ran.txt").exists());

    fs::remove_dir_all(root).expect("temp tree should be removed");
}

#[test]
fn rejects_direct_helper_task_execution_via_run_with() {
    let _cwd_lock = cwd_lock();
    let root = temp_case_dir("only-runtime-helper-task");
    let onlyfile_path = root.join("Onlyfile");
    fs::write(&onlyfile_path, "_prepare():\n    echo helper\n")
        .expect("Onlyfile should be written");

    let input = CliInput {
        onlyfile_path: Some(onlyfile_path),
        print_discovered_path: false,
        top_level_help_requested: false,
        top_level_version_requested: false,
        task_path: vec!["_prepare".into()],
        parameter_overrides: vec![],
    };

    let error = run_with(input).expect_err("helper task should not execute directly");
    assert_eq!(
        error.to_string(),
        "helper task '_prepare' cannot be invoked directly"
    );

    fs::remove_dir_all(root).expect("temp tree should be removed");
}

#[test]
fn helper_task_help_is_available_via_cli_binary() {
    let _cwd_lock = cwd_lock();
    let temp_dir = TempDir::new("helper-help-hidden");
    let onlyfile_path = temp_dir.path().join("Onlyfile");
    fs::write(&onlyfile_path, "_prepare(target):\n    printf '%s\\n' \"{{target}}\"\n")
        .expect("Onlyfile should be written");

    let output = Command::new(cli_binary_path())
        .arg("-f")
        .arg(&onlyfile_path)
        .arg("_prepare")
        .arg("--help")
        .output()
        .expect("CLI process should run");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf-8");
    let plain_stdout = strip_ansi(&stdout);
    let plain_stderr = strip_ansi(&stderr);

    assert_eq!(output.status.code(), Some(0), "stderr was: {stderr}");
    assert!(plain_stdout.contains("Usage: only _prepare [OPTIONS] [target]"));
    assert!(plain_stdout.contains("[target]  Required parameter"));
    assert!(plain_stderr.is_empty(), "stderr was: {plain_stderr}");
}

#[cfg(unix)]
#[test]
fn preview_prints_selected_variant_and_commands_before_execution() {
    let _cwd_lock = cwd_lock();
    let temp_dir = TempDir::new("preview-cli-unix");
    let onlyfile_path = temp_dir.path().join("Onlyfile");
    let current_os = std::env::consts::OS;
    let other_os = if current_os == "windows" {
        "linux"
    } else {
        "windows"
    };
    fs::write(
        &onlyfile_path,
        format!(
            "!preview true\nprobe() ? @os(\"{current_os}\"):\n    printf 'guarded\\n'\nprobe() ? @os(\"{other_os}\"):\n    printf 'skipped\\n'\nprobe():\n    printf 'fallback\\n'\n"
        ),
    )
    .expect("Onlyfile should be written");

    let output = Command::new(cli_binary_path())
        .arg("probe")
        .current_dir(temp_dir.path())
        .output()
        .expect("CLI process should run");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be valid utf-8");
    let plain_stdout = strip_ansi(&stdout);
    let plain_stderr = strip_ansi(&stderr);

    assert_eq!(output.status.code(), Some(0), "stderr was: {stderr}");
    assert!(plain_stderr.contains("Preview:"));
    assert!(plain_stderr.contains(&format!("variant: probe() ? @os(\"{current_os}\")")));
    assert!(plain_stderr.contains("[probe] printf 'guarded"));
    assert!(plain_stdout.contains("[probe] guarded"));
}

#[test]
fn exposes_workspace_cli_version() {
    assert_eq!(version_string(), env!("CARGO_PKG_VERSION"));
}
