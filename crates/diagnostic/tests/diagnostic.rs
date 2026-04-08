use only_diagnostic::{Diagnostic, DiagnosticCode, DiagnosticPhase, DiagnosticSeverity};
use text_size::{TextRange, TextSize};

#[test]
fn creates_diagnostic_value() {
    let diagnostic = Diagnostic::new(
        DiagnosticSeverity::Error,
        DiagnosticCode::new("parse.unexpected-token"),
        "unexpected token",
        DiagnosticPhase::Parse,
        TextRange::new(TextSize::from(0), TextSize::from(4)),
    );

    assert_eq!(diagnostic.code.as_str(), "parse.unexpected-token");
    assert_eq!(diagnostic.phase, DiagnosticPhase::Parse);
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
}
