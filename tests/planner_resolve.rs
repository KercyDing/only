use only::{CliInput, build_execution_plan, parse_onlyfile};

#[test]
fn resolves_namespace_default_task_and_dependencies() {
    let document = parse_onlyfile(
        "bootstrap():
    echo bootstrap

[frontend]
install():
    npm install

default() & install & bootstrap:
    npm run build
",
    )
    .expect("document should parse");

    let plan = build_execution_plan(
        &document,
        &CliInput {
            onlyfile_path: None,
            print_discovered_path: false,
            task: Some("frontend".into()),
            subtask: None,
            parameter_overrides: vec![],
        },
    )
    .expect("plan should build");

    let names = plan
        .nodes
        .iter()
        .map(|node| node.qualified_name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec!["frontend.install", "bootstrap", "frontend.default"]
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

    let error = build_execution_plan(
        &document,
        &CliInput {
            onlyfile_path: None,
            print_discovered_path: false,
            task: Some("a".into()),
            subtask: None,
            parameter_overrides: vec![],
        },
    )
    .expect_err("cycle should fail");

    assert_eq!(error.to_string(), "cyclic dependency detected: a -> b -> a");
}
