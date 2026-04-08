use smol_str::SmolStr;
use text_size::TextRange;

use crate::InterpolationAst;

pub(crate) fn scan_interpolations(text: &str) -> Vec<InterpolationAst> {
    let mut out = Vec::new();
    let mut offset = 0usize;

    while let Some(start) = text[offset..].find("{{") {
        let open = offset + start;
        let Some(end_rel) = text[open + 2..].find("}}") else {
            break;
        };
        let close = open + 2 + end_rel + 2;
        let name = text[open + 2..close - 2].trim();
        out.push(InterpolationAst {
            name: SmolStr::new(name),
            range: TextRange::new((open as u32).into(), (close as u32).into()),
        });
        offset = close;
    }

    out
}
