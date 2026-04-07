use only::{Directive, parse_onlyfile};

#[test]
fn parses_empty_onlyfile() {
    let document = parse_onlyfile("").expect("empty Onlyfile should parse");
    assert!(document.directives.is_empty());
    assert!(document.global_tasks.is_empty());
    assert!(document.namespaces.is_empty());
}

#[test]
fn parses_minimal_document_shape() {
    let source = "!verbose false\nhello():\n    echo hello\n[tools]\nfmt():\n    cargo fmt\n";
    let document = parse_onlyfile(source).expect("minimal document should parse");

    assert!(matches!(
        document.directives[0],
        Directive::Verbose { value: false, .. }
    ));
    assert_eq!(document.global_tasks[0].signature.name, "hello");
    assert_eq!(document.global_tasks[0].commands[0].text, "echo hello");
    assert_eq!(document.namespaces[0].name, "tools");
    assert_eq!(document.namespaces[0].tasks[0].signature.name, "fmt");
}
