use only_syntax::{SyntaxSnapshot, snapshot};

use crate::SemanticSnapshot;
use crate::lower::lower_syntax;
use crate::symbols::build_symbol_index;
use crate::validate::validate_document;

/// Compiles source text into a semantic snapshot shared by hosts.
///
/// Args:
/// source: Raw Onlyfile source text.
///
/// Returns:
/// Immutable semantic snapshot with AST, diagnostics and symbols.
pub fn compile_document(source: &str) -> SemanticSnapshot {
    compile_syntax(&snapshot(source))
}

/// Compiles a pre-parsed syntax snapshot into a semantic snapshot shared by hosts.
///
/// Args:
/// snapshot: Immutable syntax snapshot for one source version.
///
/// Returns:
/// Immutable semantic snapshot with AST, diagnostics and symbols.
pub fn compile_syntax(snapshot: &SyntaxSnapshot) -> SemanticSnapshot {
    let (document, mut diagnostics) = lower_syntax(snapshot);
    let symbols = build_symbol_index(&document);
    diagnostics.extend(validate_document(&document, &symbols));

    SemanticSnapshot {
        document,
        diagnostics,
        symbols,
    }
}
