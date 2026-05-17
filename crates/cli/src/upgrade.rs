//! Upgrades the installed `only` binary from GitHub release assets.
//!
//! Args:
//! None.
//!
//! Returns:
//! Platform-specific download and install helpers used by the CLI.
//!
//! Edge Cases:
//! Windows defers replacement until the current process exits because a running executable is
//! locked by the OS.

use crate::error::{OnlyError, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const REPO: &str = "KercyDing/only";

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpgradePlan {
    download_url: String,
    install_path: PathBuf,
    staged_path: PathBuf,
}

/// Downloads and installs the latest GitHub release for the current platform.
///
/// Args:
/// None.
///
/// Returns:
/// Success when the upgrade is applied or scheduled.
///
/// Edge Cases:
/// On Windows the final copy is run by a detached PowerShell process after this process exits.
pub(crate) fn run_upgrade() -> Result<ExitCode> {
    let plan = build_upgrade_plan()?;
    execute_upgrade(&plan)?;
    Ok(ExitCode::SUCCESS)
}

fn build_upgrade_plan() -> Result<UpgradePlan> {
    let binary = current_platform_binary()?;
    let download_url = latest_download_url(binary);
    let install_path = default_install_path()?;
    let staged_path = staged_path_for(&install_path)?;

    Ok(UpgradePlan {
        download_url,
        install_path,
        staged_path,
    })
}

fn latest_download_url(binary: &str) -> String {
    format!("https://github.com/{REPO}/releases/latest/download/{binary}")
}

#[cfg(windows)]
fn current_platform_binary() -> Result<&'static str> {
    match env::consts::ARCH {
        "x86_64" => Ok("only-windows-amd64.exe"),
        arch => Err(OnlyError::runtime(format!(
            "unsupported Windows architecture: {arch}"
        ))),
    }
}

#[cfg(unix)]
fn current_platform_binary() -> Result<&'static str> {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Ok("only-linux-amd64"),
        ("macos", "x86_64") => Ok("only-darwin-amd64"),
        ("macos", "aarch64") => Ok("only-darwin-arm64"),
        (os, arch) => Err(OnlyError::runtime(format!(
            "unsupported platform: {os} {arch}"
        ))),
    }
}

#[cfg(windows)]
fn default_install_path() -> Result<PathBuf> {
    let local_app_data =
        env::var_os("LOCALAPPDATA").ok_or_else(|| OnlyError::runtime("LOCALAPPDATA is not set"))?;
    Ok(PathBuf::from(local_app_data)
        .join("Programs")
        .join("Only")
        .join("only.exe"))
}

#[cfg(unix)]
fn default_install_path() -> Result<PathBuf> {
    let install_dir = if directory_is_writable(Path::new("/usr/local/bin")) {
        PathBuf::from("/usr/local/bin")
    } else {
        let home = env::var_os("HOME").ok_or_else(|| OnlyError::runtime("HOME is not set"))?;
        PathBuf::from(home).join(".local").join("bin")
    };

    Ok(install_dir.join("only"))
}

#[cfg(windows)]
fn staged_path_for(_install_path: &Path) -> Result<PathBuf> {
    Ok(env::temp_dir().join(format!("only-upgrade-{}.exe", std::process::id())))
}

#[cfg(unix)]
fn staged_path_for(install_path: &Path) -> Result<PathBuf> {
    let install_dir = install_path
        .parent()
        .ok_or_else(|| OnlyError::runtime("install path has no parent directory"))?;
    Ok(install_dir.join(format!(".only-upgrade-{}", std::process::id())))
}

#[cfg(windows)]
fn execute_upgrade(plan: &UpgradePlan) -> Result<()> {
    let install_dir = plan
        .install_path
        .parent()
        .ok_or_else(|| OnlyError::runtime("install path has no parent directory"))?;
    fs::create_dir_all(install_dir).map_err(|error| {
        OnlyError::io_with_path(
            "failed to create install directory",
            install_dir.to_path_buf(),
            error,
        )
    })?;

    let latest_version = fetch_latest_version()?;
    if current_version_matches(&latest_version) {
        println!("Already up to date.");
        return Ok(());
    }

    print_download_summary(plan);
    download_with_windows_tool(&plan.download_url, &plan.staged_path)?;
    let installed_version = binary_version(&plan.staged_path)?;

    if current_exe_is_install_path(&plan.install_path) {
        let powershell = find_windows_powershell()?;
        schedule_windows_replacement(&powershell, &plan.staged_path, &plan.install_path)?;
    } else {
        fs::copy(&plan.staged_path, &plan.install_path).map_err(|error| {
            OnlyError::io_with_path(
                "failed to install upgraded binary",
                plan.install_path.clone(),
                error,
            )
        })?;
        fs::remove_file(&plan.staged_path).map_err(|error| {
            OnlyError::io_with_path(
                "failed to remove staged binary",
                plan.staged_path.clone(),
                error,
            )
        })?;
    }

    print_upgrade_done(&installed_version);
    Ok(())
}

