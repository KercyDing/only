use only_lsp::{LspCompletionKind, completions};
use text_size::TextSize;

#[test]
fn completes_directives() {
    let source = "!";
    let items = completions(source, TextSize::of(source));

    assert_eq!(
        items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["!version", "!var", "!shell"]
    );
    assert!(
        items
            .iter()
            .all(|item| item.kind == LspCompletionKind::Directive)
    );
    let version = env!("CARGO_PKG_VERSION")
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");
    assert_eq!(items[0].insert_text, format!("!version ${{1:{version}}}"));
}

#[test]
fn filters_directives() {
    let source = "!v";
    let items = completions(source, TextSize::of(source));

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].label, "!version");
    assert_eq!(items[1].label, "!var");
    assert_eq!(usize::from(items[0].replace_range.start()), 0);
    assert_eq!(usize::from(items[0].replace_range.end()), source.len());
}

#[test]
fn completes_group_keyword() {
    let source = "g";
    let items = completions(source, TextSize::of(source));

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, LspCompletionKind::Keyword);
    assert_eq!(items[0].label, "group");
    assert_eq!(
        items[0].replace_range,
        text_size::TextRange::new(0.into(), 1.into())
    );
    assert!(items[0].insert_text.starts_with("group ${1:name}"));
}

#[test]
fn completes_dependencies() {
    let source = "build():\n    true\n\ngroup dev {\n    check():\n        true\n}\n\nci() & ";
    let items = completions(source, TextSize::of(source));

    assert_eq!(
        items
            .iter()
            .map(|item| (item.kind, item.label.as_str()))
            .collect::<Vec<_>>(),
        [
            (LspCompletionKind::Task, "build"),
            (LspCompletionKind::Group, "dev"),
        ]
    );
    assert_eq!(items[0].insert_text, "build");
}

#[test]
fn completes_chained_dependencies() {
    let source = "build():\n    true\n\ntest():\n    true\n\nci() & build &";
    let items = completions(source, TextSize::of(source));

    assert_eq!(
        items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["build", "test"]
    );
    assert!(items.iter().all(|item| item.insert_text.starts_with(' ')));
}

#[test]
fn completes_group_tasks() {
    let source =
        "group dev {\n    check():\n        true\n    test():\n        true\n}\n\nci() & dev.";
    let items = completions(source, TextSize::of(source));

    assert_eq!(
        items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["check", "test"]
    );
    assert!(
        items
            .iter()
            .all(|item| item.kind == LspCompletionKind::Task)
    );
    assert!(items.iter().all(|item| item.insert_text == item.label));
}

#[test]
fn ignores_command_ampersand() {
    let source = "build():\n    echo done & ";
    let items = completions(source, TextSize::of(source));

    assert!(items.is_empty());
}

#[test]
fn completes_guards() {
    let source = "build() ? @";
    let items = completions(source, TextSize::of(source));

    assert_eq!(
        items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["@os", "@arch", "@env", "@has"]
    );
    assert!(
        items
            .iter()
            .all(|item| item.kind == LspCompletionKind::Guard)
    );
    assert_eq!(items[3].insert_text, "@has(\"${1:command}\")");
}

#[test]
fn ignores_command_at_signs() {
    let source = "build():\n    echo user@";
    let items = completions(source, TextSize::of(source));

    assert!(items.is_empty());
}

#[test]
fn completes_metadata_fields() {
    let source = "[";
    let items = completions(source, TextSize::of(source));

    assert_eq!(
        items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["[help]", "[desc]", "[pass]", "[fail]"]
    );
    assert!(
        items
            .iter()
            .all(|item| item.kind == LspCompletionKind::Metadata)
    );
    assert_eq!(items[0].insert_text, "[help] ${1:text}");
}

#[test]
fn replaces_auto_closed_bracket() {
    let source = "[]";
    let items = completions(source, TextSize::from(1));

    assert_eq!(usize::from(items[0].replace_range.start()), 0);
    assert_eq!(usize::from(items[0].replace_range.end()), source.len());
    assert_eq!(items[0].insert_text, "[help] ${1:text}");
}

#[test]
fn replaces_metadata_inside_brackets() {
    let source = "[f] Done.";
    let items = completions(source, TextSize::from(2));

    assert_eq!(items[0].label, "[fail]");
    assert_eq!(items[0].insert_text, "[fail]");
    assert_eq!(usize::from(items[0].replace_range.start()), 0);
    assert_eq!(usize::from(items[0].replace_range.end()), 3);
}
