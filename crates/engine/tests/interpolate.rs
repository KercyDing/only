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