#[cfg(unix)]
fn execute_upgrade(plan: &UpgradePlan) -> Result<()> {
    let install_dir = plan
        .install_path
        .parent()
        .ok_or_else(|| OnlyError::runtime("install path has no parent directory"))?;
    fs::create_dir_all(install_dir).map_err(|error| {
        OnlyError::io_with_path(
            "failed to create install directory",
            install_dir.to_path_buf(),
            error,
        )
    })?;

    let latest_version = fetch_latest_version()?;
    if current_version_matches(&latest_version) {
        println!("Already up to date.");
        return Ok(());
    }

    print_download_summary(plan);
    download_with_system_tool(&plan.download_url, &plan.staged_path)?;
    let installed_version = binary_version(&plan.staged_path)?;
    make_executable(&plan.staged_path)?;
    fs::rename(&plan.staged_path, &plan.install_path).map_err(|error| {
        OnlyError::io_with_path(
            "failed to install upgraded binary",
            plan.install_path.clone(),
            error,
        )
    })?;

    print_upgrade_done(&installed_version);
    Ok(())
}

fn print_download_summary(plan: &UpgradePlan) {
    println!("Downloading from: {}", plan.download_url);
    println!("Installing to: {}", plan.install_path.display());
}

fn print_upgrade_done(installed_version: &str) {
    println!("{} -> {installed_version}", env!("CARGO_PKG_VERSION"));
    println!("Done.");
}

fn binary_version(path: &Path) -> Result<String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| {
            OnlyError::runtime(format!("failed to read downloaded binary version: {error}"))
        })?;

    if !output.status.success() {
        return Err(OnlyError::runtime(
            "downloaded binary did not report its version",
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn fetch_latest_version() -> Result<String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let body = fetch_url_text(&url)?;
    parse_latest_release_tag(&body)
}

fn current_version_matches(latest_version: &str) -> bool {
    normalize_version(env!("CARGO_PKG_VERSION")) == normalize_version(latest_version)
}

fn normalize_version(version: &str) -> &str {
    version.trim().trim_start_matches('v')
}

fn parse_latest_release_tag(body: &str) -> Result<String> {
    let Some(key_start) = body.find("\"tag_name\"") else {
        return Err(OnlyError::runtime(
            "GitHub latest release response did not include tag_name",
        ));
    };
    let after_key = &body[key_start + "\"tag_name\"".len()..];
    let Some(colon_start) = after_key.find(':') else {
        return Err(OnlyError::runtime(
            "GitHub latest release tag_name was malformed",
        ));
    };
    let after_colon = after_key[colon_start + 1..].trim_start();
    let Some(stripped) = after_colon.strip_prefix('"') else {
        return Err(OnlyError::runtime(
            "GitHub latest release tag_name was not a string",
        ));
    };
    let Some(end) = stripped.find('"') else {
        return Err(OnlyError::runtime(
            "GitHub latest release tag_name string was unterminated",
        ));
    };

    Ok(stripped[..end].to_string())
}

#[cfg(windows)]
fn fetch_url_text(url: &str) -> Result<String> {
    if command_exists("curl.exe") {
        return run_text_command("curl.exe", &["-fsSL", url]);
    }

    let powershell = find_windows_powershell()?;
    let script = format!(
        "$ErrorActionPreference = 'Stop'; $ProgressPreference = 'SilentlyContinue'; Invoke-WebRequest -Uri {} -UseBasicParsing | Select-Object -ExpandProperty Content",
        ps_literal(url),
    );
    run_text_command(
        &powershell,
        &[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ],
    )
}

#[cfg(unix)]
fn fetch_url_text(url: &str) -> Result<String> {
    if command_exists("curl") {
        return run_text_command("curl", &["-fsSL", url]);
    }
    if command_exists("wget") {
        return run_text_command("wget", &["-qO-", url]);
    }

    Err(OnlyError::runtime(
        "curl or wget is required to check the latest release",
    ))
}

fn run_text_command(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|error| OnlyError::runtime(format!("failed to start {command}: {error}")))?;

    if !output.status.success() {
        return Err(OnlyError::runtime(format!(
            "failed to fetch latest release metadata with {command}"
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(windows)]
fn current_exe_is_install_path(install_path: &Path) -> bool {
    let Ok(current_exe) = env::current_exe() else {
        return true;
    };

    paths_equal(&current_exe, install_path)
}

#[cfg(windows)]
fn paths_equal(left: &Path, right: &Path) -> bool {
    let left = fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left.as_os_str().to_string_lossy().to_lowercase()
        == right.as_os_str().to_string_lossy().to_lowercase()
}

#[cfg(windows)]
fn find_windows_powershell() -> Result<String> {
    if command_exists("pwsh.exe") {
        return Ok("pwsh.exe".to_string());
    }
    if command_exists("powershell.exe") {
        return Ok("powershell.exe".to_string());
    }

    Err(OnlyError::runtime(
        "PowerShell is required to download the upgrade",
    ))
}

#[cfg(windows)]
fn download_with_windows_tool(url: &str, output: &Path) -> Result<()> {
    if command_exists("curl.exe") {
        return run_download_command(
            "curl.exe",
            &["-fL", "--progress-bar", url, "-o"],
            output,
            url,
        );
    }

    let powershell = find_windows_powershell()?;
    download_with_powershell(&powershell, url, output)
}

#[cfg(windows)]
fn download_with_powershell(powershell: &str, url: &str, output: &Path) -> Result<()> {
    let script = format!(
        "$ErrorActionPreference = 'Stop'; $ProgressPreference = 'Continue'; Invoke-WebRequest -Uri {} -OutFile {} -UseBasicParsing",
        ps_literal(url),
        ps_literal_path(output),
    );
    let status = Command::new(powershell)
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"])
        .arg(script)
        .status()
        .map_err(|error| OnlyError::runtime(format!("failed to start PowerShell: {error}")))?;

    if status.success() {
        Ok(())
    } else {
        Err(OnlyError::runtime(format!(
            "failed to download only from {url}"
        )))
    }
}

#[cfg(windows)]
fn schedule_windows_replacement(
    powershell: &str,
    staged: &Path,
    install_path: &Path,
) -> Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let script = format!(
        "$ErrorActionPreference = 'Stop'; Wait-Process -Id {}; Copy-Item -LiteralPath {} -Destination {} -Force; Remove-Item -LiteralPath {} -Force",
        std::process::id(),
        ps_literal_path(staged),
        ps_literal_path(install_path),
        ps_literal_path(staged),
    );
    let encoded = encode_powershell_command(&script);

    Command::new(powershell)
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
        ])
        .arg(encoded)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| OnlyError::runtime(format!("failed to schedule upgrade: {error}")))?;

    Ok(())
}

#[cfg(windows)]
fn ps_literal_path(path: &Path) -> String {
    ps_literal(&path.display().to_string())
}

#[cfg(windows)]
fn ps_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(windows)]
fn encode_powershell_command(script: &str) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = script
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;

        encoded.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        encoded.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            encoded.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
        } else {
            encoded.push('=');
        }
        if chunk.len() > 2 {
            encoded.push(TABLE[(n & 0x3f) as usize] as char);
        } else {
            encoded.push('=');
        }
    }

    encoded
}

