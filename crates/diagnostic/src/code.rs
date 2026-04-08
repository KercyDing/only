use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticCode(String);

impl DiagnosticCode {
    /// Creates a stable diagnostic code.
    ///
    /// Args:
    /// value: Code text shared across hosts.
    ///
    /// Returns:
    /// New diagnostic code wrapper.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the raw diagnostic code text.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Borrowed diagnostic code string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
