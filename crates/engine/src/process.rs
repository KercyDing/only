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

#[derive(Debug, Clone)]
pub(crate) struct TerminalContext {
    pub backend: TerminalBackend,
    pub size: portable_pty::PtySize,
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
            size: portable_pty::PtySize::default(),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    #[cfg(all(test, unix))]
    pub(crate) fn pty() -> Self {
        Self {
            backend: TerminalBackend::Pty,
            size: portable_pty::PtySize::default(),
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

fn host_pty_size() -> portable_pty::PtySize {
    detected_pty_size().unwrap_or_default()
}

fn detected_pty_size() -> Option<portable_pty::PtySize> {
    terminal_size::terminal_size().map(|(width, height)| portable_pty::PtySize {
        rows: height.0,
        cols: width.0,
        pixel_width: 0,
        pixel_height: 0,
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
    if terminal.backend == TerminalBackend::Pty
        && let Some(status) =
            run_with_system_shell_pty(program, arg, command, working_dir, &output, terminal)?
    {
        return Ok(status);
    }

    run_with_system_shell_pipe(program, arg, command, working_dir, output, terminal)
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

fn run_with_system_shell_pty(
    program: &str,
    arg: &str,
    command: &str,
    working_dir: &Path,
    output: &Sender<OutputChunk>,
    terminal: &TerminalContext,
) -> Result<Option<CommandStatus>, EngineError> {
    let pty_system = portable_pty::native_pty_system();
    let pair = match pty_system.openpty(terminal.size) {
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
    for (name, value) in build_command_env(terminal) {
        builder.env(name, value);
    }
    let mut child = match pair.slave.spawn_command(builder) {
        Ok(child) => child,
        Err(_) => return Ok(None),
    };
    let process_tree = ProcessTree::attach_pty(child.as_mut())?;
    // Keep only the master side after spawning. Holding the parent slave open
    // can delay the EOF that terminates the output reader.
    drop(pair.slave);
    drop(closed_input);
    let reader_handle = spawn_output_reader(reader, OutputStream::Stdout, output.clone());
    let status = wait_for_pty_child(
        child.as_mut(),
        &process_tree,
        pair.master.as_ref(),
        terminal.size,
        &terminal.cancelled,
    )?;
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

struct ProcessTree {
    #[cfg(unix)]
    pid: u32,
    #[cfg(windows)]
    job: WindowsJob,
}

impl ProcessTree {
    fn attach_pipe(child: &mut std::process::Child, _program: &str) -> Result<Self, EngineError> {
        #[cfg(windows)]
        let job = {
            use std::os::windows::io::AsRawHandle;
            match WindowsJob::assign(child.as_raw_handle()) {
                Ok(job) => job,
                Err(source) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(EngineError::Io {
                        message: "failed to manage shell process tree",
                        path: _program.to_string(),
                        source,
                    });
                }
            }
        };
        Ok(Self {
            #[cfg(unix)]
            pid: child.id(),
            #[cfg(windows)]
            job,
        })
    }

    fn attach_pty(child: &mut dyn portable_pty::Child) -> Result<Self, EngineError> {
        #[cfg(unix)]
        let pid = child.process_id().ok_or_else(|| {
            EngineError::Runtime("PTY child did not expose a process id".to_string())
        })?;
        #[cfg(windows)]
        let job = {
            let handle = child.as_raw_handle().ok_or_else(|| {
                EngineError::Runtime("PTY child did not expose a process handle".to_string())
            })?;
            match WindowsJob::assign(handle) {
                Ok(job) => job,
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(EngineError::Runtime(format!(
                        "failed to manage PTY process tree: {error}"
                    )));
                }
            }
        };
        Ok(Self {
            #[cfg(unix)]
            pid,
            #[cfg(windows)]
            job,
        })
    }

    fn terminate(&self) -> Result<(), EngineError> {
        #[cfg(unix)]
        {
            send_group_signal(self.pid, libc::SIGTERM);
            thread::sleep(Duration::from_millis(200));
            send_group_signal(self.pid, libc::SIGKILL);
            Ok(())
        }
        #[cfg(windows)]
        {
            self.job.terminate().map_err(|error| {
                EngineError::Runtime(format!("failed to terminate process tree: {error}"))
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            Ok(())
        }
    }
}

#[cfg(windows)]
struct WindowsJob(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl WindowsJob {
    fn assign(process: std::os::windows::io::RawHandle) -> std::io::Result<Self> {
        use std::mem::size_of;
        use std::ptr;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };

        let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        let job = Self(handle);
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&raw const limits).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if configured == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let assigned = unsafe { AssignProcessToJobObject(job.0, process.cast()) };
        if assigned == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(job)
    }

    fn terminate(&self) -> std::io::Result<()> {
        let terminated =
            unsafe { windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0, 130) };
        if terminated == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[cfg(windows)]
impl Drop for WindowsJob {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
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

fn wait_for_pty_child(
    child: &mut dyn portable_pty::Child,
    process_tree: &ProcessTree,
    master: &dyn portable_pty::MasterPty,
    mut size: portable_pty::PtySize,
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
            let _ = master.resize(next_size);
            size = next_size;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
fn send_group_signal(pid: u32, signal: libc::c_int) {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return;
    };
    // The child is created as its own process-group leader, so a negative
    // pid targets only that execution unit and its descendants.
    unsafe {
        libc::kill(-pid, signal);
    }
}

pub(crate) fn run_with_system_shell_inherit(
    program: &str,
    arg: &str,
    command: &str,
    working_dir: &Path,
    terminal: &TerminalContext,
) -> Result<CommandStatus, EngineError> {
    let mut process = Command::new(program);
    process
        .current_dir(working_dir)
        .arg(arg)
        .arg(command)
        .envs(build_command_env(terminal))
        .stdin(Stdio::inherit())
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
    if terminal.backend == TerminalBackend::MergedPipe {
        enable_color(&mut env_vars);
    }
    env_vars
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
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::io::IsTerminal;
    #[cfg(unix)]
    use std::sync::mpsc;
    #[cfg(unix)]
    use std::time::{Duration, Instant};

    use super::{CommandStatus, TerminalBackend, enable_color, terminal_context};
    #[cfg(unix)]
    use super::{OutputStream, TerminalContext, run_with_system_shell};

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
        context.size = portable_pty::PtySize {
            rows: 31,
            cols: 97,
            pixel_width: 0,
            pixel_height: 0,
        };
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
    fn captured_output_uses_merged_pipe() {
        let terminal = TerminalContext::pty();
        let captured = terminal.for_captured_output();

        assert_eq!(captured.backend, TerminalBackend::MergedPipe);
        terminal.cancel();
        assert!(captured.is_cancelled());
    }

    #[cfg(unix)]
    #[test]
    fn merged_pipe_preserves_order() {
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
    fn merged_pipe_enables_color() {
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
    fn no_color_disables_forced_color() {
        let mut env_vars = HashMap::from([(OsString::from("NO_COLOR"), OsString::new())]);
        enable_color(&mut env_vars);

        assert!(!env_vars.contains_key(std::ffi::OsStr::new("CARGO_TERM_COLOR")));
        assert!(!env_vars.contains_key(std::ffi::OsStr::new("CLICOLOR_FORCE")));
        assert!(!env_vars.contains_key(std::ffi::OsStr::new("FORCE_COLOR")));
    }
}
