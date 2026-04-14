use only_engine::{Invocation, build_execution_plan, render_command};
use only_semantic::compile_document;

#[test]
fn renders_interpolated_command_from_bound_parameters() {
    let compiled = compile_document("hello(name=\"world\"):\n    echo {{name}}\n");
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "hello",
            args: vec![],
            overrides: vec![],
        },
    );

    let rendered = render_command(&plan.nodes[0].commands[0], &plan.nodes[0].params)
        .expect("interpolation should succeed");

    assert_eq!(rendered, "echo world");
}

#[test]
fn preserves_escaped_interpolation_braces() {
    let compiled = compile_document(
        r#"hello(name="world"):
    echo \{{name\}} {{name}}
"#,
    );
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "hello",
            args: vec![],
            overrides: vec![],
        },
    );

    let rendered = render_command(&plan.nodes[0].commands[0], &plan.nodes[0].params)
        .expect("escaped braces should render");

    assert_eq!(rendered, "echo {{name}} world");
}

#[test]
fn keeps_even_backslashes_before_real_interpolation() {
    let compiled = compile_document(
        r#"hello(name="world"):
    echo \\{{name}}
"#,
    );
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "hello",
            args: vec![],
            overrides: vec![],
        },
    );

    let rendered = render_command(&plan.nodes[0].commands[0], &plan.nodes[0].params)
        .expect("double backslashes should keep interpolation active");

    assert_eq!(rendered, r#"echo \\world"#);
}

#[test]
fn reports_unterminated_interpolation() {
    let compiled = compile_document("hello(name=\"world\"):\n    echo {{name\n");
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "hello",
            args: vec![],
            overrides: vec![],
        },
    );

    let error = render_command(&plan.nodes[0].commands[0], &plan.nodes[0].params)
        .expect_err("invalid interpolation should fail");

    assert!(error.to_string().contains("unterminated interpolation"));
}
