use only_diagnostic::DiagnosticSeverity;
use text_size::TextRange;

use crate::DocumentSnapshot;

/// Host-facing diagnostic severity used by the LSP crate.
///
/// Args:
/// None.
///
/// Returns:
/// Stable severity categories for editor protocol conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspDiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Host-facing diagnostic payload for one in-memory document snapshot.
///
/// Args:
/// None.
///
/// Returns:
/// LSP-ready diagnostic fields detached from semantic internals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDiagnostic {
    pub severity: LspDiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub range: TextRange,
}

/// Converts one document snapshot into LSP-facing diagnostics.
///
/// Args:
/// snapshot: In-memory document snapshot with semantic diagnostics.
///
/// Returns:
/// Diagnostics mapped into LSP host-facing values.
pub fn diagnostics(snapshot: &DocumentSnapshot) -> Vec<LspDiagnostic> {
    snapshot
        .semantic
        .diagnostics
        .iter()
        .map(|diagnostic| LspDiagnostic {
            severity: map_severity(diagnostic.severity),
            code: diagnostic.code.as_str().to_owned(),
            message: diagnostic.message.clone(),
            range: diagnostic.primary_range,
        })
        .collect()
}

fn map_severity(severity: DiagnosticSeverity) -> LspDiagnosticSeverity {
    match severity {
        DiagnosticSeverity::Error => LspDiagnosticSeverity::Error,
        DiagnosticSeverity::Warning => LspDiagnosticSeverity::Warning,
        DiagnosticSeverity::Info => LspDiagnosticSeverity::Info,
        DiagnosticSeverity::Hint => LspDiagnosticSeverity::Hint,
    }
}
