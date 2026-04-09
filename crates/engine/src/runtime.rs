use anstyle::{AnsiColor as TermAnsiColor, Style as TermStyle};
use std::process::ExitCode;

use crate::error::command_failed;
use crate::interpolate::interpolate;
use crate::shell::run_command;
use crate::{EngineError, ExecutionPlan};

/// Runs a pre-built execution plan.
///
/// Args:
/// plan: Dependency-expanded execution plan.
///
/// Returns:
/// Success when all execution nodes complete successfully.
pub fn run_plan(plan: &ExecutionPlan) -> Result<ExitCode, EngineError> {
    let total_tasks = plan.nodes.len();

    for (task_index, node) in plan.nodes.iter().enumerate() {
        if plan.verbose {
            eprintln!(
                "{}",
                render_task_progress(task_index + 1, total_tasks, &node.name)
            );
        }

        let total_commands = node.commands.len();
        for (index, command) in node.commands.iter().enumerate() {
            let rendered = interpolate(command, &node.params)?;
            if plan.verbose {
                eprintln!("{}", render_verbose_command(&rendered));
            }

            let shell = node
                .shell
                .as_deref()
                .or(plan.shell.as_deref())
                .unwrap_or("deno");
            let code = run_command(&rendered, &plan.working_dir, shell, node.shell_fallback)?;
            if code != ExitCode::SUCCESS {
                return Err(command_failed(
                    &node.name,
                    index + 1,
                    total_commands,
                    &rendered,
                    code,
                ));
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn render_task_progress(task_index: usize, total_tasks: usize, task_name: &str) -> String {
    let label_style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightGreen.into()))
        .bold();
    let task_style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightCyan.into()))
        .bold();

    format!(
        "{}[task {}/{}]{} {}{}{}",
        label_style.render(),
        task_index,
        total_tasks,
        label_style.render_reset(),
        task_style.render(),
        task_name,
        task_style.render_reset()
    )
}

fn render_verbose_command(command: &str) -> String {
    let prefix_style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightYellow.into()))
        .bold();
    let command_style = TermStyle::new().fg_color(Some(TermAnsiColor::BrightWhite.into()));

    format!(
        "  {}${} {}{}{}",
        prefix_style.render(),
        prefix_style.render_reset(),
        command_style.render(),
        command,
        command_style.render_reset()
    )
}
