use only_engine::{Invocation, build_execution_plan, try_build_execution_plan};
use only_semantic::compile_document;

#[test]
fn builds_dag_order_from_semantic_ast() {
    let compiled = compile_document("check():\n    cargo check\nci() & check:\n    echo done\n");
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "ci",
            args: vec![],
            overrides: vec![],
        },
    );

    assert_eq!(plan.nodes.len(), 2);
    assert_eq!(plan.nodes[0].name, "check");
    assert_eq!(plan.nodes[1].name, "ci");
}

#[test]
fn builds_namespaced_dag_order_from_semantic_ast() {
    let compiled = compile_document(
        "[dev]\n\
         build():\n\
             cargo build\n\
         serve() & build:\n\
             cargo run\n",
    );
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "dev.serve",
            args: vec![],
            overrides: vec![],
        },
    );

    assert_eq!(plan.nodes.len(), 2);
    assert_eq!(plan.nodes[0].name, "dev.build");
    assert_eq!(plan.nodes[1].name, "dev.serve");
}

#[test]
fn assigns_parallel_dependency_groups_to_shared_stage() {
    let compiled = compile_document(
        "fmt():\n    cargo fmt\nlint():\n    cargo clippy\nbuild():\n    cargo build\nci() & fmt & (lint, build):\n    echo done\n",
    );
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "ci",
            args: vec![],
            overrides: vec![],
        },
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
fn carries_echo_shell_and_default_params_into_plan() {
    let compiled = compile_document(
        "!echo true\n\
         !shell bash\n\
         build(tag=\"v1\") shell?=pwsh:\n\
             echo {{tag}}\n",
    );
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "build",
            args: vec![],
            overrides: vec![],
        },
    );

    assert!(plan.echo);
    assert_eq!(plan.shell.as_deref(), Some("bash"));
    assert_eq!(plan.nodes.len(), 1);
    assert_eq!(plan.nodes[0].shell.as_deref(), Some("pwsh"));
    assert!(plan.nodes[0].shell_fallback);
    assert_eq!(plan.nodes[0].params.len(), 1);
    assert_eq!(plan.nodes[0].params[0].name, "tag");
    assert_eq!(plan.nodes[0].params[0].default_value.as_deref(), Some("v1"));
}

#[test]
fn binds_positional_and_named_parameter_inputs() {
    let compiled = compile_document("run(task, profile=\"dev\"):\n    echo {{task}} {{profile}}\n");
    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "run",
            args: vec!["check"],
            overrides: vec![("profile", "release")],
        },
    )
    .expect("plan should build");

    assert_eq!(plan.nodes.len(), 1);
    assert_eq!(plan.nodes[0].params[0].name, "task");
    assert_eq!(plan.nodes[0].params[0].value.as_deref(), Some("check"));
    assert_eq!(plan.nodes[0].params[1].name, "profile");
    assert_eq!(plan.nodes[0].params[1].value.as_deref(), Some("release"));
}

#[test]
fn rejects_missing_required_parameter_for_new_engine_planner() {
    let compiled = compile_document("run(task):\n    echo {{task}}\n");
    let error = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "run",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect_err("missing parameter should fail");

    assert_eq!(error.to_string(), "missing required parameter '{{task}}'");
}

#[test]
fn rejects_unknown_override_for_new_engine_planner() {
    let compiled = compile_document("run(task=\"dev\"):\n    echo {{task}}\n");
    let error = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "run",
            args: vec![],
            overrides: vec![("other", "x")],
        },
    )
    .expect_err("unknown override should fail");

    assert_eq!(
        error.to_string(),
        "unknown parameter 'other' for task 'run'"
    );
}

#[test]
fn rejects_duplicate_parameter_overrides_for_new_engine_planner() {
    let compiled = compile_document("run(task=\"dev\"):\n    echo {{task}}\n");
    let error = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "run",
            args: vec![],
            overrides: vec![("task", "a"), ("task", "b")],
        },
    )
    .expect_err("duplicate override should fail");

    assert_eq!(error.to_string(), "duplicate parameter override 'task'");
}

