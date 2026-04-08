#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticPhase {
    Lex,
    Parse,
    Lower,
    Semantic,
    Engine,
    Host,
}
