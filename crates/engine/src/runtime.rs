use anstyle::{AnsiColor as TermAnsiColor, Style as TermStyle};
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
    let mut task_index = 0usize;

    while task_index < total_tasks {
        let stage = plan.nodes[task_index].stage;
        let stage_start = task_index;
        while task_index < total_tasks && plan.nodes[task_index].stage == stage {
            task_index += 1;
        }
        let stage_nodes = &plan.nodes[stage_start..task_index];

        if !options.quiet {
            for (offset, node) in stage_nodes.iter().enumerate() {
                eprintln!(
                    "{}",
                    render_task_progress(stage_start + offset + 1, total_tasks, &node.name)
                );
            }
        }

        if stage_nodes.len() == 1 {
            run_node_inherit(
                &stage_nodes[0],
                &plan.working_dir,
                plan.shell.as_ref(),
                options.quiet,
            )?;
        } else {
            execute_stage(
                stage_nodes,
                &plan.working_dir,
                plan.shell.as_ref(),
                options.quiet,
            )?;
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn execute_stage(
    stage_nodes: &[ExecutionNode],
    working_dir: &std::path::Path,
    default_shell: Option<&ShellKind>,
    quiet: bool,
) -> Result<(), EngineError> {
    let stage_len = stage_nodes.len();
    let (event_tx, event_rx) = mpsc::channel::<StageEvent>();

    thread::scope(|scope| {
        let mut handles = Vec::new();

        for (index, node) in stage_nodes.iter().cloned().enumerate() {
            let working_dir = working_dir.to_path_buf();
            let shell = default_shell.cloned();
            let event_tx = event_tx.clone();
            handles.push(
                scope.spawn(move || run_node(index, &node, &working_dir, shell.as_ref(), event_tx)),
            );
        }
        drop(event_tx);

        run_ordered_output_event_loop(&event_rx, stage_len, quiet)?;

        collect_thread_results(handles)
    })
}

fn run_ordered_output_event_loop(
    event_rx: &mpsc::Receiver<StageEvent>,
    stage_len: usize,
    quiet: bool,
) -> Result<(), EngineError> {
    let mut buffers = vec![Vec::<OutputChunk>::new(); stage_len];
    let mut finished = vec![false; stage_len];
    let mut results = (0..stage_len)
        .map(|_| None)
        .collect::<Vec<Option<TaskResult>>>();
    let mut task_errors = (0..stage_len)
        .map(|_| None)
        .collect::<Vec<Option<EngineError>>>();
    let mut current_index = 0usize;
    let mut finished_count = 0usize;
    let mut first_error = None;

    while finished_count < stage_len {
        match event_rx.recv() {
            Ok(StageEvent::Output { task_index, chunk }) => {
                if task_index == current_index {
                    print_output_chunk(&chunk)?;
                } else {
                    buffers[task_index].push(chunk);
                }
            }
            Ok(StageEvent::Finished {
                task_index,
                error,
                result,
            }) => {
                finished[task_index] = true;
                task_errors[task_index] = error;
                results[task_index] = result;
                finished_count += 1;

                while current_index < stage_len {
                    flush_task_buffer(&mut buffers[current_index])?;
                    if !finished[current_index] {
                        break;
                    }
                    let mut task_error = task_errors[current_index].take();
                    match results[current_index].take() {
                        Some(TaskResult::Pass(message)) if !quiet => print_task_message(&message)?,
                        Some(TaskResult::Fail(message)) => {
                            if let Some(error) = task_error.take() {
                                task_error = Some(task_failure(error, message));
                            }
                        }
                        Some(TaskResult::Pass(_)) | None => {}
                    }
                    if first_error.is_none() {
                        first_error = task_error;
                    }
                    current_index += 1;
                }
            }
            Err(_) => break,
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn collect_thread_results(
    handles: Vec<thread::ScopedJoinHandle<'_, Result<(), EngineError>>>,
) -> Result<(), EngineError> {
    let mut first_error = None;
    for handle in handles {
        match handle.join() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn run_node(
    task_index: usize,
    node: &ExecutionNode,
    working_dir: &std::path::Path,
    default_shell: Option<&ShellKind>,
    event_tx: mpsc::Sender<StageEvent>,
) -> Result<(), EngineError> {
    let (output_tx, output_rx) = mpsc::channel::<OutputChunk>();
    let forwarder = thread::spawn({
        let event_tx = event_tx.clone();
        move || -> Result<(), EngineError> {
            while let Ok(chunk) = output_rx.recv() {
                event_tx
                    .send(StageEvent::Output { task_index, chunk })
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
        .send(StageEvent::Finished {
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
    quiet: bool,
) -> Result<(), EngineError> {
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
        Ok(()) => {
            if !quiet && let Some(message) = task_result_message(node, true)? {
                print_task_message(&message)?;
            }
            Ok(())
        }
        Err(error) => {
            if let Some(message) = task_result_message(node, false)? {
                Err(task_failure(error, message))
            } else {
                Err(error)
            }
        }
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
enum StageEvent {
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
