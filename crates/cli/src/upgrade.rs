//! Upgrades the installed `only` binary from GitHub release assets.
//!
//! Args:
//! None.
//!
//! Returns:
//! Verified download and platform-specific install helpers used by the CLI.
//!
//! Edge Cases:
//! Installed binaries replace themselves in place; ad hoc binaries copy into the install path.

use crate::error::{OnlyError, Result};
use self_replace::self_replace;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const REPO: &str = "KercyDing/only";
const CHECKSUMS_FILE: &str = "SHA256SUMS";
const HTTP_TIMEOUT_SECONDS: u64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpgradePlan {
    binary: &'static str,
    download_url: String,
    checksum_url: String,
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
    let checksum_url = latest_download_url(CHECKSUMS_FILE);
    let install_path = upgrade_install_path()?;
    let staged_path = staged_path_for(&install_path)?;

    Ok(UpgradePlan {
        binary,
        download_url,
        checksum_url,
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
    download_verified(plan)?;

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
    download_verified(plan)?;
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
            "invalid release version '{version}'\nexpected: A.B.C"
        )));
    }

    Ok((major, minor, patch))
}

fn parse_version_component(component: Option<&str>, version: &str, label: &str) -> Result<u64> {
    let Some(component) = component else {
        return Err(OnlyError::runtime(format!(
            "release version '{version}' has no {label} number"
        )));
    };

    component.parse::<u64>().map_err(|_| {
        OnlyError::runtime(format!(
            "release version '{version}' has an invalid {label} number"
        ))
    })
}

fn parse_latest_release_tag(body: &str) -> Result<String> {
    let Some(key_start) = body.find("\"tag_name\"") else {
        return Err(OnlyError::runtime("GitHub returned no release version"));
    };
    let after_key = &body[key_start + "\"tag_name\"".len()..];
    let Some(colon_start) = after_key.find(':') else {
        return Err(OnlyError::runtime("GitHub returned invalid release data"));
    };
    let after_colon = after_key[colon_start + 1..].trim_start();
    let Some(stripped) = after_colon.strip_prefix('"') else {
        return Err(OnlyError::runtime(
            "GitHub returned an invalid release version",
        ));
    };
    let Some(end) = stripped.find('"') else {
        return Err(OnlyError::runtime(
            "GitHub returned an invalid release version",
        ));
    };

    Ok(stripped[..end].to_string())
}

fn fetch_url_text(url: &str) -> Result<String> {
    let response = request(url)
        .send()
        .map_err(|error| request_error("fetch", url, error))?;
    ensure_success(response.status_code, &response.reason_phrase, url)?;
    response
        .as_str()
        .map(str::to_owned)
        .map_err(|error| OnlyError::runtime(format!("invalid UTF-8 response from {url}: {error}")))
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

fn download_verified(plan: &UpgradePlan) -> Result<()> {
    let checksums = fetch_url_text(&plan.checksum_url)?;
    let expected = parse_checksum(&checksums, plan.binary)?;
    let result = download_and_hash(&plan.download_url, &plan.staged_path);
    let actual = match result {
        Ok(actual) => actual,
        Err(error) => {
            let _ = fs::remove_file(&plan.staged_path);
            return Err(error);
        }
    };

    if actual != expected {
        let _ = fs::remove_file(&plan.staged_path);
        return Err(OnlyError::runtime(format!(
            "checksum mismatch for {}\nexpected: {expected}\nactual:   {actual}",
            plan.binary
        )));
    }

    Ok(())
}

fn download_and_hash(url: &str, output: &Path) -> Result<String> {
    let mut response = request(url)
        .send_lazy()
        .map_err(|error| request_error("download", url, error))?;
    ensure_success(response.status_code, &response.reason_phrase, url)?;

    let mut file = fs::File::create(output).map_err(|error| {
        OnlyError::io_with_path(
            "failed to create staged binary",
            output.to_path_buf(),
            error,
        )
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = response.read(&mut buffer).map_err(|error| {
            OnlyError::runtime(format!("failed to read download from {url}: {error}"))
        })?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read]).map_err(|error| {
            OnlyError::io_with_path("failed to write staged binary", output.to_path_buf(), error)
        })?;
        hasher.update(&buffer[..read]);
    }
    file.flush().map_err(|error| {
        OnlyError::io_with_path("failed to flush staged binary", output.to_path_buf(), error)
    })?;

    Ok(encode_hex(hasher.finalize().as_ref()))
}

fn encode_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[(byte >> 4) as usize] as char);
        encoded.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn parse_checksum(contents: &str, binary: &str) -> Result<String> {
    for line in contents.lines() {
        let mut fields = line.split_whitespace();
        let Some(checksum) = fields.next() else {
            continue;
        };
        let Some(name) = fields.next() else {
            continue;
        };
        if name.trim_start_matches('*') != binary {
            continue;
        }
        if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(OnlyError::runtime(format!(
                "invalid SHA-256 checksum for {binary}"
            )));
        }
        return Ok(checksum.to_ascii_lowercase());
    }

    Err(OnlyError::runtime(format!(
        "no SHA-256 checksum found for {binary}"
    )))
}

fn request(url: &str) -> minreq::Request {
    minreq::get(url)
        .with_header("User-Agent", concat!("only/", env!("CARGO_PKG_VERSION")))
        .with_timeout(HTTP_TIMEOUT_SECONDS)
}

fn ensure_success(status: u16, reason: &str, url: &str) -> Result<()> {
    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(OnlyError::runtime(format!(
            "request to {url} failed with HTTP {status} {reason}"
        )))
    }
}

fn request_error(action: &str, url: &str, error: minreq::Error) -> OnlyError {
    OnlyError::runtime(format!("failed to {action} {url}: {error}"))
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
        "the install directory is not writable\nhelp: run the upgrade with sudo"
    } else {
        "install directory is not writable"
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|error| {
            OnlyError::io_with_path(
                "failed to read the downloaded file",
                path.to_path_buf(),
                error,
            )
        })?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).map_err(|error| {
        OnlyError::io_with_path(
            "failed to make the downloaded file executable",
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

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::is_unix_install_path;
    use super::{
        VersionOrder, compare_versions, encode_hex, latest_download_url, parse_checksum,
        parse_latest_release_tag,
    };
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
    fn selects_platform_checksum() {
        let checksums = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  only-linux-amd64\n\
                         BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB *only-windows-amd64.exe\n";

        assert_eq!(
            parse_checksum(checksums, "only-windows-amd64.exe")
                .expect("platform checksum should parse"),
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        );
    }

    #[test]
    fn rejects_invalid_platform_checksum() {
        let error = parse_checksum("not-a-sha256  only-linux-amd64\n", "only-linux-amd64")
            .expect_err("invalid checksum should fail");

        assert_eq!(
            error.to_string(),
            "invalid SHA-256 checksum for only-linux-amd64"
        );
    }

    #[test]
    fn encodes_digest_bytes_as_lowercase_hex() {
        assert_eq!(encode_hex(&[0x00, 0xab, 0xff]), "00abff");
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
