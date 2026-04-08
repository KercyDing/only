use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, ExitCode};

use crate::EngineError;

pub(crate) fn run_with_system_shell(
    program: &str,
    arg: &str,
    command: &str,
    working_dir: &Path,
) -> Result<ExitCode, EngineError> {
    let mut process = Command::new(program);
    process
        .current_dir(working_dir)
        .arg(arg)
        .arg(command)
        .envs(build_command_env());

    let status = process.status().map_err(|source| EngineError::Io {
        message: "failed to start shell command",
        path: program.to_string(),
        source,
    })?;

    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}

pub(crate) fn build_command_env() -> HashMap<OsString, OsString> {
    let mut env_vars = std::env::vars_os().collect::<HashMap<_, _>>();
    env_vars
        .entry(OsString::from("INIT_CWD"))
        .or_insert_with(|| std::env::current_dir().unwrap_or_default().into_os_string());
    env_vars
}
