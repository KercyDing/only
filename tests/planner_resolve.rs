use only::{CliInput, build_execution_plan, parse_onlyfile};

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
fn rejects_namespace_without_task_target() {
    let document = parse_onlyfile(
        "bootstrap():
    echo bootstrap

[frontend]
install():
    npm install

workflow() & install & bootstrap:
    npm run build
",
    )
    .expect("document should parse");

    let error = build_execution_plan(&document, &cli(&["frontend"]))
        .expect_err("namespace should require explicit task");

    assert_eq!(
        error.to_string(),
        "namespace 'frontend' requires a task target"
    );
}

#[test]
fn detects_cyclic_dependencies() {
    let document = parse_onlyfile(
        "a() & b:
    echo a

b() & a:
    echo b
",
    )
    .expect("document should parse");

    let error = build_execution_plan(&document, &cli(&["a"])).expect_err("cycle should fail");

    assert_eq!(error.to_string(), "cyclic dependency detected: a -> b -> a");
}
