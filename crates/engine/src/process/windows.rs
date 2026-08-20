use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::Sender;
use std::thread;

use super::{
    CommandStatus, OutputChunk, OutputStream, TerminalContext, join_output_reader,
    pty_command_status, wait_for_pty_child,
};
use crate::EngineError;

pub(crate) struct ProcessTree {
    job: WindowsJob,
}

impl ProcessTree {
    pub(crate) fn attach_pipe(
        child: &mut std::process::Child,
        program: &str,
    ) -> Result<Self, EngineError> {
        use std::os::windows::io::AsRawHandle;
        let job = WindowsJob::assign(child.as_raw_handle()).map_err(|source| EngineError::Io {
            message: "failed to manage shell process tree",
            path: program.to_string(),
            source,
        })?;
        Ok(Self { job })
    }

    pub(crate) fn attach_pty(child: &mut dyn portable_pty::Child) -> Result<Self, EngineError> {
        let handle = child.as_raw_handle().ok_or_else(|| {
            EngineError::Runtime("PTY child did not expose a process handle".to_string())
        })?;
        let job = WindowsJob::assign(handle).map_err(|error| {
            EngineError::Runtime(format!("failed to manage PTY process tree: {error}"))
        })?;
        Ok(Self { job })
    }

    pub(crate) fn terminate(&self) -> Result<(), EngineError> {
        self.job.terminate().map_err(|error| {
            EngineError::Runtime(format!("failed to terminate process tree: {error}"))
        })
    }
}

pub(crate) fn configure_process_group(_command: &mut std::process::Command) {}

pub(crate) fn uses_pipe_for_system_shell(_program: &str) -> bool {
    false
}

pub(crate) fn add_powershell_process_flags(process: &mut std::process::Command, program: &str) {
    if is_powershell(program) {
        process.args(["-NoLogo", "-NoProfile", "-NonInteractive"]);
    }
}

pub(crate) fn add_powershell_pty_flags(builder: &mut portable_pty::CommandBuilder, program: &str) {
    if is_powershell(program) {
        builder.args(["-NoLogo", "-NoProfile", "-NonInteractive"]);
    }
}

pub(crate) fn run_with_system_shell_pty(
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
    add_powershell_pty_flags(&mut builder, program);
    builder.arg(arg);
    builder.arg(command);
    builder.cwd(working_dir);
    for (name, value) in super::build_command_env(terminal) {
        builder.env(name, value);
    }
    let mut child = match pair.slave.spawn_command(builder) {
        Ok(child) => child,
        Err(_) => return Ok(None),
    };
    let process_tree = ProcessTree::attach_pty(child.as_mut())?;
    drop(pair.slave);
    let reader_handle = spawn_conpty_output_reader(reader, closed_input, output.clone());
    let status = wait_for_pty_child(
        child.as_mut(),
        &process_tree,
        pair.master.as_ref(),
        terminal.size,
        &terminal.cancelled,
    )?;
    drop(pair.master);
    join_output_reader(reader_handle)?;
    Ok(Some(pty_command_status(status)))
}

fn spawn_conpty_output_reader<R, W>(
    mut reader: R,
    mut input: W,
    output: Sender<OutputChunk>,
) -> thread::JoinHandle<Result<(), EngineError>>
where
    R: Read + Send + 'static,
    W: Write + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        let mut pending = Vec::new();
        let mut replied_to_cursor_query = false;

        loop {
            let bytes_read = reader.read(&mut buffer).map_err(|error| {
                EngineError::Runtime(format!("failed to read ConPTY output: {error}"))
            })?;
            if bytes_read == 0 {
                break;
            }
            pending.extend_from_slice(&buffer[..bytes_read]);
            forward_conpty_output(
                &mut pending,
                &mut replied_to_cursor_query,
                &mut input,
                &output,
                false,
            )?;
        }
        forward_conpty_output(
            &mut pending,
            &mut replied_to_cursor_query,
            &mut input,
            &output,
            true,
        )
    })
}

fn forward_conpty_output<W>(
    pending: &mut Vec<u8>,
    replied_to_cursor_query: &mut bool,
    input: &mut W,
    output: &Sender<OutputChunk>,
    eof: bool,
) -> Result<(), EngineError>
where
    W: Write,
{
    const CURSOR_QUERY: &[u8] = b"\x1b[6n";
    const CURSOR_POSITION: &[u8] = b"\x1b[1;1R";

    let mut forwarded = Vec::with_capacity(pending.len());
    let mut index = 0;
    while index < pending.len() {
        let remaining = &pending[index..];
        // portable-pty enables INHERIT_CURSOR, so ConPTY blocks startup until
        // its initial cursor-position query receives a response.
        if !*replied_to_cursor_query
            && remaining.len() < CURSOR_QUERY.len()
            && CURSOR_QUERY.starts_with(remaining)
            && !eof
        {
            break;
        }
        if !*replied_to_cursor_query && remaining.starts_with(CURSOR_QUERY) {
            input.write_all(CURSOR_POSITION).map_err(|error| {
                EngineError::Runtime(format!("failed to answer ConPTY cursor query: {error}"))
            })?;
            *replied_to_cursor_query = true;
            index += CURSOR_QUERY.len();
        } else {
            forwarded.push(pending[index]);
            index += 1;
        }
    }
    pending.drain(..index);
    if !forwarded.is_empty() {
        output
            .send(OutputChunk {
                stream: OutputStream::Stdout,
                bytes: forwarded,
            })
            .map_err(|_| EngineError::Runtime("failed to forward task output".to_string()))?;
    }
    Ok(())
}

fn is_powershell(program: &str) -> bool {
    program.eq_ignore_ascii_case("pwsh")
        || program.eq_ignore_ascii_case("pwsh.exe")
        || program.eq_ignore_ascii_case("powershell")
        || program.eq_ignore_ascii_case("powershell.exe")
}

struct WindowsJob(windows_sys::Win32::Foundation::HANDLE);

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

impl Drop for WindowsJob {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
