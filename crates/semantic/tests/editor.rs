use only_semantic::{
    DocumentSymbolKind, FoldingRangeKind, compile_document, document_symbols, folding_ranges,
    hover_at,
};
use text_size::TextSize;

#[test]
fn builds_document_symbols_for_namespaces_and_tasks() {
    let compiled = compile_document(
        "% Developer commands.\n[dev]\n% Start the app.\nserve(port=\"3000\"):\n    echo {{port}}\n",
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
fn builds_folding_ranges_for_namespace_and_task_blocks() {
    let compiled = compile_document(
        "[dev]\nserve():\n    echo one\n    echo two\nbuild():\n    cargo build\n",
    );

    let ranges = folding_ranges(&compiled);

    assert!(
        ranges
            .iter()
            .any(|range| range.kind == FoldingRangeKind::Namespace)
    );
    assert!(
        ranges
            .iter()
            .any(|range| range.kind == FoldingRangeKind::Task)
    );
}

#[test]
fn returns_hover_for_task_at_offset() {
    let source = "% Start the app.\nserve(port=\"3000\"):\n    echo {{port}}\n";
    let compiled = compile_document(source);
    let offset = TextSize::from(source.find("serve").expect("task name should exist") as u32);

    let hover = hover_at(&compiled, offset).expect("hover should exist");

    assert_eq!(hover.name.as_str(), "serve");
    assert_eq!(hover.docs.as_deref(), Some("Start the app."));
    assert!(hover.signature.as_str().contains("serve(port=\"3000\")"));
}
