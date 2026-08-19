use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{IsTerminal, Read};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

use crate::EngineError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandStatus {
    Success,
    Failed(String),
}

impl CommandStatus {
    pub(crate) fn from_code(code: i32) -> Self {
        if code == 0 {
            Self::Success
        } else {
            Self::Failed(platform_exit_reason(code))
        }
    }

    pub(crate) fn failure_reason(&self) -> Option<&str> {
        match self {
            Self::Success => None,
            Self::Failed(reason) => Some(reason),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputChunk {
    pub stream: OutputStream,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerminalBackend {
    Pipe,
    Pty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerminalContext {
    pub backend: TerminalBackend,
}

impl TerminalContext {
    #[cfg(test)]
    fn pipe() -> Self {
        Self {
            backend: TerminalBackend::Pipe,
        }
    }

    #[cfg(test)]
    fn pty() -> Self {
        Self {
            backend: TerminalBackend::Pty,
        }
    }
}

pub(crate) fn terminal_context() -> TerminalContext {
    TerminalContext {
        backend: if std::io::stdin().is_terminal()
            && std::io::stdout().is_terminal()
            && std::io::stderr().is_terminal()
            && std::env::var_os("CI").is_none()
        {
            TerminalBackend::Pty
        } else {
            TerminalBackend::Pipe
        },
    }
}

pub(crate) fn run_with_system_shell(
    program: &str,
    arg: &str,
    command: &str,
    working_dir: &Path,
    output: Sender<OutputChunk>,
    terminal: TerminalContext,
) -> Result<CommandStatus, EngineError> {
    if terminal.backend == TerminalBackend::Pty
        && let Some(status) =
            run_with_system_shell_pty(program, arg, command, working_dir, &output)?
    {
        return Ok(status);
    }

    run_with_system_shell_pipe(program, arg, command, working_dir, output)
}

fn run_with_system_shell_pipe(
    program: &str,
    arg: &str,
    command: &str,
    working_dir: &Path,
    output: Sender<OutputChunk>,
) -> Result<CommandStatus, EngineError> {
    let mut process = Command::new(program);
    process
        .current_dir(working_dir)
        .arg(arg)
        .arg(command)
        .envs(build_command_env())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = process.spawn().map_err(|source| EngineError::Io {
        message: "failed to start shell command",
        path: program.to_string(),
        source,
    })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        EngineError::Runtime(format!("failed to capture stdout for shell '{program}'"))
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        EngineError::Runtime(format!("failed to capture stderr for shell '{program}'"))
    })?;

    let stdout_handle = spawn_output_reader(stdout, OutputStream::Stdout, output.clone());
    let stderr_handle = spawn_output_reader(stderr, OutputStream::Stderr, output);
    let status = child.wait().map_err(|source| EngineError::Io {
        message: "failed to wait for shell command",
        path: program.to_string(),
        source,
    })?;
    join_output_reader(stdout_handle)?;
    join_output_reader(stderr_handle)?;

    Ok(command_status(status))
}

fn run_with_system_shell_pty(
    program: &str,
    arg: &str,
    command: &str,
    working_dir: &Path,
    output: &Sender<OutputChunk>,
) -> Result<Option<CommandStatus>, EngineError> {
    let pty_system = portable_pty::native_pty_system();
    let pair = match pty_system.openpty(portable_pty::PtySize::default()) {
        Ok(pair) => pair,
        Err(_) => return Ok(None),
    };
    // All fallible setup happens before spawning. A fallback after the child
    // starts would execute the command a second time through the Pipe backend.
    let reader = match pair.master.try_clone_reader() {
        Ok(reader) => reader,
        Err(_) => return Ok(None),
    };
    let closed_input = match pair.master.take_writer() {
        Ok(writer) => writer,
        Err(_) => return Ok(None),
    };

    let mut builder = portable_pty::CommandBuilder::new(program);
    builder.set_controlling_tty(true);
    builder.arg(arg);
    builder.arg(command);
    builder.cwd(working_dir);
    for (name, value) in build_command_env() {
        builder.env(name, value);
    }
    let mut child = match pair.slave.spawn_command(builder) {
        Ok(child) => child,
        Err(_) => return Ok(None),
    };
    // Keep only the master side after spawning. Holding the parent slave open
    // can delay the EOF that terminates the output reader.
    drop(pair.slave);
    drop(closed_input);
    let reader_handle = spawn_output_reader(reader, OutputStream::Stdout, output.clone());
    let status = child.wait().map_err(|error| {
        EngineError::Runtime(format!("failed to wait for PTY command: {error}"))
    })?;
    join_output_reader(reader_handle)?;

    let status = if status.success() {
        CommandStatus::Success
    } else if let Some(signal) = status.signal() {
        CommandStatus::Failed(format!("pty_signal({signal})"))
    } else {
        CommandStatus::from_code(status.exit_code() as i32)
    };
    Ok(Some(status))
}

pub(crate) fn run_with_system_shell_inherit(
    program: &str,
    arg: &str,
    command: &str,
    working_dir: &Path,
) -> Result<CommandStatus, EngineError> {
    let mut process = Command::new(program);
    process
        .current_dir(working_dir)
        .arg(arg)
        .arg(command)
        .envs(build_command_env())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = process.status().map_err(|source| EngineError::Io {
        message: "failed to run shell command",
        path: program.to_string(),
        source,
    })?;

    Ok(command_status(status))
}

pub(crate) fn build_command_env() -> HashMap<OsString, OsString> {
    let mut env_vars = std::env::vars_os().collect::<HashMap<_, _>>();
    env_vars
        .entry(OsString::from("INIT_CWD"))
        .or_insert_with(|| std::env::current_dir().unwrap_or_default().into_os_string());
    env_vars
}

pub(crate) fn spawn_output_reader<R>(
    mut reader: R,
    stream: OutputStream,
    output: Sender<OutputChunk>,
) -> thread::JoinHandle<Result<(), EngineError>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = reader.read(&mut buffer).map_err(|error| {
                EngineError::Runtime(format!("failed to read task output: {error}"))
            })?;
            if bytes_read == 0 {
                break;
            }

            output
                .send(OutputChunk {
                    stream,
                    bytes: buffer[..bytes_read].to_vec(),
                })
                .map_err(|_| EngineError::Runtime("failed to forward task output".to_string()))?;
        }

        Ok(())
    })
}

