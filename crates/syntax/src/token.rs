use smol_str::SmolStr;
use text_size::TextRange;

use crate::SyntaxKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexToken {
    pub kind: SyntaxKind,
    pub text: SmolStr,
    pub range: TextRange,
}
