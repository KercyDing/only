use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{IsTerminal, Read};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::thread;
use std::time::Duration;

use crate::EngineError;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use platform::ProcessTree;
use platform::{add_powershell_process_flags, configure_process_group};
#[cfg(unix)]
use unix as platform;
#[cfg(windows)]
use windows as platform;

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
    MergedPipe,
    Pty,
}

/// Decides whether a directly-attached task may read the caller's stdin.
///
/// Only one task can meaningfully own the terminal's input. Tasks that run
/// alongside others get a closed stdin so they cannot steal keystrokes from
/// the task the user is actually interacting with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StdinAccess {
    Owned,
    Closed,
}

/// Terminal dimensions in character cells.
///
/// Kept independent of `portable_pty::PtySize` so that Windows, which has no
/// PTY path left, does not drag in the PTY stack for four integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerminalSize {
    pub rows: u16,
    pub cols: u16,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self { rows: 24, cols: 80 }
    }
}

#[cfg(unix)]
impl From<TerminalSize> for portable_pty::PtySize {
    fn from(size: TerminalSize) -> Self {
        Self {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TerminalContext {
    pub backend: TerminalBackend,
    pub size: TerminalSize,
    cancelled: Arc<AtomicBool>,
}

impl TerminalContext {
    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub(crate) fn for_captured_output(&self) -> Self {
        Self {
            // A PTY control stream is only meaningful while it is attached to
            // its terminal. Replaying it later can erase or omit earlier logs.
            // Merge both streams so their write order survives capture too.
            backend: match self.backend {
                TerminalBackend::Pty => TerminalBackend::MergedPipe,
                TerminalBackend::Pipe | TerminalBackend::MergedPipe => self.backend,
            },
            size: self.size,
            cancelled: self.cancelled.clone(),
        }
    }

    #[cfg(test)]
    pub(crate) fn pipe() -> Self {
        Self {
            backend: TerminalBackend::Pipe,
            size: TerminalSize::default(),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    #[cfg(all(test, unix))]
    pub(crate) fn pty() -> Self {
        Self {
            backend: TerminalBackend::Pty,
            size: TerminalSize::default(),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    #[cfg(test)]
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

static CTRL_C_HANDLER: OnceLock<Result<(), String>> = OnceLock::new();
static ACTIVE_CANCELLATIONS: OnceLock<Mutex<Vec<Weak<AtomicBool>>>> = OnceLock::new();

pub(crate) fn begin_terminal_invocation() -> Result<TerminalContext, EngineError> {
    let handler = CTRL_C_HANDLER.get_or_init(|| {
        ctrlc::set_handler(|| {
            let mut active = active_cancellations()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            active.retain(|token| {
                token.upgrade().is_some_and(|token| {
                    token.store(true, Ordering::SeqCst);
                    true
                })
            });
        })
        .map_err(|error| error.to_string())
    });
    if let Err(error) = handler {
        return Err(EngineError::Runtime(format!(
            "failed to install Ctrl-C handler: {error}"
        )));
    }
    let context = terminal_context();
    active_cancellations()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(Arc::downgrade(&context.cancelled));
    Ok(context)
}

fn active_cancellations() -> &'static Mutex<Vec<Weak<AtomicBool>>> {
    ACTIVE_CANCELLATIONS.get_or_init(|| Mutex::new(Vec::new()))
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
        size: host_pty_size(),
        cancelled: Arc::new(AtomicBool::new(false)),
    }
}

fn host_pty_size() -> TerminalSize {
    detected_pty_size().unwrap_or_default()
}

fn detected_pty_size() -> Option<TerminalSize> {
    terminal_size::terminal_size().map(|(width, height)| TerminalSize {
        rows: height.0,
        cols: width.0,
    })
}

pub(crate) fn run_with_system_shell(
    program: &str,
    arg: &str,
    command: &str,
    working_dir: &Path,
    output: Sender<OutputChunk>,
    terminal: &TerminalContext,
) -> Result<CommandStatus, EngineError> {
    #[cfg(unix)]
    if terminal.backend == TerminalBackend::Pty
        && !platform::uses_pipe_for_system_shell(program)
        && let Some(status) = platform::run_with_system_shell_pty(
            program,
            arg,
            command,
            working_dir,
            &output,
            terminal,
        )?
    {
        return Ok(status);
    }

    let terminal = if platform::uses_pipe_for_system_shell(program) {
        terminal.for_captured_output()
    } else {
        terminal.clone()
    };
    run_with_system_shell_pipe(program, arg, command, working_dir, output, &terminal)
}

fn run_with_system_shell_pipe(
    program: &str,
    arg: &str,
    command: &str,
    working_dir: &Path,
    output: Sender<OutputChunk>,
    terminal: &TerminalContext,
) -> Result<CommandStatus, EngineError> {
    let mut process = Command::new(program);
    add_powershell_process_flags(&mut process, program);
    process
        .current_dir(working_dir)
        .arg(arg)
        .arg(command)
        .envs(build_command_env(terminal))
        .stdin(Stdio::null());
    let merged_reader = if terminal.backend == TerminalBackend::MergedPipe {
        let (reader, writer) = std::io::pipe().map_err(|source| EngineError::Io {
            message: "failed to create shell output pipe",
            path: program.to_string(),
            source,
        })?;
        let stderr_writer = writer.try_clone().map_err(|source| EngineError::Io {
            message: "failed to clone shell output pipe",
            path: program.to_string(),
            source,
        })?;
        process
            .stdout(Stdio::from(writer))
            .stderr(Stdio::from(stderr_writer));
        Some(reader)
    } else {
        process.stdout(Stdio::piped()).stderr(Stdio::piped());
        None
    };
    configure_process_group(&mut process);

    let mut child = process.spawn().map_err(|source| EngineError::Io {
        message: "failed to start shell command",
        path: program.to_string(),
        source,
    })?;
    drop(process);
    let process_tree = ProcessTree::attach_pipe(&mut child, program)?;

    let output_handles = if let Some(reader) = merged_reader {
        vec![spawn_output_reader(reader, OutputStream::Stdout, output)]
    } else {
        let stdout = child.stdout.take().ok_or_else(|| {
            EngineError::Runtime(format!("failed to capture stdout for shell '{program}'"))
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            EngineError::Runtime(format!("failed to capture stderr for shell '{program}'"))
        })?;
        vec![
            spawn_output_reader(stdout, OutputStream::Stdout, output.clone()),
            spawn_output_reader(stderr, OutputStream::Stderr, output),
        ]
    };
    let status = wait_for_pipe_child(&mut child, &process_tree, program, &terminal.cancelled)?;
    for handle in output_handles {
        join_output_reader(handle)?;
    }

    Ok(command_status(status))
}

fn wait_for_pipe_child(
    child: &mut std::process::Child,
    process_tree: &ProcessTree,
    program: &str,
    cancelled: &AtomicBool,
) -> Result<ExitStatus, EngineError> {
    loop {
        if let Some(status) = child.try_wait().map_err(|source| EngineError::Io {
            message: "failed to wait for shell command",
            path: program.to_string(),
            source,
        })? {
            return Ok(status);
        }
        if cancelled.load(Ordering::SeqCst) {
            process_tree.terminate()?;
            return child.wait().map_err(|source| EngineError::Io {
                message: "failed to wait for cancelled shell command",
                path: program.to_string(),
                source,
            });
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(unix)]
fn wait_for_pty_child(
    child: &mut dyn portable_pty::Child,
    process_tree: &ProcessTree,
    master: &dyn portable_pty::MasterPty,
    mut size: TerminalSize,
    cancelled: &AtomicBool,
) -> Result<portable_pty::ExitStatus, EngineError> {
    loop {
        if let Some(status) = child.try_wait().map_err(|error| {
            EngineError::Runtime(format!("failed to wait for PTY command: {error}"))
        })? {
            return Ok(status);
        }
        if cancelled.load(Ordering::SeqCst) {
            process_tree.terminate()?;
            return child.wait().map_err(|error| {
                EngineError::Runtime(format!("failed to wait for cancelled PTY command: {error}"))
            });
        }
        if let Some(next_size) = detected_pty_size()
            && next_size != size
        {
            let _ = master.resize(next_size.into());
            size = next_size;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

pub(crate) fn run_with_system_shell_inherit(
    program: &str,
    arg: &str,
    command: &str,
    working_dir: &Path,
    terminal: &TerminalContext,
    stdin: StdinAccess,
) -> Result<CommandStatus, EngineError> {
    let mut process = Command::new(program);
    add_powershell_process_flags(&mut process, program);
    process
        .current_dir(working_dir)
        .arg(arg)
        .arg(command)
        .envs(build_command_env(terminal))
        .stdin(match stdin {
            StdinAccess::Owned => Stdio::inherit(),
            StdinAccess::Closed => Stdio::null(),
        })
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let mut child = process.spawn().map_err(|source| EngineError::Io {
        message: "failed to run shell command",
        path: program.to_string(),
        source,
    })?;

    #[cfg(windows)]
    let status = {
        let process_tree = ProcessTree::attach_pipe(&mut child, program)?;
        wait_for_pipe_child(&mut child, &process_tree, program, &terminal.cancelled)?
    };
    #[cfg(not(windows))]
    let status = child.wait().map_err(|source| EngineError::Io {
        message: "failed to wait for shell command",
        path: program.to_string(),
        source,
    })?;

    Ok(command_status(status))
}

pub(crate) fn build_command_env(terminal: &TerminalContext) -> HashMap<OsString, OsString> {
    let mut env_vars = std::env::vars_os().collect::<HashMap<_, _>>();
    env_vars
        .entry(OsString::from("INIT_CWD"))
        .or_insert_with(|| std::env::current_dir().unwrap_or_default().into_os_string());
    if matches!(
        terminal.backend,
        TerminalBackend::MergedPipe | TerminalBackend::Pty
    ) {
        configure_terminal_env(&mut env_vars, terminal.size);
        if terminal.backend == TerminalBackend::MergedPipe {
            enable_color(&mut env_vars);
        }
    }
    env_vars
}

fn configure_terminal_env(env_vars: &mut HashMap<OsString, OsString>, size: TerminalSize) {
    // Keep explicit user values intact and fill in the standard terminal
    // capability hints when a child is running in a PTY-like execution path.
    env_vars
        .entry(OsString::from("TERM"))
        .or_insert_with(|| OsString::from("xterm-256color"));
    env_vars
        .entry(OsString::from("COLORTERM"))
        .or_insert_with(|| OsString::from("truecolor"));
    if size.cols > 0 {
        env_vars
            .entry(OsString::from("COLUMNS"))
            .or_insert_with(|| OsString::from(size.cols.to_string()));
    }
    if size.rows > 0 {
        env_vars
            .entry(OsString::from("LINES"))
            .or_insert_with(|| OsString::from(size.rows.to_string()));
    }
}

fn enable_color(env_vars: &mut HashMap<OsString, OsString>) {
    if env_vars.contains_key(std::ffi::OsStr::new("NO_COLOR")) {
        return;
    }
    for (name, value) in [
        ("CARGO_TERM_COLOR", "always"),
        ("CLICOLOR_FORCE", "1"),
        ("FORCE_COLOR", "1"),
    ] {
        env_vars
            .entry(OsString::from(name))
            .or_insert_with(|| OsString::from(value));
    }
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

#[cfg(unix)]
pub(crate) fn pty_command_status(status: portable_pty::ExitStatus) -> CommandStatus {
    if status.success() {
        CommandStatus::Success
    } else if let Some(signal) = status.signal() {
        CommandStatus::Failed(format!("pty_signal({signal})"))
    } else {
        CommandStatus::from_code(status.exit_code() as i32)
    }
}

fn platform_exit_reason(code: i32) -> String {
    format!("exit code {code}")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::io::IsTerminal;
    #[cfg(any(unix, windows))]
    use std::sync::mpsc;
    #[cfg(any(unix, windows))]
    use std::time::{Duration, Instant};

    #[cfg(unix)]
    use super::OutputStream;
    use super::{
        CommandStatus, TerminalBackend, TerminalSize, configure_terminal_env, enable_color,
        terminal_context,
    };
    #[cfg(any(unix, windows))]
    use super::{TerminalContext, run_with_system_shell};

    #[test]
    fn formats_exit_status_cleanly() {
        let status = CommandStatus::from_code(1);
        let reason = status
            .failure_reason()
            .expect("a non-zero status should have a failure reason");
        let expected = "exit code 1";

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
            &TerminalContext::pty(),
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
            &TerminalContext::pty(),
        )
        .expect("PTY command should run");

        assert_eq!(status, CommandStatus::Success);
    }

    #[cfg(windows)]
    #[test]
    fn detached_child_outlives_task() {
        // The Windows installer task spawns a helper that waits for `only` to
        // exit before replacing the running binary. The job object that groups a
        // task's process tree must not kill such a helper when it is closed, so
        // the helper here waits for its own parent shell to exit and only then
        // writes the marker.
        let marker =
            std::env::temp_dir().join(format!("only-detached-child-{}.txt", std::process::id()));
        let _ = std::fs::remove_file(&marker);
        let command = format!(
            concat!(
                "$script = \"Wait-Process -Id $PID -ErrorAction SilentlyContinue; ",
                "Set-Content -LiteralPath '{}' -Value survived\"; ",
                "$encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($script)); ",
                "Start-Process -WindowStyle Hidden -FilePath (Get-Process -Id $PID).Path ",
                "-ArgumentList '-NoProfile','-EncodedCommand',$encoded"
            ),
            marker.display()
        );

        let (output_tx, _output_rx) = mpsc::channel();
        let status = run_with_system_shell(
            "pwsh",
            "-Command",
            &command,
            std::path::Path::new("."),
            output_tx,
            &TerminalContext::pipe(),
        )
        .expect("PowerShell should run");
        assert_eq!(status, CommandStatus::Success);

        let deadline = Instant::now() + Duration::from_secs(15);
        while !marker.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        let survived = marker.exists();
        let _ = std::fs::remove_file(&marker);
        assert!(
            survived,
            "detached helper should outlive the task process tree"
        );
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
            &TerminalContext::pipe(),
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

    #[cfg(unix)]
    #[test]
    fn cancels_pipe_process_tree() {
        let context = TerminalContext::pipe();
        let execution_context = context.clone();
        let (output_tx, _output_rx) = mpsc::channel();
        let started = Instant::now();
        let command = std::thread::spawn(move || {
            run_with_system_shell(
                "sh",
                "-c",
                "sleep 30 & wait",
                std::path::Path::new("."),
                output_tx,
                &execution_context,
            )
        });

        std::thread::sleep(Duration::from_millis(50));
        context.cancel();
        let status = command
            .join()
            .expect("command thread should finish")
            .expect("cancelled command should be reaped");

        assert!(status.failure_reason().is_some());
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[cfg(unix)]
    #[test]
    fn pty_uses_terminal_size() {
        let mut context = TerminalContext::pty();
        context.size = TerminalSize { rows: 31, cols: 97 };
        let (output_tx, output_rx) = mpsc::channel();
        let status = run_with_system_shell(
            "sh",
            "-c",
            "stty size",
            std::path::Path::new("."),
            output_tx,
            &context,
        )
        .expect("PTY command should run");
        let output = output_rx
            .into_iter()
            .flat_map(|chunk| chunk.bytes)
            .collect::<Vec<_>>();

        assert_eq!(status, CommandStatus::Success);
        assert!(String::from_utf8_lossy(&output).contains("31 97"));
    }

    #[cfg(unix)]
    #[test]
    fn cancels_pty_process_tree() {
        let context = TerminalContext::pty();
        let execution_context = context.clone();
        let (output_tx, _output_rx) = mpsc::channel();
        let started = Instant::now();
        let command = std::thread::spawn(move || {
            run_with_system_shell(
                "sh",
                "-c",
                "sleep 30 & wait",
                std::path::Path::new("."),
                output_tx,
                &execution_context,
            )
        });

        std::thread::sleep(Duration::from_millis(50));
        context.cancel();
        let status = command
            .join()
            .expect("command thread should finish")
            .expect("cancelled command should be reaped");

        assert!(status.failure_reason().is_some());
        assert!(started.elapsed() < Duration::from_secs(2));
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

    #[cfg(unix)]
    #[test]
    fn captured_uses_merged_pipe() {
        let terminal = TerminalContext::pty();
        let captured = terminal.for_captured_output();

        assert_eq!(captured.backend, TerminalBackend::MergedPipe);
        terminal.cancel();
        assert!(captured.is_cancelled());
    }

    #[cfg(unix)]
    #[test]
    fn merged_preserves_order() {
        let (output_tx, output_rx) = mpsc::channel();
        let terminal = TerminalContext::pty().for_captured_output();
        let status = run_with_system_shell(
            "sh",
            "-c",
            "printf first; printf second >&2; printf third",
            std::path::Path::new("."),
            output_tx,
            &terminal,
        )
        .expect("merged pipe command should run");
        let output = output_rx
            .into_iter()
            .flat_map(|chunk| chunk.bytes)
            .collect::<Vec<_>>();

        assert_eq!(status, CommandStatus::Success);
        assert_eq!(output, b"firstsecondthird");
    }

    #[test]
    fn merged_enables_color() {
        let mut env_vars = HashMap::new();
        enable_color(&mut env_vars);

        assert_eq!(
            env_vars.get(std::ffi::OsStr::new("CARGO_TERM_COLOR")),
            Some(&OsString::from("always"))
        );
        assert_eq!(
            env_vars.get(std::ffi::OsStr::new("CLICOLOR_FORCE")),
            Some(&OsString::from("1"))
        );
        assert_eq!(
            env_vars.get(std::ffi::OsStr::new("FORCE_COLOR")),
            Some(&OsString::from("1"))
        );
    }

    #[test]
    fn terminal_env_has_hints() {
        let mut env_vars = HashMap::new();
        configure_terminal_env(&mut env_vars, TerminalSize { rows: 31, cols: 97 });

        assert_eq!(
            env_vars.get(std::ffi::OsStr::new("TERM")),
            Some(&OsString::from("xterm-256color"))
        );
        assert_eq!(
            env_vars.get(std::ffi::OsStr::new("COLORTERM")),
            Some(&OsString::from("truecolor"))
        );
        assert_eq!(
            env_vars.get(std::ffi::OsStr::new("COLUMNS")),
            Some(&OsString::from("97"))
        );
        assert_eq!(
            env_vars.get(std::ffi::OsStr::new("LINES")),
            Some(&OsString::from("31"))
        );
    }

    #[test]
    fn terminal_env_preserves_values() {
        let mut env_vars = HashMap::from([
            (OsString::from("TERM"), OsString::from("dumb")),
            (OsString::from("COLUMNS"), OsString::from("120")),
        ]);
        configure_terminal_env(&mut env_vars, TerminalSize { rows: 31, cols: 97 });

        assert_eq!(
            env_vars.get(std::ffi::OsStr::new("TERM")),
            Some(&OsString::from("dumb"))
        );
        assert_eq!(
            env_vars.get(std::ffi::OsStr::new("COLUMNS")),
            Some(&OsString::from("120"))
        );
        assert_eq!(
            env_vars.get(std::ffi::OsStr::new("LINES")),
            Some(&OsString::from("31"))
        );
    }

    #[test]
    fn no_color_is_respected() {
        let mut env_vars = HashMap::from([(OsString::from("NO_COLOR"), OsString::new())]);
        enable_color(&mut env_vars);

        assert!(!env_vars.contains_key(std::ffi::OsStr::new("CARGO_TERM_COLOR")));
        assert!(!env_vars.contains_key(std::ffi::OsStr::new("CLICOLOR_FORCE")));
        assert!(!env_vars.contains_key(std::ffi::OsStr::new("FORCE_COLOR")));
    }
}