pub(crate) fn join_output_reader(
    handle: thread::JoinHandle<Result<(), EngineError>>,
) -> Result<(), EngineError> {
    match handle.join() {
        Ok(result) => result,
        Err(_) => Err(EngineError::Runtime(
            "task output reader thread panicked".to_string(),
        )),
    }
}

fn command_status(status: ExitStatus) -> CommandStatus {
    if let Some(code) = status.code() {
        return CommandStatus::from_code(code);
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return CommandStatus::Failed(format!("unix_signal({signal})"));
        }
    }

    CommandStatus::Failed("unknown_exit_status".to_string())
}

fn platform_exit_reason(code: i32) -> String {
    if cfg!(windows) {
        format!("windows_exit_code({code})")
    } else if cfg!(unix) {
        format!("unix_exit_code({code})")
    } else {
        format!("exit_code({code})")
    }
}

#[cfg(test)]
mod tests {
    use std::io::IsTerminal;
    use std::sync::mpsc;

    use super::{
        CommandStatus, OutputStream, TerminalBackend, TerminalContext, run_with_system_shell,
        terminal_context,
    };

    #[test]
    fn formats_exit_status_cleanly() {
        let status = CommandStatus::from_code(1);
        let reason = status
            .failure_reason()
            .expect("a non-zero status should have a failure reason");
        let expected = if cfg!(windows) {
            "windows_exit_code(1)"
        } else if cfg!(unix) {
            "unix_exit_code(1)"
        } else {
            "exit_code(1)"
        };

        assert_eq!(reason, expected);
        assert!(!reason.contains("ExitCode("));
    }

    #[cfg(unix)]
    #[test]
    fn pty_merges_command_output() {
        let (output_tx, output_rx) = mpsc::channel();
        let status = run_with_system_shell(
            "sh",
            "-c",
            "printf out; printf err >&2",
            std::path::Path::new("."),
            output_tx,
            TerminalContext::pty(),
        )
        .expect("PTY command should run");

        assert_eq!(status, CommandStatus::Success);
        let chunks = output_rx.into_iter().collect::<Vec<_>>();
        assert!(!chunks.is_empty());
        assert!(
            chunks
                .iter()
                .all(|chunk| chunk.stream == OutputStream::Stdout)
        );
        let bytes = chunks
            .into_iter()
            .flat_map(|chunk| chunk.bytes)
            .collect::<Vec<_>>();
        assert!(bytes.ends_with(b"outerr"));
    }

    #[cfg(unix)]
    #[test]
    fn pty_closes_task_input() {
        let (output_tx, _output_rx) = mpsc::channel();
        let status = run_with_system_shell(
            "sh",
            "-c",
            "read value || test -z \"$value\"",
            std::path::Path::new("."),
            output_tx,
            TerminalContext::pty(),
        )
        .expect("PTY command should run");

        assert_eq!(status, CommandStatus::Success);
    }

    #[cfg(unix)]
    #[test]
    fn pipe_keeps_output_streams_separate() {
        let (output_tx, output_rx) = mpsc::channel();
        let status = run_with_system_shell(
            "sh",
            "-c",
            "printf out; printf err >&2",
            std::path::Path::new("."),
            output_tx,
            TerminalContext::pipe(),
        )
        .expect("Pipe command should run");

        assert_eq!(status, CommandStatus::Success);
        let chunks = output_rx.into_iter().collect::<Vec<_>>();
        assert!(
            chunks
                .iter()
                .any(|chunk| { chunk.stream == OutputStream::Stdout && chunk.bytes == b"out" })
        );
        assert!(
            chunks
                .iter()
                .any(|chunk| { chunk.stream == OutputStream::Stderr && chunk.bytes == b"err" })
        );
    }

    #[test]
    fn terminal_context_uses_pipe_for_non_terminal_output() {
        if std::io::stdin().is_terminal()
            && std::io::stdout().is_terminal()
            && std::io::stderr().is_terminal()
            && std::env::var_os("CI").is_none()
        {
            return;
        }

        assert_eq!(terminal_context().backend, TerminalBackend::Pipe);
    }
}
