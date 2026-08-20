use std::path::Path;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use super::{
    CommandStatus, OutputChunk, OutputStream, TerminalContext, join_output_reader,
    pty_command_status, spawn_output_reader, wait_for_pty_child,
};
use crate::EngineError;

pub(crate) struct ProcessTree {
    pid: u32,
}

impl ProcessTree {
    pub(crate) fn attach_pipe(
        child: &mut std::process::Child,
        _program: &str,
    ) -> Result<Self, EngineError> {
        Ok(Self { pid: child.id() })
    }

    pub(crate) fn attach_pty(child: &mut dyn portable_pty::Child) -> Result<Self, EngineError> {
        let pid = child.process_id().ok_or_else(|| {
            EngineError::Runtime("PTY child did not expose a process id".to_string())
        })?;
        Ok(Self { pid })
    }

    pub(crate) fn terminate(&self) -> Result<(), EngineError> {
        send_group_signal(self.pid, libc::SIGTERM);
        thread::sleep(Duration::from_millis(200));
        send_group_signal(self.pid, libc::SIGKILL);
        Ok(())
    }
}

pub(crate) fn configure_process_group(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

pub(crate) fn add_powershell_process_flags(_process: &mut std::process::Command, _program: &str) {}

pub(crate) fn uses_pipe_for_system_shell(_program: &str) -> bool {
    false
}

pub(crate) fn add_powershell_pty_flags(
    _builder: &mut portable_pty::CommandBuilder,
    _program: &str,
) {
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
    drop(closed_input);
    let reader_handle = spawn_output_reader(reader, OutputStream::Stdout, output.clone());
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

fn send_group_signal(pid: u32, signal: libc::c_int) {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return;
    };
    unsafe {
        libc::kill(-pid, signal);
    }
}
