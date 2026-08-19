use only_engine::{Invocation, build_execution_plan, try_build_execution_plan};
use only_semantic::{ShellKind, ShellOperator, compile_document};

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
fn builds_group_dag_order_from_semantic_ast() {
    let compiled = compile_document(
        "group dev {\n\
         build():\n\
             cargo build\n\
         serve() & build:\n\
             cargo run\n\
         }\n",
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
fn applies_global_values_across_dag() {
    let compiled = compile_document(concat!(
        "!version 0.4\n",
        "!var profile = \"release\"\n",
        "prepare():\n",
        "    echo {{profile}}\n",
        "build(profile=\"debug\") & prepare:\n",
        "    echo {{profile}}\n",
    ));
    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "build",
            args: vec![],
            overrides: vec![("profile", "dist")],
        },
    )
    .expect("global and local values should bind");

    assert_eq!(plan.nodes[0].params[0].value.as_deref(), Some("release"));
    assert_eq!(plan.nodes[1].params[0].value.as_deref(), Some("dist"));
}

#[test]
fn carries_result_messages_with_global_interpolation() {
    let compiled = compile_document(
        "!version 0.4\n!var output = \"dist\"\n[pass] wrote {{output}}\n[fail] could not write {{output}}\nbuild():\n    true\n",
    );
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "build",
            args: vec![],
            overrides: vec![],
        },
    );

    assert_eq!(plan.nodes[0].pass.as_deref(), Some("wrote {{output}}"));
    assert_eq!(
        plan.nodes[0].fail.as_deref(),
        Some("could not write {{output}}")
    );
    assert_eq!(
        plan.nodes[0].result_params[0].value.as_deref(),
        Some("dist")
    );
}

#[test]
fn inherits_variant_result_messages() {
    let unavailable_os = if std::env::consts::OS == "windows" {
        "linux"
    } else {
        "windows"
    };
    let compiled = compile_document(&format!(
        "!version 0.4\n[pass] Tests passed.\n[fail] Tests failed.\ntest() ? @os(\"{unavailable_os}\"):\n    true\n\ntest():\n    true\n"
    ));
    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "test",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect("fallback variant should inherit result messages");

    assert_eq!(plan.nodes[0].pass.as_deref(), Some("Tests passed."));
    assert_eq!(plan.nodes[0].fail.as_deref(), Some("Tests failed."));
}

#[test]
fn overrides_global_value_for_whole_dag() {
    let compiled = compile_document(concat!(
        "!version 0.4\n",
        "!var profile = \"release\"\n",
        "prepare():\n",
        "    echo {{profile}}\n",
        "build() & prepare:\n",
        "    echo {{profile}}\n",
    ));
    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "build",
            args: vec![],
            overrides: vec![("profile", "dist")],
        },
    )
    .expect("global override should bind");

    assert!(plan.nodes.iter().all(|node| {
        node.params
            .iter()
            .any(|param| param.name == "profile" && param.value.as_deref() == Some("dist"))
    }));
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
fn preserves_dependency_declaration_order() {
    let compiled = compile_document(
        "group front {\n\
         a():\n             true\n\
         b() & a:\n             true\n\
         c() & b:\n             true\n\
         run() & c:\n             true\n\
         }\n\
         group back {\n\
         a():\n             true\n\
         run() & a:\n             true\n\
         }\n\
         ci() & (front.run, back.run):\n             true\n",
    );
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "ci",
            args: vec![],
            overrides: vec![],
        },
    );

    let names = plan
        .nodes
        .iter()
        .map(|node| node.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "front.a",
            "front.b",
            "front.c",
            "front.run",
            "back.a",
            "back.run",
            "ci",
        ]
    );
}

#[test]
fn lists_shared_dependency_once() {
    let compiled = compile_document(
        "prepare():\n    true\nfront() & prepare:\n    true\nback() & prepare:\n    true\nci() & (front, back):\n    true\n",
    );
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "ci",
            args: vec![],
            overrides: vec![],
        },
    );

    let names = plan
        .nodes
        .iter()
        .map(|node| node.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["prepare", "front", "back", "ci"]);
}

#[test]
fn carries_shell_and_default_params_into_plan() {
    let compiled = compile_document(
        "!version 0.4\n!shell bash\n\
         build(tag=\"v1\") shell~=pwsh:\n\
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

    assert_eq!(plan.shell, Some(ShellKind::Bash));
    assert_eq!(plan.nodes.len(), 1);
    let shell = plan.nodes[0].shell.as_ref().expect("shell should exist");
    assert_eq!(shell.kind, ShellKind::Pwsh);
    assert_eq!(shell.operator, ShellOperator::Fallback);
    assert_eq!(plan.nodes[0].params.len(), 1);
    assert_eq!(plan.nodes[0].params[0].name, "tag");
    assert_eq!(plan.nodes[0].params[0].default_value.as_deref(), Some("v1"));
}

#[test]
fn carries_exact_task_shell_assignment_into_plan() {
    let compiled = compile_document("probe() shell=powershell:\n    Write-Output ok\n");
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "probe",
            args: vec![],
            overrides: vec![],
        },
    );

    assert_eq!(plan.nodes.len(), 1);
    let shell = plan.nodes[0].shell.as_ref().expect("shell should exist");
    assert_eq!(shell.kind, ShellKind::Powershell);
    assert_eq!(shell.operator, ShellOperator::Required);
}

