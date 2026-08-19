use anstyle::{AnsiColor as TermAnsiColor, Style as TermStyle};
use std::collections::VecDeque;
use std::io::{self, Write};
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;

use only_semantic::{ShellKind, ShellSelection};

use crate::error::{
    command_block_failed, command_block_start_failed, command_failed, task_failure,
};
use crate::interpolate::{interpolate, interpolate_with_parts};
use crate::process::{OutputChunk, OutputStream};
use crate::shell::{run_command, run_command_inherit};
use crate::{EngineError, ExecutionNode, ExecutionPlan, ExecutionStep, PlanParam};

/// Runs a pre-built execution plan.
///
/// Args:
/// plan: Dependency-expanded execution plan.
///
/// Returns:
/// Success when all execution nodes complete successfully.
pub fn run_plan(plan: &ExecutionPlan) -> Result<ExitCode, EngineError> {
    run_plan_with_options(plan, RuntimeOptions::default())
}

/// Runtime options that affect only the task host, not command semantics.
///
/// Args:
/// None.
///
/// Returns:
/// Host display options used while executing an already-built plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RuntimeOptions {
    pub quiet: bool,
    pub max_parallelism: Option<usize>,
}

/// Runs a pre-built execution plan with host display options.
///
/// Args:
/// plan: Dependency-expanded execution plan.
/// options: Host-side display options.
///
/// Returns:
/// Success when all execution nodes complete successfully.
pub fn run_plan_with_options(
    plan: &ExecutionPlan,
    options: RuntimeOptions,
) -> Result<ExitCode, EngineError> {
    let total_tasks = plan.nodes.len();
    if total_tasks == 0 {
        return Ok(ExitCode::SUCCESS);
    }
    if plan.successors.len() != total_tasks {
        return Err(EngineError::Runtime(
            "execution plan has invalid dependency edges".to_string(),
        ));
    }

    let mut indegree = vec![0usize; total_tasks];
    for successors in &plan.successors {
        for &successor in successors {
            if successor >= total_tasks {
                return Err(EngineError::Runtime(
                    "execution plan has invalid dependency edge".to_string(),
                ));
            }
            indegree[successor] += 1;
        }
    }
    let mut ready = (0..total_tasks)
        .filter(|&index| indegree[index] == 0)
        .collect::<VecDeque<_>>();
    let limit = options.max_parallelism.unwrap_or(usize::MAX).max(1);
    let (event_tx, event_rx) = mpsc::channel::<ExecutionEvent>();
    let mut buffers = vec![Vec::<OutputChunk>::new(); total_tasks];
    let mut progress_printed = vec![false; total_tasks];
    let mut finished = vec![false; total_tasks];
    let mut results = (0..total_tasks)
        .map(|_| None)
        .collect::<Vec<Option<TaskResult>>>();
    let mut task_errors = (0..total_tasks)
        .map(|_| None)
        .collect::<Vec<Option<EngineError>>>();
    let mut current_index = 0usize;
    let mut finished_count = 0usize;
    let mut active_count = 0usize;
    let mut failure_seen = false;

    thread::scope(|scope| {
        let mut handles = Vec::new();
        while finished_count < total_tasks && (!failure_seen || active_count > 0) {
            while !failure_seen && active_count < limit {
                let Some(index) = ready.pop_front() else {
                    break;
                };
                let node = plan.nodes[index].clone();
                let working_dir = plan.working_dir.clone();
                let shell = plan.shell.clone();
                let event_tx = event_tx.clone();
                let direct = active_count == 0 && (limit == 1 || ready.is_empty());
                handles.push(scope.spawn(move || {
                    event_tx
                        .send(ExecutionEvent::Started { task_index: index })
                        .map_err(|_| {
                            EngineError::Runtime("failed to start task presentation".to_string())
                        })?;
                    if direct {
                        let (error, result) =
                            match run_node_inherit(&node, &working_dir, shell.as_ref()) {
                                Ok(result) => result,
                                Err(error) => (Some(error), None),
                            };
                        event_tx
                            .send(ExecutionEvent::Finished {
                                task_index: index,
                                error,
                                result,
                            })
                            .map_err(|_| {
                                EngineError::Runtime("failed to finalize task output".to_string())
                            })
                    } else {
                        run_node(index, &node, &working_dir, shell.as_ref(), event_tx)
                    }
                }));
                active_count += 1;
            }
            let event = event_rx.recv().map_err(|_| {
                EngineError::Runtime("execution event channel closed unexpectedly".to_string())
            })?;
            match event {
                ExecutionEvent::Started { task_index } => {
                    if task_index == current_index {
                        print_task_progress(
                            task_index,
                            total_tasks,
                            plan,
                            &mut progress_printed,
                            options.quiet,
                        )?;
                    }
                }
                ExecutionEvent::Output { task_index, chunk } => {
                    if task_index == current_index {
                        print_task_progress(
                            task_index,
                            total_tasks,
                            plan,
                            &mut progress_printed,
                            options.quiet,
                        )?;
                        print_output_chunk(&chunk)?;
                    } else {
                        buffers[task_index].push(chunk);
                    }
                }
                ExecutionEvent::Finished {
                    task_index,
                    error,
                    result,
                } => {
                    active_count -= 1;
                    finished[task_index] = true;
                    task_errors[task_index] = error;
                    results[task_index] = result;
                    finished_count += 1;
                    if task_errors[task_index].is_some() {
                        failure_seen = true;
                    } else if !failure_seen {
                        for &successor in &plan.successors[task_index] {
                            indegree[successor] -= 1;
                            if indegree[successor] == 0 {
                                ready.push_back(successor);
                            }
                        }
                    }
                }
            }

            while current_index < total_tasks && finished[current_index] {
                print_task_progress(
                    current_index,
                    total_tasks,
                    plan,
                    &mut progress_printed,
                    options.quiet,
                )?;
                flush_task_buffer(&mut buffers[current_index])?;
                let mut task_error = task_errors[current_index].take();
                match results[current_index].take() {
                    Some(TaskResult::Pass(message)) if !options.quiet => {
                        print_task_message(&message)?
                    }
                    Some(TaskResult::Fail(message)) => {
                        if let Some(error) = task_error.take() {
                            task_error = Some(task_failure(error, message));
                        }
                    }
                    Some(TaskResult::Pass(_)) | None => {}
                }
                task_errors[current_index] = task_error;
                current_index += 1;
            }
        }
        drop(event_tx);
        for handle in handles {
            if let Err(payload) = handle.join() {
                std::panic::resume_unwind(payload);
            }
        }
        Ok::<(), EngineError>(())
    })?;

    task_errors
        .into_iter()
        .find_map(|error| error)
        .map_or(Ok(ExitCode::SUCCESS), Err)
}

