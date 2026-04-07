use std::process::ExitCode;

use crate::diagnostic::error::Result;
use crate::planner::ExecutionPlan;

use super::process::run_command;

/// Executes an ordered execution plan.
///
/// Args:
/// plan: Resolved execution plan.
///
/// Returns:
/// Final process exit code. Execution stops on the first failure.
pub fn run_plan(plan: &ExecutionPlan) -> Result<ExitCode> {
    for node in &plan.nodes {
        for command in &node.commands {
            if plan.verbose {
                println!("{}", command);
            }

            let code = run_command(command)?;
            if code != ExitCode::SUCCESS {
                return Ok(code);
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}