#[test]
fn keeps_command_blocks_as_single_plan_steps() {
    let compiled = compile_document(
        "!version 0.4\ntask():\n    | value={{value}}\n    | echo \"$value\"\n    echo done\n",
    );
    let plan = build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "task",
            args: vec![],
            overrides: vec![],
        },
    );

    assert_eq!(plan.nodes[0].steps.len(), 2);
    let only_engine::ExecutionStep::CommandBlock {
        line_count,
        interpolations,
        ..
    } = &plan.nodes[0].steps[0]
    else {
        panic!("expected a command block");
    };
    assert_eq!(*line_count, 2);
    assert_eq!(interpolations.len(), 1);
    assert_eq!(interpolations[0].name, "value");
    assert_eq!(
        plan.nodes[0].steps[0].source(),
        "value={{value}}\necho \"$value\"\n"
    );
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
fn binds_slice_parameter_from_remaining_arguments() {
    let compiled = compile_document("run(args..):\n    echo {{args}}\n");
    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "run",
            args: vec!["fetch", "https://example.invalid/repo.git", "--force"],
            overrides: vec![],
        },
    )
    .expect("plan should build");

    assert_eq!(plan.nodes.len(), 1);
    assert_eq!(plan.nodes[0].params[0].name, "args");
    assert_eq!(
        plan.nodes[0].params[0].value.as_deref(),
        Some("fetch https://example.invalid/repo.git --force")
    );
}

#[test]
fn binds_slice_parameter_after_fixed_arguments() {
    let compiled = compile_document("run(tool, args..):\n    {{tool}} {{args}}\n");
    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "run",
            args: vec!["cargo", "run", "--release"],
            overrides: vec![],
        },
    )
    .expect("plan should build");

    assert_eq!(plan.nodes[0].params[0].value.as_deref(), Some("cargo"));
    assert_eq!(
        plan.nodes[0].params[1].value.as_deref(),
        Some("run --release")
    );
}

#[test]
fn binds_empty_slice_parameter_when_no_remaining_arguments() {
    let compiled = compile_document("run(args..):\n    echo {{args}}\n");
    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "run",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect("plan should build");

    assert_eq!(plan.nodes[0].params[0].value.as_deref(), Some(""));
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

    assert_eq!(error.to_string(), "parameter '{{task}}' is required");
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
        "task 'run' has no parameter named 'other'"
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

    assert_eq!(
        error.to_string(),
        "parameter 'task' was given more than once"
    );
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
        "task 'run' accepts 1 argument, but got 2"
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

    assert_eq!(error.to_string(), "dependency loop: a -> b -> a");
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
    assert_eq!(plan.nodes[0].steps[0].source(), "echo guarded");
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
    assert_eq!(plan.nodes[0].steps[0].source(), "echo guarded-build");
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
        "helper task '_prepare' cannot be run directly"
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
fn binds_dependency_arguments() {
    let compiled = compile_document(concat!(
        "build(profile):\n    echo {{profile}}\n",
        "ci() & build(\"dev\"):\n    true\n",
    ));
    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "ci",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect("dependency argument should bind");

    assert_eq!(plan.nodes[0].name, "build");
    assert_eq!(plan.nodes[0].params[0].value.as_deref(), Some("dev"));
}

#[test]
fn uses_dependency_parameter_defaults() {
    let compiled = compile_document(concat!(
        "build(profile=\"dev\"):\n    echo {{profile}}\n",
        "ci() & build:\n    true\n",
    ));
    let plan = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "ci",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect("dependency default should bind");

    assert_eq!(plan.nodes[0].params[0].value.as_deref(), Some("dev"));
}

#[test]
fn reports_missing_dependency_parameter() {
    let compiled = compile_document(concat!(
        "build(profile):\n    echo {{profile}}\n",
        "ci() & build:\n    true\n",
    ));
    let error = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "ci",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect_err("required dependency parameter should fail planning");

    assert_eq!(
        error.to_string(),
        "dependency `build` requires parameter `profile`\nprovide it with `build(\"value\")`"
    );
}

#[test]
fn rejects_conflicting_dependency_arguments() {
    let compiled = compile_document(concat!(
        "build(profile):\n    echo {{profile}}\n",
        "ci() & (build(\"dev\"), build(\"release\")):\n    true\n",
    ));
    let error = try_build_execution_plan(
        &compiled.document,
        Invocation::Task {
            target: "ci",
            args: vec![],
            overrides: vec![],
        },
    )
    .expect_err("different bindings must not be deduplicated");

    assert_eq!(
        error.to_string(),
        "dependency `build` is called with different arguments"
    );
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
        "task 'probe' is not available on this system"
    );
}
