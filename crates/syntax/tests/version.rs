use only_syntax::{
    DiagnosticCode, bootstrap, check_version_compatibility, scan_bootstrap_header, snapshot,
};

fn diagnostic_code(source: &str) -> DiagnosticCode {
    scan_bootstrap_header(source)
        .expect_err("source should have an invalid version declaration")
        .code
}

#[test]
fn scans_header_trivia_and_span() {
    let source = "\u{feff}\r\n// project tasks\r\n  !version 0.12\r\nbuild():\r\n    true\r\n";
    let header = scan_bootstrap_header(source).expect("header should scan");
    let requirement = header.required_version.expect("version should be present");
    let span = &source[usize::from(requirement.span.start())..usize::from(requirement.span.end())];

    assert_eq!((requirement.major, requirement.minor), (0, 12));
    assert_eq!(span, "!version 0.12");
}

#[test]
fn accepts_only_two_segments() {
    for source in [
        "!version 0\n",
        "!version 1\n",
        "!version 1.2.3\n",
        "!version 01.2\n",
        "!version 1.02\n",
        "!version ^1.2\n",
        "!version >=1.2,<2.0\n",
        "!version 1.2.x\n",
        "!version\n",
    ] {
        assert_eq!(
            diagnostic_code(source),
            DiagnosticCode::new("version.invalid-format"),
            "unexpected diagnostic for {source:?}"
        );
    }
}

#[test]
fn rejects_undefined_and_overflowing_ranges() {
    assert_eq!(
        diagnostic_code("!version 0.0\n"),
        DiagnosticCode::new("version.pre-0.1-unsupported")
    );
    assert_eq!(
        diagnostic_code("!version 18446744073709551616.1\n"),
        DiagnosticCode::new("version.range-overflow")
    );
    assert_eq!(
        diagnostic_code("!version 18446744073709551615.1\n"),
        DiagnosticCode::new("version.range-overflow")
    );
}

#[test]
fn skips_gate_when_first_declaration_is_not_version() {
    for source in [
        "# task docs\n!version 0.1\n",
        "!shell bash\n!version 0.1\n",
        "build():\n    true\n!version 0.1\n",
    ] {
        let header = scan_bootstrap_header(source).expect("header should scan");
        assert!(header.required_version.is_none());
    }
}

#[test]
fn checks_language_capability_range() {
    let header = scan_bootstrap_header("!version 0.1\n").expect("header should scan");

    for runner in ["0.1.0", "0.2.0", "0.9.7", "0.9.7+build.2"] {
        check_version_compatibility(&header, runner)
            .unwrap_or_else(|_| panic!("runner {runner} should be compatible"));
    }
    for runner in ["0.0.9", "1.0.0", "0.1.0-alpha.1"] {
        let diagnostic = check_version_compatibility(&header, runner)
            .expect_err("runner should be incompatible");
        assert_eq!(diagnostic.code, DiagnosticCode::new("version.incompatible"));
    }
}

#[test]
fn validates_runner_semver() {
    let header = scan_bootstrap_header("!version 1.2\n").expect("header should scan");

    check_version_compatibility(&header, "1.2.3-alpha-beta+build.7")
        .expect_err("prerelease runner should be incompatible");
    for runner in ["1.2", "01.2.3", "1.2.3+", "1.2.3-alpha.01"] {
        let diagnostic = check_version_compatibility(&header, runner)
            .expect_err("runner version should be invalid");
        assert_eq!(
            diagnostic.code,
            DiagnosticCode::new("version.invalid-runner-version"),
            "unexpected diagnostic for {runner}"
        );
    }
}

#[test]
fn reparses_compatible_source_from_start() {
    let source = "\u{feff}// tasks\n!version 1.2\n\nbuild():\n    true\n";
    bootstrap(source, "1.8.4+build.2").expect("version should be compatible");
    let syntax = snapshot(source);
    let directive = syntax
        .document()
        .directives()
        .next()
        .expect("version directive should remain in CST");

    assert_eq!(syntax.root().text().to_string(), source);
    assert_eq!(directive.name().as_deref(), Some("version"));
    assert_eq!(directive.raw_value().as_deref(), Some("1.2"));
    assert!(syntax.diagnostics().is_empty());
}