#[test]
fn rejects_too_many_arguments_for_new_engine_planner() {
    let compiled = compile_document("run(task):\n    echo {{task}}\n");
    let error = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "run",
            args: vec!["a", "b"],
            overrides: vec![],
        },
    )
    .expect_err("too many args should fail");

    assert_eq!(
        error.to_string(),
        "too many arguments for task 'run'; expected at most 1, got 2"
    );
}

#[test]
fn detects_cyclic_dependencies_for_new_engine_planner() {
    let compiled = compile_document("a() & b:\n    echo a\nb() & a:\n    echo b\n");
    let error = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "a",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect_err("cycle should fail");

    assert_eq!(error.to_string(), "cyclic dependency detected: a -> b -> a");
}

#[test]
fn selects_guarded_root_task_variant_for_current_environment() {
    let current_os = std::env::consts::OS;
    let other_os = if current_os == "windows" {
        "linux"
    } else {
        "windows"
    };
    let compiled = compile_document(&format!(
        "probe() ? @os(\"{current_os}\"):\n    echo guarded\nprobe() ? @os(\"{other_os}\"):\n    echo skipped\nprobe():\n    echo fallback\n"
    ));

    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "probe",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect("guarded task should resolve");

    assert_eq!(plan.nodes.len(), 1);
    assert_eq!(plan.nodes[0].name, "probe");
    assert_eq!(plan.nodes[0].commands, vec!["echo guarded"]);
}

#[test]
fn selects_guarded_dependency_variant_for_current_environment() {
    let current_os = std::env::consts::OS;
    let other_os = if current_os == "windows" {
        "linux"
    } else {
        "windows"
    };
    let compiled = compile_document(&format!(
        "build() ? @os(\"{current_os}\"):\n    echo guarded-build\nbuild() ? @os(\"{other_os}\"):\n    echo skipped-build\nbuild():\n    echo fallback-build\nci() & build:\n    echo ci\n"
    ));

    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "ci",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect("guarded dependency should resolve");

    assert_eq!(plan.nodes.len(), 2);
    assert_eq!(plan.nodes[0].name, "build");
    assert_eq!(plan.nodes[0].commands, vec!["echo guarded-build"]);
    assert_eq!(plan.nodes[1].name, "ci");
}

#[test]
fn rejects_direct_invocation_of_helper_task() {
    let compiled = compile_document("_prepare():\n    echo helper\n");
    let error = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "_prepare",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect_err("helper task should not be invokable directly");

    assert_eq!(
        error.to_string(),
        "helper task '_prepare' cannot be invoked directly"
    );
}

#[test]
fn allows_helper_task_as_dependency() {
    let compiled =
        compile_document("_prepare():\n    echo helper\nci() & _prepare:\n    echo ci\n");
    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "ci",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect("helper dependency should remain usable");

    assert_eq!(plan.nodes.len(), 2);
    assert_eq!(plan.nodes[0].name, "_prepare");
    assert_eq!(plan.nodes[1].name, "ci");
}

#[test]
fn carries_preview_directive_into_plan() {
    let compiled = compile_document("!preview true\nhello():\n    echo ok\n");
    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "hello",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect("preview directive should compile into plan");

    assert!(plan.preview);
}

#[test]
fn leaves_preview_disabled_by_default() {
    let compiled = compile_document("hello():\n    echo ok\n");
    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "hello",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect("plan should build without preview directive");

    assert!(!plan.preview);
}

#[test]
fn reports_unavailable_root_task_for_current_environment() {
    let other_os = if std::env::consts::OS == "windows" {
        "linux"
    } else {
        "windows"
    };
    let compiled = compile_document(&format!(
        "probe() ? @os(\"{other_os}\"):\n    echo skipped\n"
    ));

    let error = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "probe",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect_err("unavailable guarded target should fail");

    assert_eq!(
        error.to_string(),
        "task 'probe' is not available for this environment"
    );
}