fn run_node(
    task_index: usize,
    node: &ExecutionNode,
    working_dir: &std::path::Path,
    default_shell: Option<&ShellKind>,
    event_tx: mpsc::Sender<ExecutionEvent>,
) -> Result<(), EngineError> {
    let (output_tx, output_rx) = mpsc::channel::<OutputChunk>();
    let forwarder = thread::spawn({
        let event_tx = event_tx.clone();
        move || -> Result<(), EngineError> {
            while let Ok(chunk) = output_rx.recv() {
                event_tx
                    .send(ExecutionEvent::Output { task_index, chunk })
                    .map_err(|_| {
                        EngineError::Runtime("failed to forward task output".to_string())
                    })?;
            }
            Ok(())
        }
    });

    let total_steps = node.steps.len();
    let mut task_error = None;

    for (index, step) in node.steps.iter().enumerate() {
        let rendered = match interpolate_step(step, &node.params) {
            Ok(rendered) => rendered,
            Err(error) => {
                task_error = Some(error);
                break;
            }
        };

        let shell = select_shell(node, default_shell);
        let status = match run_command(&rendered, working_dir, &shell, output_tx.clone()) {
            Ok(status) => status,
            Err(error) => {
                task_error = Some(if step.is_block() {
                    command_block_start_failed(shell.kind.as_str(), error)
                } else {
                    error
                });
                break;
            }
        };

        if let Some(reason) = status.failure_reason() {
            task_error = Some(if step.is_block() {
                command_block_failed(&node.name, index + 1, total_steps, reason)
            } else {
                command_failed(&node.name, index + 1, total_steps, &rendered, reason)
            });
            break;
        }
    }

    drop(output_tx);
    match forwarder.join() {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            if task_error.is_none() {
                task_error = Some(error);
            }
        }
        Err(_) => {
            if task_error.is_none() {
                task_error = Some(EngineError::Runtime(
                    "task output forwarder thread panicked".to_string(),
                ));
            }
        }
    }

    let result = match task_result_message(node, task_error.is_none()) {
        Ok(message) => message.map(|message| {
            if task_error.is_none() {
                TaskResult::Pass(message)
            } else {
                TaskResult::Fail(message)
            }
        }),
        Err(error) => {
            if task_error.is_none() {
                task_error = Some(error);
            }
            None
        }
    };

    event_tx
        .send(ExecutionEvent::Finished {
            task_index,
            error: task_error,
            result,
        })
        .map_err(|_| EngineError::Runtime("failed to finalize task output".to_string()))?;
    Ok(())
}

