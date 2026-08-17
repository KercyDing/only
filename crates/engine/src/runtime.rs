use anstyle::{AnsiColor as TermAnsiColor, Style as TermStyle};
use std::io::{self, Write};
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;

use crate::error::{command_block_failed, command_block_start_failed, command_failed};
use crate::interpolate::interpolate;
use crate::process::{OutputChunk, OutputStream};
use crate::shell::{run_command, run_command_inherit};
use crate::{EngineError, ExecutionNode, ExecutionPlan};

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
            run_node_inherit(&stage_nodes[0], &plan.working_dir, plan.shell.as_deref())?;
        } else {
            execute_stage(stage_nodes, &plan.working_dir, plan.shell.as_deref())?;
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn execute_stage(
    stage_nodes: &[ExecutionNode],
    working_dir: &std::path::Path,
    default_shell: Option<&str>,
) -> Result<(), EngineError> {
    let stage_len = stage_nodes.len();
    let (event_tx, event_rx) = mpsc::channel::<StageEvent>();

    thread::scope(|scope| {
        let mut handles = Vec::new();

        for (index, node) in stage_nodes.iter().cloned().enumerate() {
            let working_dir = working_dir.to_path_buf();
            let shell = default_shell.map(str::to_string);
            let event_tx = event_tx.clone();
            handles.push(
                scope.spawn(move || {
                    run_node(index, &node, &working_dir, shell.as_deref(), event_tx)
                }),
            );
        }
        drop(event_tx);

        run_ordered_output_event_loop(&event_rx, stage_len)?;

        collect_thread_results(handles)
    })
}

fn run_ordered_output_event_loop(
    event_rx: &mpsc::Receiver<StageEvent>,
    stage_len: usize,
) -> Result<(), EngineError> {
    let mut buffers = vec![Vec::<OutputChunk>::new(); stage_len];
    let mut finished = vec![false; stage_len];
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
            Ok(StageEvent::Finished { task_index, error }) => {
                finished[task_index] = true;
                task_errors[task_index] = error;
                finished_count += 1;

                while current_index < stage_len {
                    flush_task_buffer(&mut buffers[current_index])?;
                    if !finished[current_index] {
                        break;
                    }
                    if first_error.is_none() {
                        first_error = task_errors[current_index].take();
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
    default_shell: Option<&str>,
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
        let rendered = match interpolate(step.source(), &node.params) {
            Ok(rendered) => rendered,
            Err(error) => {
                task_error = Some(error);
                break;
            }
        };

        let shell = node.shell.as_deref().or(default_shell).unwrap_or("deno");
        let code = match run_command(
            &rendered,
            working_dir,
            shell,
            node.shell_fallback,
            output_tx.clone(),
        ) {
            Ok(code) => code,
            Err(error) => {
                task_error = Some(if step.is_block() {
                    command_block_start_failed(shell, error)
                } else {
                    error
                });
                break;
            }
        };

        if code != ExitCode::SUCCESS {
            task_error = Some(if step.is_block() {
                command_block_failed(&node.name, index + 1, total_steps, code)
            } else {
                command_failed(&node.name, index + 1, total_steps, &rendered, code)
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

    event_tx
        .send(StageEvent::Finished {
            task_index,
            error: task_error,
        })
        .map_err(|_| EngineError::Runtime("failed to finalize task output".to_string()))?;
    Ok(())
}

fn run_node_inherit(
    node: &ExecutionNode,
    working_dir: &std::path::Path,
    default_shell: Option<&str>,
) -> Result<(), EngineError> {
    let total_steps = node.steps.len();

    for (index, step) in node.steps.iter().enumerate() {
        let rendered = interpolate(step.source(), &node.params)?;
        let shell = node.shell.as_deref().or(default_shell).unwrap_or("deno");
        let code = run_command_inherit(&rendered, working_dir, shell, node.shell_fallback)
            .map_err(|error| {
                if step.is_block() {
                    command_block_start_failed(shell, error)
                } else {
                    error
                }
            })?;

        if code != ExitCode::SUCCESS {
            return Err(if step.is_block() {
                command_block_failed(&node.name, index + 1, total_steps, code)
            } else {
                command_failed(&node.name, index + 1, total_steps, &rendered, code)
            });
        }
    }

    Ok(())
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
    },
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
