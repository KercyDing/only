use crate::diagnostic::error::{OnlyError, Result};
use crate::model::Onlyfile;

pub fn parse(content: &str) -> Result<Onlyfile> {
    if content.trim().is_empty() {
        return Ok(Onlyfile::default());
    }

    Err(OnlyError::parse(
        "parser is not implemented yet; only empty Onlyfile is currently accepted",
    ))
}
