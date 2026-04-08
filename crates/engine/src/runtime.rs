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
    for node in &plan.nodes {
        let total_commands = node.commands.len();
        for (index, command) in node.commands.iter().enumerate() {
            let rendered = interpolate(command, &node.params)?;
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
