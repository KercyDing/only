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
    for node in &plan.nodes {
        if plan.verbose {
            println!("[task] {}", node.qualified_name);
        }

        for command in &node.commands {
            let rendered = interpolate(command, &node.parameters)?;

            if plan.verbose {
                println!("  $ {}", rendered);
            }

            let code = run_command(&rendered)?;
            if code != ExitCode::SUCCESS {
                return Err(command_failed(&node.qualified_name, &rendered, code));
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}
