use anstyle::{AnsiColor, Style};
use std::path::Path;
use std::process::ExitCode;

use crate::diagnostic::error::Result;
use crate::planner::ExecutionPlan;

use super::interpolate::interpolate;
use super::process::{command_failed, run_command};

/// Executes an ordered execution plan.
///
/// Args:
/// plan: Resolved execution plan.
///
/// Returns:
/// Final process exit code. Execution stops on the first failure.
pub fn run_plan(plan: &ExecutionPlan) -> Result<ExitCode> {
    let current_dir = std::env::current_dir().map_err(crate::diagnostic::error::OnlyError::cwd)?;

    for node in &plan.nodes {
        if plan.verbose {
            println!(
                "{}",
                render_task_banner(&node.qualified_name, &current_dir, &plan.working_dir)
            );
        }

        let total_commands = node.commands.len();
        for (index, command) in node.commands.iter().enumerate() {
            let rendered = interpolate(command, &node.parameters)?;

            if plan.verbose {
                println!(
                    "{}",
                    render_command_line(index + 1, total_commands, &rendered)
                );
            }

            let code = run_command(
                &rendered,
                &plan.working_dir,
                node.shell.unwrap_or(plan.shell),
                node.shell_fallback,
            )?;
            if code != ExitCode::SUCCESS {
                return Err(command_failed(
                    &node.qualified_name,
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

fn render_task_banner(task_name: &str, current_dir: &Path, working_dir: &Path) -> String {
    let label_style = Style::new()
        .fg_color(Some(AnsiColor::BrightGreen.into()))
        .bold();
    let task_style = Style::new()
        .fg_color(Some(AnsiColor::BrightCyan.into()))
        .bold();
    let path_style = Style::new().fg_color(Some(AnsiColor::BrightYellow.into()));

    if current_dir == working_dir {
        return format!(
            "{}[task]{} {}{}{}",
            label_style.render(),
            label_style.render_reset(),
            task_style.render(),
            task_name,
            task_style.render_reset()
        );
    }

    format!(
        "{}[task]{} {}{}{} {}(at {}){}",
        label_style.render(),
        label_style.render_reset(),
        task_style.render(),
        task_name,
        task_style.render_reset(),
        path_style.render(),
        working_dir.display(),
        path_style.render_reset()
    )
}

fn render_command_line(index: usize, total: usize, command: &str) -> String {
    let prefix_style = Style::new()
        .fg_color(Some(AnsiColor::BrightGreen.into()))
        .bold();
    format!(
        "  {}[{index}/{total}]{} {}",
        prefix_style.render(),
        prefix_style.render_reset(),
        command
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{render_command_line, render_task_banner};

    #[test]
    fn omits_working_dir_when_it_matches_current_dir() {
        let path = Path::new("/tmp/project");
        let banner = render_task_banner("build", path, path);
        assert!(banner.contains("[task]"));
        assert!(banner.contains("build"));
        assert!(!banner.contains("(at /tmp/project)"));
    }

    #[test]
    fn shows_working_dir_when_it_differs_from_current_dir() {
        let banner = render_task_banner(
            "build",
            Path::new("/tmp/project/src"),
            Path::new("/tmp/project"),
        );
        assert!(banner.contains("[task]"));
        assert!(banner.contains("build"));
        assert!(banner.contains("(at /tmp/project)"));
    }

    #[test]
    fn highlights_command_prefix() {
        let rendered = render_command_line(1, 3, "cargo test");
        assert!(rendered.contains("[1/3]"));
        assert!(rendered.contains("cargo test"));
    }
}
