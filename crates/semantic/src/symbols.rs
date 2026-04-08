use smol_str::SmolStr;
use text_size::TextRange;

use crate::DocumentAst;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceSymbol {
    pub name: SmolStr,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSymbol {
    pub name: SmolStr,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SymbolIndex {
    pub namespaces: Vec<NamespaceSymbol>,
    pub tasks: Vec<TaskSymbol>,
}

pub(crate) fn build_symbol_index(document: &DocumentAst) -> SymbolIndex {
    let mut symbols = SymbolIndex::default();

    for namespace in &document.namespaces {
        symbols.namespaces.push(NamespaceSymbol {
            name: namespace.name.clone(),
            range: namespace.range,
        });
    }

    for task in &document.tasks {
        symbols.tasks.push(TaskSymbol {
            name: task.qualified_name(),
            range: task.range,
        });
    }

    symbols
}
