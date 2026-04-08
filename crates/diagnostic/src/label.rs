use text_size::TextRange;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    pub range: TextRange,
    pub message: String,
}

impl DiagnosticLabel {
    /// Creates a diagnostic label for a specific text range.
    ///
    /// Args:
    /// range: Source range highlighted by the host.
    /// message: Short label rendered beside the range.
    ///
    /// Returns:
    /// New diagnostic label.
    pub fn new(range: TextRange, message: impl Into<String>) -> Self {
        Self {
            range,
            message: message.into(),
        }
    }
}
