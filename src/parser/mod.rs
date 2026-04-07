mod grammar;
mod lexer;
mod validate;

use crate::diagnostic::error::Result;
use crate::model::Onlyfile;

/// Parses `Onlyfile` text into a domain model.
///
/// Args:
/// content: Raw source text.
///
/// Returns:
/// Parsed `Onlyfile` document.
pub fn parse_onlyfile(content: &str) -> Result<Onlyfile> {
    grammar::parse(content)
}
