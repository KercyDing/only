//! Upgrades the installed `only` binary from GitHub release assets.
//!
//! Args:
//! None.
//!
//! Returns:
//! Platform-specific download and install helpers used by the CLI.
//!
//! Edge Cases:
//! Installed binaries replace themselves in place; ad hoc binaries copy into the install path.

use crate::error::{OnlyError, Result};
use self_replace::self_replace;
use std::env;
use std::fs;
use std::fs::OpenOptions;
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
/// Success when the upgrade is applied or copied into place.
///
/// Edge Cases:
/// Self-update uses the running install path when available and falls back to a normal copy
/// otherwise.
pub(crate) fn run_upgrade() -> Result<ExitCode> {
    let plan = build_upgrade_plan()?;
    execute_upgrade(&plan)?;
    Ok(ExitCode::SUCCESS)
}

fn build_upgrade_plan() -> Result<UpgradePlan> {
    let binary = current_platform_binary()?;
    let download_url = latest_download_url(binary);
    let install_path = upgrade_install_path()?;
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
        "aarch64" => Ok("only-windows-arm64.exe"),
        arch => Err(OnlyError::runtime(format!(
            "unsupported Windows architecture: {arch}"
        ))),
    }
}

#[cfg(unix)]
fn current_platform_binary() -> Result<&'static str> {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Ok("only-linux-amd64"),
        ("linux", "aarch64") => Ok("only-linux-arm64"),
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
fn upgrade_install_path() -> Result<PathBuf> {
    default_install_path()
}

#[cfg(unix)]
fn upgrade_install_path() -> Result<PathBuf> {
    if let Ok(current_exe) = env::current_exe()
        && is_unix_install_path(&current_exe)
    {
        return Ok(current_exe);
    }

    default_install_path()
}

#[cfg(unix)]
fn is_unix_install_path(path: &Path) -> bool {
    if path.file_name().and_then(|name| name.to_str()) != Some("only") {
        return false;
    }

    let Some(parent) = path.parent() else {
        return false;
    };

    if parent == Path::new("/usr/local/bin") {
        return true;
    }

    let Some(home) = env::var_os("HOME") else {
        return false;
    };

    parent == PathBuf::from(home).join(".local").join("bin")
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
    if compare_versions(env!("CARGO_PKG_VERSION"), &latest_version)? != VersionOrder::Older {
        println!("Already up to date.");
        return Ok(());
    }

    ensure_directory_writable(install_dir, &plan.install_path)?;
    print_download_summary(plan);
    download_with_windows_tool(&plan.download_url, &plan.staged_path)?;

    if current_exe_is_install_path(&plan.install_path) {
        self_replace(&plan.staged_path).map_err(|error| {
            OnlyError::runtime(format!("failed to replace running binary: {error}"))
        })?;
    } else {
        fs::copy(&plan.staged_path, &plan.install_path).map_err(|error| {
            OnlyError::io_with_path(
                "failed to install upgraded binary",
                plan.install_path.clone(),
                error,
            )
        })?;
    }

    fs::remove_file(&plan.staged_path).map_err(|error| {
        OnlyError::io_with_path(
            "failed to remove staged binary",
            plan.staged_path.clone(),
            error,
        )
    })?;

    print_upgrade_done(normalize_version(&latest_version));
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
    if compare_versions(env!("CARGO_PKG_VERSION"), &latest_version)? != VersionOrder::Older {
        println!("Already up to date.");
        return Ok(());
    }

    ensure_directory_writable(install_dir, &plan.install_path)?;
    print_download_summary(plan);
    download_with_system_tool(&plan.download_url, &plan.staged_path)?;
    make_executable(&plan.staged_path)?;
    if current_exe_is_install_path(&plan.install_path) {
        self_replace(&plan.staged_path).map_err(|error| {
            OnlyError::runtime(format!("failed to replace running binary: {error}"))
        })?;
    } else {
        fs::copy(&plan.staged_path, &plan.install_path).map_err(|error| {
            OnlyError::io_with_path(
                "failed to install upgraded binary",
                plan.install_path.clone(),
                error,
            )
        })?;
    }
    fs::remove_file(&plan.staged_path).map_err(|error| {
        OnlyError::io_with_path(
            "failed to remove staged binary",
            plan.staged_path.clone(),
            error,
        )
    })?;

    print_upgrade_done(normalize_version(&latest_version));
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

fn fetch_latest_version() -> Result<String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let body = fetch_url_text(&url)?;
    parse_latest_release_tag(&body)
}

