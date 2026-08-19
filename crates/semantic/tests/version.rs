use only_semantic::{DirectiveAst, compile_document, compile_document_for_runner};

fn diagnostic_codes(source: &str) -> Vec<String> {
    compile_document(source)
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str().to_owned())
        .collect()
}

#[test]
fn lowers_version_directive() {
    let compiled = compile_document("!version 1.2\n\nbuild():\n    true\n");

    assert!(compiled.diagnostics.is_empty());
    assert!(matches!(
        compiled.document.directives.first(),
        Some(DirectiveAst::Version {
            major: 1,
            minor: 2,
            ..
        })
    ));
}

#[test]
fn reports_misplaced_version() {
    for source in [
        "!shell bash\n!version 0.1\n",
        "group dev {\n!version 0.1\n}\n",
        "build():\n    true\n!version 0.1\n",
    ] {
        assert!(
            diagnostic_codes(source)
                .iter()
                .any(|code| code == "version.not-first-declaration"),
            "missing placement diagnostic for {source:?}"
        );
    }
}

#[test]
fn reports_duplicate_version() {
    let codes = diagnostic_codes("!version 0.1\n!version 0.2\n");

    assert!(codes.iter().any(|code| code == "version.duplicate"));
    assert!(
        !codes
            .iter()
            .any(|code| code == "version.not-first-declaration")
    );
}

#[test]
fn gates_before_full_parse() {
    let compiled = compile_document_for_runner("!version 1.2\n@\n", "1.1.9");

    assert_eq!(compiled.diagnostics.len(), 1);
    assert_eq!(
        compiled.diagnostics[0].code.as_str(),
        "version.incompatible"
    );
    assert!(compiled.document.tasks.is_empty());
}

#[test]
fn compiles_after_compatible_gate() {
    let compiled = compile_document_for_runner(
        "\u{feff}// tasks\r\n!version 0.1\r\n\r\nbuild():\r\n    true\r\n",
        "0.8.0",
    );

    assert!(compiled.diagnostics.is_empty());
    assert_eq!(compiled.document.tasks[0].name, "build");
}

#[test]
fn annotates_unversioned_parse_failure() {
    let compiled = compile_document_for_runner("@\n", "0.1.0");
    let diagnostic = compiled
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "parse.unexpected-token")
        .expect("parse diagnostic should remain");

    assert!(diagnostic.message.starts_with("unexpected text"));
    assert!(diagnostic.message.contains("has no `!version` line"));
    assert!(diagnostic.message.contains("help:"));
}
