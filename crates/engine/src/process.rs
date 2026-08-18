use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
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
    pub text: String,
}

pub(crate) fn run_with_system_shell(
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
    reader: R,
    stream: OutputStream,
    output: Sender<OutputChunk>,
) -> thread::JoinHandle<Result<(), EngineError>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut buffer = Vec::new();

        loop {
            buffer.clear();
            let bytes_read = reader.read_until(b'\n', &mut buffer).map_err(|error| {
                EngineError::Runtime(format!("failed to read task output: {error}"))
            })?;
            if bytes_read == 0 {
                break;
            }

            output
                .send(OutputChunk {
                    stream,
                    text: String::from_utf8_lossy(&buffer).into_owned(),
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
    use super::CommandStatus;

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
}