fn run_node_inherit(
    node: &ExecutionNode,
    working_dir: &std::path::Path,
    default_shell: Option<&ShellKind>,
) -> Result<(Option<EngineError>, Option<TaskResult>), EngineError> {
    let total_steps = node.steps.len();
    let execution = (|| {
        for (index, step) in node.steps.iter().enumerate() {
            let rendered = interpolate_step(step, &node.params)?;
            let shell = select_shell(node, default_shell);
            let status = run_command_inherit(&rendered, working_dir, &shell).map_err(|error| {
                if step.is_block() {
                    command_block_start_failed(shell.kind.as_str(), error)
                } else {
                    error
                }
            })?;

            if let Some(reason) = status.failure_reason() {
                return Err(if step.is_block() {
                    command_block_failed(&node.name, index + 1, total_steps, reason)
                } else {
                    command_failed(&node.name, index + 1, total_steps, &rendered, reason)
                });
            }
        }
        Ok::<(), EngineError>(())
    })();

    match execution {
        Ok(()) => Ok((None, task_result_message(node, true)?.map(TaskResult::Pass))),
        Err(error) => Ok((
            Some(error),
            task_result_message(node, false)?.map(TaskResult::Fail),
        )),
    }
}

fn interpolate_step(step: &ExecutionStep, params: &[PlanParam]) -> Result<String, EngineError> {
    match step {
        ExecutionStep::Command {
            source,
            interpolations,
        }
        | ExecutionStep::CommandBlock {
            source,
            interpolations,
            ..
        } => interpolate_with_parts(source, interpolations, params),
    }
}

fn select_shell(node: &ExecutionNode, default_shell: Option<&ShellKind>) -> ShellSelection {
    node.shell.clone().unwrap_or_else(|| {
        ShellSelection::required(default_shell.cloned().unwrap_or(ShellKind::Deno))
    })
}

#[derive(Debug)]
enum ExecutionEvent {
    Started {
        task_index: usize,
    },
    Output {
        task_index: usize,
        chunk: OutputChunk,
    },
    Finished {
        task_index: usize,
        error: Option<EngineError>,
        result: Option<TaskResult>,
    },
}

fn print_task_progress(
    task_index: usize,
    total_tasks: usize,
    plan: &ExecutionPlan,
    progress_printed: &mut [bool],
    quiet: bool,
) -> Result<(), EngineError> {
    if quiet || progress_printed[task_index] {
        return Ok(());
    }

    eprintln!(
        "{}",
        render_task_progress(task_index + 1, total_tasks, &plan.nodes[task_index].name)
    );
    progress_printed[task_index] = true;
    Ok(())
}

#[derive(Debug)]
enum TaskResult {
    Pass(String),
    Fail(String),
}

fn task_result_message(node: &ExecutionNode, success: bool) -> Result<Option<String>, EngineError> {
    let source = if success {
        node.pass.as_deref()
    } else {
        node.fail.as_deref()
    };
    source
        .map(|text| interpolate(text, &node.result_params))
        .transpose()
}

fn print_task_message(message: &str) -> Result<(), EngineError> {
    let style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightGreen.into()))
        .bold();
    let rendered = format!("{}{}{}\n", style.render(), message, style.render_reset());
    write_output(&rendered, io::stderr())
}

fn flush_task_buffer(buffer: &mut Vec<OutputChunk>) -> Result<(), EngineError> {
    for chunk in buffer.drain(..) {
        print_output_chunk(&chunk)?;
    }
    Ok(())
}

fn print_output_chunk(chunk: &OutputChunk) -> Result<(), EngineError> {
    match chunk.stream {
        OutputStream::Stdout => write_output(&chunk.text, io::stdout()),
        OutputStream::Stderr => write_output(&chunk.text, io::stderr()),
    }
}

fn write_output(content: &str, mut writer: impl Write) -> Result<(), EngineError> {
    write!(writer, "{content}")
        .map_err(|error| EngineError::Runtime(format!("failed to write task output: {error}")))?;

    writer
        .flush()
        .map_err(|error| EngineError::Runtime(format!("failed to flush task output: {error}")))?;
    Ok(())
}

fn render_task_progress(task_index: usize, total_tasks: usize, task_name: &str) -> String {
    let label_style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightGreen.into()))
        .bold();
    let task_style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightCyan.into()))
        .bold();

    format!(
        "{}[{}/{}]{} {}{}{}",
        label_style.render(),
        task_index,
        total_tasks,
        label_style.render_reset(),
        task_style.render(),
        task_name,
        task_style.render_reset()
    )
}
