use anstyle::{AnsiColor as TermAnsiColor, Style as TermStyle};
use std::io::{self, Write};
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;

use crate::error::command_failed;
use crate::interpolate::interpolate;
use crate::process::{OutputChunk, OutputStream};
use crate::shell::run_command;
use crate::{EngineError, ExecutionNode, ExecutionPlan};

/// Runs a pre-built execution plan.
///
/// Args:
/// plan: Dependency-expanded execution plan.
///
/// Returns:
/// Success when all execution nodes complete successfully.
pub fn run_plan(plan: &ExecutionPlan) -> Result<ExitCode, EngineError> {
    let total_tasks = plan.nodes.len();
    let mut task_index = 0usize;

    while task_index < total_tasks {
        let stage = plan.nodes[task_index].stage;
        let stage_start = task_index;
        while task_index < total_tasks && plan.nodes[task_index].stage == stage {
            task_index += 1;
        }
        let stage_nodes = &plan.nodes[stage_start..task_index];

        for (offset, node) in stage_nodes.iter().enumerate() {
            eprintln!(
                "{}",
                render_task_progress(stage_start + offset + 1, total_tasks, &node.name)
            );
        }

        execute_stage(
            stage_nodes,
            &plan.working_dir,
            plan.shell.as_deref(),
            plan.echo,
        )?;
    }

    if !plan.echo {
        eprintln!("{}", render_status("Success", TermAnsiColor::BrightGreen));
    }

    Ok(ExitCode::SUCCESS)
}

fn execute_stage(
    stage_nodes: &[ExecutionNode],
    working_dir: &std::path::Path,
    default_shell: Option<&str>,
    echo: bool,
) -> Result<(), EngineError> {
    if !echo {
        return execute_quiet_stage(stage_nodes, working_dir, default_shell);
    }

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
                    if !echo && matches!(chunk.stream, OutputStream::Stdout) {
                        continue;
                    }
                    if task_index == current_index {
                        print_output_chunk(&stage_nodes[task_index].name, &chunk)?;
                    } else {
                        buffers[task_index].push(chunk);
                    }
                }
                Ok(StageEvent::Finished { task_index, error }) => {
                    finished[task_index] = true;
                    task_errors[task_index] = error;
                    finished_count += 1;

                    while current_index < stage_len {
                        flush_task_buffer(
                            &stage_nodes[current_index].name,
                            &mut buffers[current_index],
                        )?;
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
    })
}

fn execute_quiet_stage(
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

        let mut stderr_buffers = vec![Vec::<OutputChunk>::new(); stage_len];
        let mut task_errors = (0..stage_len)
            .map(|_| None)
            .collect::<Vec<Option<EngineError>>>();
        let mut finished_count = 0usize;
        let mut first_error = None;

        while finished_count < stage_len {
            match event_rx.recv() {
                Ok(StageEvent::Output { task_index, chunk }) => {
                    if matches!(chunk.stream, OutputStream::Stderr) {
                        stderr_buffers[task_index].push(chunk);
                    }
                }
                Ok(StageEvent::Finished { task_index, error }) => {
                    task_errors[task_index] = error;
                    finished_count += 1;
                }
                Err(_) => break,
            }
        }

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

        if first_error.is_none() {
            first_error = task_errors.into_iter().flatten().next();
        }

        if let Some(error) = first_error {
            for (index, buffer) in stderr_buffers.iter_mut().enumerate() {
                flush_task_buffer(&stage_nodes[index].name, buffer)?;
            }
            eprintln!("{}", render_status("Fail", TermAnsiColor::BrightRed));
            return Err(error);
        }

        Ok(())
    })
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

    let total_commands = node.commands.len();
    let mut task_error = None;

    for (index, command) in node.commands.iter().enumerate() {
        let rendered = match interpolate(command, &node.params) {
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
                task_error = Some(error);
                break;
            }
        };

        if code != ExitCode::SUCCESS {
            task_error = Some(command_failed(
                &node.name,
                index + 1,
                total_commands,
                &rendered,
                code,
            ));
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

fn flush_task_buffer(task_name: &str, buffer: &mut Vec<OutputChunk>) -> Result<(), EngineError> {
    for chunk in buffer.drain(..) {
        print_output_chunk(task_name, &chunk)?;
    }
    Ok(())
}

fn print_output_chunk(task_name: &str, chunk: &OutputChunk) -> Result<(), EngineError> {
    let prefix_style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightCyan.into()))
        .bold();
    let prefix = format!(
        "{}[{}]{} ",
        prefix_style.render(),
        task_name,
        prefix_style.render_reset()
    );

    match chunk.stream {
        OutputStream::Stdout => write_prefixed(prefix.as_str(), &chunk.text, io::stdout()),
        OutputStream::Stderr => write_prefixed(prefix.as_str(), &chunk.text, io::stderr()),
    }
}

fn write_prefixed(prefix: &str, content: &str, mut writer: impl Write) -> Result<(), EngineError> {
    for segment in content.split_inclusive('\n') {
        if segment.is_empty() {
            continue;
        }
        write!(writer, "{prefix}{segment}").map_err(|error| {
            EngineError::Runtime(format!("failed to write task output: {error}"))
        })?;
    }

    if !content.is_empty() && !content.ends_with('\n') {
        writeln!(writer).map_err(|error| {
            EngineError::Runtime(format!("failed to write task output: {error}"))
        })?;
    }

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

fn render_status(label: &str, color: TermAnsiColor) -> String {
    let style = TermStyle::new().fg_color(Some(color.into())).bold();
    format!("{}{}{}", style.render(), label, style.render_reset())
}
