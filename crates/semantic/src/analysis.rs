use only_diagnostic::DiagnosticPhase;
use only_syntax::{BootstrapHeader, SyntaxSnapshot, bootstrap, snapshot};

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

/// Compiles source after checking its optional language version declaration.
pub fn compile_document_for_runner(source: &str, runner_version: &str) -> SemanticSnapshot {
    compile_document_with_runner(source, runner_version).1
}

/// Compiles source and retains the syntax snapshot created after the version gate.
pub fn compile_document_with_runner(
    source: &str,
    runner_version: &str,
) -> (SyntaxSnapshot, SemanticSnapshot) {
    let header = match bootstrap(source, runner_version) {
        Ok(header) => header,
        Err(diagnostic) => return snapshot_with_version_error(diagnostic),
    };
    let syntax = snapshot(source);
    let mut compiled = compile_syntax(&syntax);
    append_unversioned_parse_help(&mut compiled, header);
    (syntax, compiled)
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

fn snapshot_with_version_error(
    diagnostic: only_diagnostic::Diagnostic,
) -> (SyntaxSnapshot, SemanticSnapshot) {
    let syntax = snapshot("");
    let mut compiled = compile_syntax(&syntax);
    compiled.diagnostics.push(diagnostic);
    (syntax, compiled)
}

fn append_unversioned_parse_help(compiled: &mut SemanticSnapshot, header: BootstrapHeader) {
    if header.required_version.is_some() {
        return;
    }

    if let Some(diagnostic) = compiled
        .diagnostics
        .iter_mut()
        .find(|diagnostic| diagnostic.phase == DiagnosticPhase::Parse)
    {
        diagnostic.message.push_str(
            "\nnote: this Onlyfile has no `!version` declaration\nhelp: the syntax may require an Onlyfile language version not recognized by this runner",
        );
    }
}
