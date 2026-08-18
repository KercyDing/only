use smol_str::SmolStr;
use text_size::TextRange;

use crate::InterpolationAst;

pub(crate) fn scan_interpolations(text: &str) -> Vec<InterpolationAst> {
    let mut out = Vec::new();
    let mut offset = 0usize;

    while let Some(start) = text[offset..].find("{{") {
        let open = offset + start;

        if marker_is_escaped(text, open) {
            offset = open + 2;
            continue;
        }

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

/// Returns the source ranges of interpolation names without their delimiters.
pub fn interpolation_name_ranges(text: &str) -> Vec<TextRange> {
    scan_interpolations(text)
        .into_iter()
        .filter_map(|interpolation| {
            let start = usize::from(interpolation.range.start()) + 2;
            let end = usize::from(interpolation.range.end()) - 2;
            let name = text.get(start..end)?.trim();
            let name_start = start + text.get(start..end)?.find(name)?;
            Some(TextRange::new(
                (name_start as u32).into(),
                ((name_start + name.len()) as u32).into(),
            ))
        })
        .collect()
}

fn marker_is_escaped(text: &str, marker_start: usize) -> bool {
    let mut slash_count = 0usize;
    let bytes = text.as_bytes();
    let mut index = marker_start;

    while index > 0 && bytes[index - 1] == b'\\' {
        slash_count += 1;
        index -= 1;
    }

    slash_count % 2 == 1
}
