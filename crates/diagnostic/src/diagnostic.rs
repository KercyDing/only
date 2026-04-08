use text_size::TextRange;

use crate::{DiagnosticCode, DiagnosticLabel, DiagnosticPhase, DiagnosticSeverity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub code: DiagnosticCode,
    pub primary_range: TextRange,
    pub secondary_ranges: Vec<TextRange>,
    pub labels: Vec<DiagnosticLabel>,
    pub phase: DiagnosticPhase,
}

impl Diagnostic {
    /// Creates a diagnostic with a single primary range.
    ///
    /// Args:
    /// severity: Host-facing severity level.
    /// code: Stable machine-readable code.
    /// message: Human-readable summary.
    /// phase: Pipeline stage that produced the issue.
    /// primary_range: Main highlighted source range.
    ///
    /// Returns:
    /// New diagnostic value.
    pub fn new(
        severity: DiagnosticSeverity,
        code: DiagnosticCode,
        message: impl Into<String>,
        phase: DiagnosticPhase,
        primary_range: TextRange,
    ) -> Self {
        Self {
            severity,
            message: message.into(),
            code,
            primary_range,
            secondary_ranges: Vec::new(),
            labels: Vec::new(),
            phase,
        }
    }
}