fn normalize_version(version: &str) -> &str {
    version.trim().trim_start_matches('v')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VersionOrder {
    Older,
    Equal,
    Newer,
}

fn compare_versions(current: &str, latest: &str) -> Result<VersionOrder> {
    let current = parse_version_parts(current)?;
    let latest = parse_version_parts(latest)?;

    Ok(match current.cmp(&latest) {
        std::cmp::Ordering::Less => VersionOrder::Older,
        std::cmp::Ordering::Equal => VersionOrder::Equal,
        std::cmp::Ordering::Greater => VersionOrder::Newer,
    })
}

fn parse_version_parts(version: &str) -> Result<(u64, u64, u64)> {
    let normalized = normalize_version(version);
    let mut parts = normalized.split('.');

    let major = parse_version_component(parts.next(), version, "major")?;
    let minor = parse_version_component(parts.next(), version, "minor")?;
    let patch = parse_version_component(parts.next(), version, "patch")?;

    if parts.next().is_some() {
        return Err(OnlyError::runtime(format!(
            "unsupported release version '{version}'; expected MAJOR.MINOR.PATCH"
        )));
    }

    Ok((major, minor, patch))
}

fn parse_version_component(component: Option<&str>, version: &str, label: &str) -> Result<u64> {
    let Some(component) = component else {
        return Err(OnlyError::runtime(format!(
            "unsupported release version '{version}'; missing {label} component"
        )));
    };

    component.parse::<u64>().map_err(|_| {
        OnlyError::runtime(format!(
            "unsupported release version '{version}'; invalid {label} component"
        ))
    })
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

    let powershell = windows_powershell_command()?;
    let script = format!(
        "$ErrorActionPreference = 'Stop'; $ProgressPreference = 'SilentlyContinue'; Invoke-WebRequest -Uri {} -UseBasicParsing | Select-Object -ExpandProperty Content",
        ps_literal(url),
    );
    run_text_command(
        powershell,
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

fn current_exe_is_install_path(install_path: &Path) -> bool {
    let Ok(current_exe) = env::current_exe() else {
        return true;
    };

    paths_equal(&current_exe, install_path)
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    let left = fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left.as_os_str().to_string_lossy().to_lowercase()
        == right.as_os_str().to_string_lossy().to_lowercase()
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

    let powershell = windows_powershell_command()?;
    download_with_powershell(powershell, url, output)
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

fn ensure_directory_writable(path: &Path, install_path: &Path) -> Result<()> {
    let probe = path.join(format!(".only-write-test-{}", std::process::id()));
    match OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            Ok(())
        }
        Err(error) => Err(OnlyError::io_with_path(
            install_not_writable_message(path, install_path),
            path.to_path_buf(),
            error,
        )),
    }
}

fn install_not_writable_message(path: &Path, install_path: &Path) -> &'static str {
    if path == Path::new("/usr/local/bin") && install_path == Path::new("/usr/local/bin/only") {
        "current install path is not writable; reinstall with sudo"
    } else {
        "install directory is not writable"
    }
}

#[cfg(windows)]
fn windows_powershell_command() -> Result<&'static str> {
    if command_exists("pwsh.exe") {
        return Ok("pwsh.exe");
    }
    if command_exists("powershell.exe") {
        return Ok("powershell.exe");
    }

    Err(OnlyError::runtime(
        "PowerShell is required to download the upgrade",
    ))
}

#[cfg(windows)]
fn ps_literal_path(path: &Path) -> String {
    ps_literal(&path.display().to_string())
}

#[cfg(windows)]
fn ps_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
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
    #[cfg(unix)]
    use super::is_unix_install_path;
    use super::{VersionOrder, compare_versions, latest_download_url, parse_latest_release_tag};
    #[cfg(unix)]
    use std::path::Path;

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
    fn parses_version_order_with_optional_v_prefix() {
        assert_eq!(
            compare_versions("v1.2.3", "1.2.3").expect("versions should parse"),
            VersionOrder::Equal
        );
    }

    #[test]
    fn rejects_downgrade_as_newer_local_version() {
        assert_eq!(
            compare_versions("1.2.4", "1.2.3").expect("versions should parse"),
            VersionOrder::Newer
        );
    }

    #[cfg(unix)]
    #[test]
    fn recognizes_common_unix_install_paths() {
        assert!(is_unix_install_path(Path::new("/usr/local/bin/only")));
    }
}
