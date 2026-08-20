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

    pub(crate) fn terminate(&self) -> Result<(), EngineError> {
        self.job.terminate().map_err(|error| {
            EngineError::Runtime(format!("failed to terminate process tree: {error}"))
        })
    }
}

pub(crate) fn configure_process_group(_command: &mut std::process::Command) {}

/// Reports that captured Windows output always flows through pipes.
///
/// ConPTY would hand the child a real console, but it emits full-screen redraw
/// instructions that fight with the task progress `only` draws on the same
/// screen, and its device queries would reach the host terminal. Tasks that
/// need a real console take the inherit path instead, where the child writes
/// straight to the caller's terminal.
pub(crate) fn uses_pipe_for_system_shell(_program: &str) -> bool {
    true
}

pub(crate) fn add_powershell_process_flags(process: &mut std::process::Command, program: &str) {
    if is_powershell(program) {
        process.args(["-NoLogo", "-NoProfile", "-NonInteractive"]);
    }
}

fn is_powershell(program: &str) -> bool {
    program.eq_ignore_ascii_case("pwsh")
        || program.eq_ignore_ascii_case("pwsh.exe")
        || program.eq_ignore_ascii_case("powershell")
        || program.eq_ignore_ascii_case("powershell.exe")
}

struct WindowsJob(windows_sys::Win32::Foundation::HANDLE);

impl WindowsJob {
    /// Groups a shell process and every process it spawns, so that cancelling a
    /// task can terminate the whole tree in one call.
    ///
    /// The job deliberately carries no `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
    /// Closing the handle must not kill the members: a task may legitimately
    /// spawn a process that outlives `only`, such as a Windows installer helper
    /// that waits for `only` to exit before replacing the running binary.
    /// Cancellation still tears the tree down through `terminate`.
    fn assign(process: std::os::windows::io::RawHandle) -> std::io::Result<Self> {
        use std::ptr;
        use windows_sys::Win32::System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW};
        let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        let job = Self(handle);
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
