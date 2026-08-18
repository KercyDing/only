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
//! Cargo installs are updated through Cargo so its install records stay correct. System package
//! installs defer to the package manager.

use crate::error::{OnlyError, Result};
use self_replace::self_replace;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const REPO: &str = "KercyDing/only";
const VERSION_FILE: &str = "VERSION";
const CHECKSUMS_FILE: &str = "SHA256SUMS";
const HTTP_TIMEOUT_SECONDS: u64 = 60;

#[cfg(windows)]
const CARGO_BINARY_NAME: &str = "only.exe";
#[cfg(not(windows))]
const CARGO_BINARY_NAME: &str = "only";

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpgradePlan {
    binary: &'static str,
    install_path: PathBuf,
    staged_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LatestRelease {
    version: String,
    download_url: String,
    checksum: String,
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
    let latest_version = fetch_latest_version()?;
    if compare_versions(env!("CARGO_PKG_VERSION"), &latest_version)? != VersionOrder::Older {
        println!("Already up to date.");
        return Ok(ExitCode::SUCCESS);
    }

    if current_exe_is_cargo_install() {
        return run_cargo_upgrade(&latest_version);
    }

    let plan = build_upgrade_plan()?;
    execute_upgrade(&plan, latest_version)?;
    Ok(ExitCode::SUCCESS)
}

fn run_cargo_upgrade(latest_version: &str) -> Result<ExitCode> {
    println!(
        "Update available: {} -> {}",
        env!("CARGO_PKG_VERSION"),
        normalize_version(latest_version)
    );

    if !io::stdin().is_terminal() {
        return Err(OnlyError::runtime(
            "only was installed with Cargo\nhelp: run `cargo install only --force`",
        ));
    }

    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    if !confirm_cargo_upgrade(&mut input, &mut output)? {
        println!("Cancelled.");
        return Ok(ExitCode::SUCCESS);
    }

    execute_cargo_upgrade()
}

fn confirm_cargo_upgrade(input: &mut impl BufRead, output: &mut impl Write) -> Result<bool> {
    loop {
        write!(
            output,
            "This copy was installed with Cargo.\nRun `cargo install only --force`? [Y/n] "
        )
        .map_err(|error| OnlyError::runtime(format!("failed to write prompt: {error}")))?;
        output
            .flush()
            .map_err(|error| OnlyError::runtime(format!("failed to show prompt: {error}")))?;

        let mut answer = String::new();
        let bytes_read = input
            .read_line(&mut answer)
            .map_err(|error| OnlyError::runtime(format!("failed to read answer: {error}")))?;
        if bytes_read == 0 {
            return Ok(false);
        }

        match answer.trim().to_ascii_lowercase().as_str() {
            "" | "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => writeln!(output, "Enter y or n.")
                .map_err(|error| OnlyError::runtime(format!("failed to write prompt: {error}")))?,
        }
    }
}

fn current_exe_is_cargo_install() -> bool {
    let Ok(current_exe) = env::current_exe() else {
        return false;
    };
    let Some(cargo_home) = cargo_home() else {
        return false;
    };

    is_cargo_install_path(&current_exe, &cargo_home)
}

fn cargo_home() -> Option<PathBuf> {
    env::var_os("CARGO_HOME").map(PathBuf::from).or_else(|| {
        env::var_os("HOME")
            .or_else(|| env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .map(|home| home.join(".cargo"))
    })
}

fn is_cargo_install_path(executable: &Path, cargo_home: &Path) -> bool {
    executable.file_name().and_then(|name| name.to_str()) == Some(CARGO_BINARY_NAME)
        && executable
            .parent()
            .is_some_and(|parent| paths_equal(parent, &cargo_home.join("bin")))
}

fn execute_cargo_upgrade() -> Result<ExitCode> {
    let status = Command::new("cargo")
        .args(["install", "only", "--force"])
        .status()
        .map_err(|error| OnlyError::runtime(format!("failed to start Cargo: {error}")))?;

    if status.success() {
        Ok(ExitCode::SUCCESS)
    } else {
        Err(OnlyError::runtime(format!(
            "Cargo update failed with {status}"
        )))
    }
}

fn build_upgrade_plan() -> Result<UpgradePlan> {
    let binary = current_platform_binary()?;
    let install_path = upgrade_install_path()?;
    let staged_path = staged_path_for(&install_path)?;

    Ok(UpgradePlan {
        binary,
        install_path,
        staged_path,
    })
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
    if let Ok(current_exe) = env::current_exe() {
        if is_system_package_install_path(&current_exe) {
            return Err(OnlyError::runtime(
                "only was installed by a system package manager\nhelp: update it with your package manager",
            ));
        }
        if is_unix_install_path(&current_exe) {
            return Ok(current_exe);
        }
    }

    default_install_path()
}

#[cfg(any(unix, test))]
fn is_system_package_install_path(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("only")
        && path.parent() == Some(Path::new("/usr/bin"))
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
fn execute_upgrade(plan: &UpgradePlan, latest_version: String) -> Result<()> {
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

    let release = fetch_release(plan.binary, latest_version)?;

    ensure_directory_writable(install_dir, &plan.install_path)?;
    print_download_summary(plan, &release.download_url);
    download_verified(plan, &release)?;

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

    print_upgrade_done(normalize_version(&release.version));
    Ok(())
}

#[cfg(unix)]
fn execute_upgrade(plan: &UpgradePlan, latest_version: String) -> Result<()> {
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

    let release = fetch_release(plan.binary, latest_version)?;

    ensure_directory_writable(install_dir, &plan.install_path)?;
    print_download_summary(plan, &release.download_url);
    download_verified(plan, &release)?;
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

    print_upgrade_done(normalize_version(&release.version));
    Ok(())
}

fn print_download_summary(plan: &UpgradePlan, download_url: &str) {
    println!("Downloading from: {download_url}");
    println!("Installing to: {}", plan.install_path.display());
}

fn print_upgrade_done(installed_version: &str) {
    println!("{} -> {installed_version}", env!("CARGO_PKG_VERSION"));
    println!("Done.");
}

fn fetch_release(binary: &str, version: String) -> Result<LatestRelease> {
    let checksum_url = release_download_url(&version, CHECKSUMS_FILE);
    let checksums = fetch_url_text(&checksum_url)?;

    Ok(LatestRelease {
        download_url: release_download_url(&version, binary),
        checksum: parse_checksum(&checksums, binary)?,
        version,
    })
}

fn fetch_latest_version() -> Result<String> {
    let version = fetch_url_text(&latest_download_url(VERSION_FILE))?;
    parse_release_version(&version)
}

fn latest_download_url(asset: &str) -> String {
    format!("https://github.com/{REPO}/releases/latest/download/{asset}")
}

fn release_download_url(version: &str, asset: &str) -> String {
    format!("https://github.com/{REPO}/releases/download/{version}/{asset}")
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

fn parse_release_version(contents: &str) -> Result<String> {
    let version = contents.trim();
    parse_version_parts(version)?;
    Ok(version.to_owned())
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

    #[cfg(windows)]
    {
        left.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
    }

    #[cfg(not(windows))]
    {
        left == right
    }
}

fn download_verified(plan: &UpgradePlan, release: &LatestRelease) -> Result<()> {
    let result = download_and_hash(&release.download_url, &plan.staged_path);
    let actual = match result {
        Ok(actual) => actual,
        Err(error) => {
            let _ = fs::remove_file(&plan.staged_path);
            return Err(error);
        }
    };

    if actual != release.checksum {
        let _ = fs::remove_file(&plan.staged_path);
        return Err(OnlyError::runtime(format!(
            "checksum mismatch for {}\nexpected: {}\nactual:   {actual}",
            plan.binary, release.checksum
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
        CARGO_BINARY_NAME, VersionOrder, compare_versions, confirm_cargo_upgrade, encode_hex,
        is_cargo_install_path, is_system_package_install_path, parse_checksum,
        parse_release_version, release_download_url,
    };
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    #[test]
    fn parses_release_version() {
        assert_eq!(
            parse_release_version("  v0.0.8\n").expect("release version should parse"),
            "v0.0.8"
        );
        assert_eq!(
            release_download_url("v0.0.8", "only-windows-amd64.exe"),
            "https://github.com/KercyDing/only/releases/download/v0.0.8/only-windows-amd64.exe"
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
    fn encodes_digest_hex() {
        assert_eq!(encode_hex(&[0x00, 0xab, 0xff]), "00abff");
    }

    #[test]
    fn compares_version_prefix() {
        assert_eq!(
            compare_versions("v1.2.3", "1.2.3").expect("versions should parse"),
            VersionOrder::Equal
        );
    }

    #[test]
    fn rejects_version_downgrade() {
        assert_eq!(
            compare_versions("1.2.4", "1.2.3").expect("versions should parse"),
            VersionOrder::Newer
        );
    }

    #[cfg(unix)]
    #[test]
    fn recognizes_unix_paths() {
        assert!(is_unix_install_path(Path::new("/usr/local/bin/only")));
    }

    #[test]
    fn recognizes_package_path() {
        assert!(is_system_package_install_path(Path::new("/usr/bin/only")));
        assert!(!is_system_package_install_path(Path::new("/usr/bin/other")));
    }

    #[test]
    fn recognizes_cargo_path() {
        let cargo_home = PathBuf::from("cargo-home");
        let executable = cargo_home.join("bin").join(CARGO_BINARY_NAME);

        assert!(is_cargo_install_path(&executable, &cargo_home));
        assert!(!is_cargo_install_path(
            &cargo_home.join("other").join(CARGO_BINARY_NAME),
            &cargo_home
        ));
    }

    #[test]
    fn accepts_cargo_confirmation() {
        for answer in ["\n", "y\n", "YES\n"] {
            let mut input = Cursor::new(answer.as_bytes());
            let mut output = Vec::new();

            assert!(
                confirm_cargo_upgrade(&mut input, &mut output)
                    .expect("confirmation should be read")
            );
        }
    }

    #[test]
    fn declines_cargo_confirmation() {
        for answer in ["n\n", "No\n"] {
            let mut input = Cursor::new(answer.as_bytes());
            let mut output = Vec::new();

            assert!(
                !confirm_cargo_upgrade(&mut input, &mut output)
                    .expect("confirmation should be read")
            );
        }
    }

    #[test]
    fn retries_cargo_confirmation() {
        let mut input = Cursor::new(b"maybe\n\n");
        let mut output = Vec::new();

        assert!(
            confirm_cargo_upgrade(&mut input, &mut output).expect("confirmation should be read")
        );
        assert!(
            String::from_utf8(output)
                .expect("prompt output should be UTF-8")
                .contains("Enter y or n.")
        );
    }
}