#[cfg(unix)]
fn download_with_system_tool(url: &str, output: &Path) -> Result<()> {
    if command_exists("curl") {
        return run_download_command("curl", &["-fL", "--progress-bar", url, "-o"], output, url);
    }
    if command_exists("wget") {
        return run_download_command("wget", &["--show-progress", url, "-O"], output, url);
    }

    Err(OnlyError::runtime(
        "curl or wget is required to download the upgrade",
    ))
}

fn run_download_command(command: &str, args: &[&str], output: &Path, url: &str) -> Result<()> {
    let status = Command::new(command)
        .args(args)
        .arg(output)
        .status()
        .map_err(|error| OnlyError::runtime(format!("failed to start {command}: {error}")))?;

    if status.success() {
        Ok(())
    } else {
        Err(OnlyError::runtime(format!(
            "failed to download only from {url}"
        )))
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|error| {
            OnlyError::io_with_path("failed to read staged binary", path.to_path_buf(), error)
        })?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).map_err(|error| {
        OnlyError::io_with_path(
            "failed to mark staged binary executable",
            path.to_path_buf(),
            error,
        )
    })
}

#[cfg(unix)]
fn directory_is_writable(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    let probe = path.join(format!(".only-write-test-{}", std::process::id()));
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

fn command_exists(command: &str) -> bool {
    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&path_var).any(|dir| dir.join(command).is_file())
}

#[cfg(test)]
mod tests {
    use super::{current_version_matches, latest_download_url, parse_latest_release_tag};

    #[test]
    fn builds_latest_github_download_url() {
        assert_eq!(
            latest_download_url("only-linux-amd64"),
            "https://github.com/KercyDing/only/releases/latest/download/only-linux-amd64"
        );
    }

    #[test]
    fn parses_latest_release_tag() {
        let body = r#"{"url":"https://api.github.com","tag_name":"v0.0.5","name":"v0.0.5"}"#;

        assert_eq!(
            parse_latest_release_tag(body).expect("tag should parse"),
            "v0.0.5"
        );
    }

    #[test]
    fn matches_current_version_with_optional_v_prefix() {
        assert!(current_version_matches(concat!(
            "v",
            env!("CARGO_PKG_VERSION")
        )));
    }
}
