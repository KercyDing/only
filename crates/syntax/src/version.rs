use only_diagnostic::{Diagnostic, DiagnosticCode, DiagnosticPhase, DiagnosticSeverity};
use text_size::{TextRange, TextSize};

const UTF8_BOM: &str = "\u{feff}";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionRequirement {
    pub major: u64,
    pub minor: u64,
    pub span: TextRange,
}

impl VersionRequirement {
    pub fn required_range(&self) -> String {
        let upper_major = self
            .major
            .checked_add(1)
            .expect("validated version requirement must have an upper bound");
        format!(">={}.{}.0, <{upper_major}.0.0", self.major, self.minor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapHeader {
    pub required_version: Option<VersionRequirement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RunnerVersion {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: bool,
}

/// Scans the optional version declaration at the start of an Onlyfile.
pub fn scan_bootstrap_header(source: &str) -> Result<BootstrapHeader, Diagnostic> {
    let mut offset = source.strip_prefix(UTF8_BOM).map_or(0, |_| UTF8_BOM.len());

    while offset < source.len() {
        let line_end = source[offset..]
            .find(['\r', '\n'])
            .map_or(source.len(), |end| offset + end);
        let line = &source[offset..line_end];
        let leading = line.len() - line.trim_start_matches([' ', '\t']).len();
        let declaration = &line[leading..];

        if declaration.is_empty() || declaration.starts_with("//") {
            offset = next_line_offset(source, line_end);
            continue;
        }

        if !is_version_directive(declaration) {
            return Ok(BootstrapHeader {
                required_version: None,
            });
        }

        let span = text_range(offset + leading, line_end);
        let value = declaration
            .strip_prefix("!version")
            .expect("version directive prefix was checked")
            .strip_prefix([' ', '\t'])
            .map(str::trim)
            .unwrap_or_default();
        let required_version = parse_version_requirement(value, span)?;
        return Ok(BootstrapHeader {
            required_version: Some(required_version),
        });
    }

    Ok(BootstrapHeader {
        required_version: None,
    })
}

/// Checks the optional Onlyfile version declaration against a runner SemVer.
pub fn check_version_compatibility(
    header: &BootstrapHeader,
    runner_version: &str,
) -> Result<(), Diagnostic> {
    let Some(requirement) = header.required_version else {
        return Ok(());
    };
    let runner = parse_runner_version(runner_version).ok_or_else(|| {
        version_error(
            "version.invalid-runner-version",
            format!("only has an invalid version: '{runner_version}'"),
            DiagnosticPhase::Host,
            requirement.span,
        )
    })?;

    let compatible = !runner.prerelease
        && runner.major == requirement.major
        && (runner.minor, runner.patch) >= (requirement.minor, 0);
    if compatible {
        return Ok(());
    }

    let help = if runner.prerelease
        || runner.major < requirement.major
        || (runner.major == requirement.major && runner.minor < requirement.minor)
    {
        "run `only --upgrade`".to_string()
    } else {
        format!(
            "install `only` {}.x or change `!version`",
            requirement.major
        )
    };
    Err(version_error(
        "version.incompatible",
        format!(
            "this Onlyfile needs `only` {}.{} or newer (not {}.x)\ninstalled: {runner_version}\nneeded: {}\nhelp: {help}",
            requirement.major,
            requirement.minor,
            requirement.major + 1,
            requirement.required_range(),
        ),
        DiagnosticPhase::Host,
        requirement.span,
    ))
}

/// Runs the header scan and compatibility check without parsing the full file.
pub fn bootstrap(source: &str, runner_version: &str) -> Result<BootstrapHeader, Diagnostic> {
    let header = scan_bootstrap_header(source)?;
    check_version_compatibility(&header, runner_version)?;
    Ok(header)
}

/// Parses the two-segment version value used by a `!version` directive.
pub fn parse_version_requirement(
    value: &str,
    span: TextRange,
) -> Result<VersionRequirement, Diagnostic> {
    let Some((major, minor)) = value.split_once('.') else {
        return Err(invalid_format(span));
    };
    if major.is_empty() || minor.is_empty() || minor.contains('.') {
        return Err(invalid_format(span));
    }
    if !valid_component(major) || !valid_component(minor) {
        return Err(invalid_format(span));
    }

    let major = major.parse::<u64>().map_err(|_| range_overflow(span))?;
    let minor = minor.parse::<u64>().map_err(|_| range_overflow(span))?;
    if major == 0 && minor == 0 {
        return Err(version_error(
            "version.pre-0.1-unsupported",
            "`!version 0.0` is not allowed\nhelp: use `!version 0.1` or remove this line",
            DiagnosticPhase::Parse,
            span,
        ));
    }
    major.checked_add(1).ok_or_else(|| range_overflow(span))?;

    Ok(VersionRequirement { major, minor, span })
}

fn is_version_directive(declaration: &str) -> bool {
    declaration == "!version"
        || declaration
            .strip_prefix("!version")
            .is_some_and(|rest| rest.starts_with([' ', '\t']))
}

fn valid_component(component: &str) -> bool {
    component.bytes().all(|byte| byte.is_ascii_digit())
        && (component == "0" || !component.starts_with('0'))
}

fn parse_runner_version(version: &str) -> Option<RunnerVersion> {
    let (without_build, build) = split_suffix(version, '+', false)?;
    if let Some(build) = build
        && !valid_identifiers(build, false)
    {
        return None;
    }
    let (core, prerelease) = split_suffix(without_build, '-', true)?;
    if let Some(prerelease) = prerelease
        && !valid_identifiers(prerelease, true)
    {
        return None;
    }

    let mut components = core.split('.');
    let major = parse_runner_component(components.next()?)?;
    let minor = parse_runner_component(components.next()?)?;
    let patch = parse_runner_component(components.next()?)?;
    if components.next().is_some() {
        return None;
    }

    Some(RunnerVersion {
        major,
        minor,
        patch,
        prerelease: prerelease.is_some(),
    })
}

fn split_suffix(
    input: &str,
    separator: char,
    allow_separator_in_suffix: bool,
) -> Option<(&str, Option<&str>)> {
    match input.split_once(separator) {
        Some((left, right)) if !left.is_empty() && !right.is_empty() => {
            if !allow_separator_in_suffix && right.contains(separator) {
                None
            } else {
                Some((left, Some(right)))
            }
        }
        Some(_) => None,
        None => Some((input, None)),
    }
}

fn parse_runner_component(component: &str) -> Option<u64> {
    valid_component(component)
        .then(|| component.parse::<u64>().ok())
        .flatten()
}

fn valid_identifiers(input: &str, reject_numeric_leading_zero: bool) -> bool {
    input.split('.').all(|identifier| {
        !identifier.is_empty()
            && identifier
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            && (!reject_numeric_leading_zero
                || !identifier.bytes().all(|byte| byte.is_ascii_digit())
                || identifier == "0"
                || !identifier.starts_with('0'))
    })
}

fn next_line_offset(source: &str, line_end: usize) -> usize {
    match source.as_bytes().get(line_end..) {
        Some([b'\r', b'\n', ..]) => line_end + 2,
        Some([b'\r' | b'\n', ..]) => line_end + 1,
        _ => line_end,
    }
}

fn text_range(start: usize, end: usize) -> TextRange {
    TextRange::new(TextSize::from(start as u32), TextSize::from(end as u32))
}

fn invalid_format(span: TextRange) -> Diagnostic {
    version_error(
        "version.invalid-format",
        "use `!version A.B`, for example `!version 0.1`",
        DiagnosticPhase::Parse,
        span,
    )
}

fn range_overflow(span: TextRange) -> Diagnostic {
    version_error(
        "version.range-overflow",
        "the version number is too large",
        DiagnosticPhase::Parse,
        span,
    )
}

fn version_error(
    code: &str,
    message: impl Into<String>,
    phase: DiagnosticPhase,
    span: TextRange,
) -> Diagnostic {
    Diagnostic::new(
        DiagnosticSeverity::Error,
        DiagnosticCode::new(code),
        message,
        phase,
        span,
    )
}
