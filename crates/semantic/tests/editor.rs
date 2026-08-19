use only_semantic::{
    DocumentSymbolKind, FoldingRangeKind, compile_document, document_symbols, folding_ranges,
    hover_at,
};
use text_size::TextSize;

#[test]
fn builds_document_symbols_for_groups_and_tasks() {
    let compiled = compile_document(
        "# Developer commands.\ngroup dev {\n# Start the app.\nserve(port=\"3000\"):\n    echo {{port}}\n}\n",
    );

    let symbols = document_symbols(&compiled);

    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[0].kind, DocumentSymbolKind::Namespace);
    assert_eq!(symbols[0].name.as_str(), "dev");
    assert_eq!(symbols[1].kind, DocumentSymbolKind::Task);
    assert_eq!(symbols[1].name.as_str(), "serve");
    assert_eq!(symbols[1].container_name.as_deref(), Some("dev"));
}

#[test]
fn builds_folding_ranges_for_group_and_task_blocks() {
    let source = concat!(
        "!version 0.4\n",
        "group dev {\n",
        "    serve():\n",
        "        echo one\n",
        "        echo two\n",
        "    build():\n",
        "        cargo build\n",
        "}\n",
        "root():\n",
        "    true\n",
    );
    let compiled = compile_document(source);

    let ranges = folding_ranges(&compiled);
    let namespace = ranges
        .iter()
        .find(|range| range.kind == FoldingRangeKind::Namespace)
        .expect("group range should exist");
    let close_end = source.find("}\n").expect("close brace should exist") + 2;

    assert_eq!(
        usize::from(namespace.range.start()),
        source.find("group dev").expect("group should exist")
    );
    assert_eq!(usize::from(namespace.range.end()), close_end);
    assert!(
        ranges
            .iter()
            .any(|range| range.kind == FoldingRangeKind::Task)
    );
}

#[test]
fn builds_folding_range_for_command_block() {
    let compiled = compile_document(
        "!version 0.4\ntask():\n    | if true; then\n    |     echo ok\n    | fi\n",
    );

    assert!(
        folding_ranges(&compiled)
            .iter()
            .any(|range| { range.kind == FoldingRangeKind::CommandBlock })
    );
}

#[test]
fn returns_hover_for_task_at_offset() {
    let source = "!version 0.4\n[help] Start the app.\nserve(port=\"3000\"):\n    echo {{port}}\n";
    let compiled = compile_document(source);
    let offset = TextSize::from(source.find("serve").expect("task name should exist") as u32);

    let hover = hover_at(&compiled, offset).expect("hover should exist");

    assert_eq!(hover.name.as_str(), "serve");
    assert_eq!(hover.docs.as_deref(), Some("Start the app."));
    assert!(hover.signature.as_str().contains("serve(port=\"3000\")"));
}
