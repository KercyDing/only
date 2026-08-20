use std::io::Write;
use std::sync::{Mutex, OnceLock};

use crate::EngineError;

static STDOUT: OnceLock<Mutex<anstream::Stdout>> = OnceLock::new();
static STDERR: OnceLock<Mutex<anstream::Stderr>> = OnceLock::new();

pub(super) fn write_stdout(content: &[u8]) -> Result<(), EngineError> {
    let mut writer = STDOUT
        .get_or_init(|| Mutex::new(ansi_stdout()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    write_output(content, &mut *writer)
}

pub(super) fn write_stderr(content: &[u8]) -> Result<(), EngineError> {
    let mut writer = STDERR
        .get_or_init(|| Mutex::new(ansi_stderr()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    write_output(content, &mut *writer)
}

fn ansi_stdout() -> anstream::Stdout {
    // The Wincon fallback only translates SGR styling and drops cursor-motion
    // sequences. ConPTY output needs the complete ANSI stream for progress UI.
    anstream::AutoStream::always_ansi(std::io::stdout())
}

fn ansi_stderr() -> anstream::Stderr {
    anstream::AutoStream::always_ansi(std::io::stderr())
}

fn write_output(content: &[u8], mut writer: impl Write) -> Result<(), EngineError> {
    writer
        .write_all(content)
        .and_then(|()| writer.flush())
        .map_err(|error| EngineError::Runtime(format!("failed to write task output: {error}")))
}
