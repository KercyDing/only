use only::{Directive, ShellKind, parse_onlyfile};

#[test]
fn parses_empty_onlyfile() {
    let document = parse_onlyfile("").expect("empty Onlyfile should parse");
    assert!(document.directives.is_empty());
    assert!(document.global_tasks.is_empty());
    assert!(document.namespaces.is_empty());
}

#[test]
fn parses_minimal_document_shape() {
    let source =
        "!verbose false\n!shell sh\nhello():\n    echo hello\n[tools]\nfmt():\n    cargo fmt\n";
    let document = parse_onlyfile(source).expect("minimal document should parse");

    assert!(matches!(
        document.directives[0],
        Directive::Verbose { value: false, .. }
    ));
    assert!(matches!(
        document.directives[1],
        Directive::Shell {
            shell: ShellKind::Sh,
            ..
        }
    ));
    assert_eq!(document.global_tasks[0].signature.name, "hello");
    assert_eq!(document.global_tasks[0].commands[0].text, "echo hello");
    assert_eq!(document.namespaces[0].name, "tools");
    assert_eq!(document.namespaces[0].tasks[0].signature.name, "fmt");
}

#[test]
fn rejects_ambiguous_guards() {
    let source = "build() ? @os(\"linux\"):
    echo one

build() ? @os(\"linux\"):
    echo two
";
    let error = parse_onlyfile(source).expect_err("ambiguous guards should fail");
    assert_eq!(
        error.to_string(),
        "ambiguous guard: 'build' conflicts with 'build'"
    );
}

#[test]
fn rejects_duplicate_parameter_names() {
    let source = "build(tag, tag=\"v1\"):
    echo build
";
    let error = parse_onlyfile(source).expect_err("duplicate parameters should fail");
    assert_eq!(
        error.to_string(),
        "duplicate parameter 'tag' in task 'build'"
    );
}

#[test]
fn assigns_following_tasks_to_current_namespace() {
    let source = "[frontend]
build():
    npm run build

test():
    npm test

[backend]
serve():
    cargo run
";
    let document = parse_onlyfile(source).expect("namespaced tasks should parse");

    assert!(document.global_tasks.is_empty());
    assert_eq!(document.namespaces.len(), 2);
    assert_eq!(document.namespaces[0].name, "frontend");
    assert_eq!(document.namespaces[0].tasks.len(), 2);
    assert_eq!(document.namespaces[0].tasks[0].signature.name, "build");
    assert_eq!(document.namespaces[0].tasks[1].signature.name, "test");
    assert_eq!(document.namespaces[1].name, "backend");
    assert_eq!(document.namespaces[1].tasks.len(), 1);
    assert_eq!(document.namespaces[1].tasks[0].signature.name, "serve");
}

#[test]
fn does_not_assign_namespace_doc_to_first_task() {
    let source = "% Developer workflow.\n[dev]\nsmoke():\n    echo smoke\n";
    let document = parse_onlyfile(source).expect("namespaced tasks should parse");

    assert_eq!(
        document.namespaces[0].doc.as_deref(),
        Some("Developer workflow.")
    );
    assert_eq!(document.namespaces[0].tasks[0].signature.name, "smoke");
    assert!(document.namespaces[0].tasks[0].doc.is_none());
}
