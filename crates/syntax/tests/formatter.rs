use only_syntax::format_source;

#[test]
fn formatting_is_idempotent() {
    let source = "!version   0.3\n\n\nbuild():\n\t|echo one\n\t|\n\t|  echo two\n";
    let formatted = format_source(source).expect("valid source should format");

    assert_eq!(
        formatted,
        format_source(&formatted).expect("formatted source should parse")
    );
    assert!(formatted.contains("!version 0.3\n"));
    assert!(formatted.contains("    | echo one\n    |\n    |  echo two\n"));
}

#[test]
fn formatting_rejects_invalid_source() {
    assert!(format_source("broken(\n").is_err());
}
