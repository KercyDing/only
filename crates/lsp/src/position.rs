use text_size::{TextRange, TextSize};
use tower_lsp::lsp_types::{Position, Range};

pub fn offset_to_position(source: &str, offset: TextSize) -> Position {
    let target = usize::min(offset.into(), source.len());
    let mut line = 0u32;
    let mut column = 0u32;

    for ch in source[..target].chars() {
        if ch == '\n' {
            line += 1;
            column = 0;
            continue;
        }

        column += ch.len_utf16() as u32;
    }

    Position::new(line, column)
}

pub fn range_to_lsp_range(source: &str, range: TextRange) -> Range {
    Range::new(
        offset_to_position(source, range.start()),
        offset_to_position(source, range.end()),
    )
}

pub fn position_to_offset(source: &str, position: Position) -> TextSize {
    let mut current_line = 0u32;
    let mut current_col = 0u32;
    let mut byte_offset = 0usize;

    for ch in source.chars() {
        if current_line == position.line && current_col >= position.character {
            break;
        }

        if ch == '\n' {
            if current_line == position.line {
                break;
            }

            current_line += 1;
            current_col = 0;
            byte_offset += ch.len_utf8();
            continue;
        }

        if current_line == position.line {
            let next_col = current_col + ch.len_utf16() as u32;
            if next_col > position.character {
                break;
            }
            current_col = next_col;
        }

        byte_offset += ch.len_utf8();
    }

    TextSize::from(byte_offset as u32)
}

#[cfg(test)]
mod tests {
    use super::{offset_to_position, position_to_offset, range_to_lsp_range};
    use text_size::{TextRange, TextSize};
    use tower_lsp::lsp_types::Position;

    #[test]
    fn converts_offsets_and_positions_with_utf16_columns() {
        let source = "task():\n    echo 🦀b\n";
        let crab_offset = TextSize::from(source.find('🦀').expect("emoji should exist") as u32);
        let b_offset = TextSize::from(source.find('b').expect("b should exist") as u32);

        assert_eq!(offset_to_position(source, crab_offset), Position::new(1, 9));
        assert_eq!(offset_to_position(source, b_offset), Position::new(1, 11));
        assert_eq!(position_to_offset(source, Position::new(1, 11)), b_offset);
    }

    #[test]
    fn converts_ranges() {
        let source = "fmt():\n    echo ok\n";
        let range = TextRange::new(TextSize::from(0), TextSize::from(5));
        let lsp = range_to_lsp_range(source, range);

        assert_eq!(lsp.start, Position::new(0, 0));
        assert_eq!(lsp.end, Position::new(0, 5));
    }
}
