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
    assert_eq!(items[0].insert_text, "!version ${1:0.3}");
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
