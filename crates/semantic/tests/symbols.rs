use only_semantic::compile_document;

#[test]
fn builds_namespace_and_task_symbols() {
    let compiled = compile_document("[dev]\nserve():\n    cargo run\n");

    assert_eq!(compiled.symbols.namespaces.len(), 1);
    assert_eq!(compiled.symbols.namespaces[0].name, "dev");
    assert_eq!(compiled.symbols.tasks.len(), 1);
    assert_eq!(compiled.symbols.tasks[0].name, "dev.serve");
}
