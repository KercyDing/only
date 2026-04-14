use std::collections::HashMap;

use crate::{EngineError, PlanParam};

/// Renders one command string by replacing semantic interpolation placeholders.
///
/// Supports `\{\{` and `\}\}` escape sequences to produce literal `{{` and `}}`.
///
/// Args:
/// command: Raw command text from the execution plan.
/// params: Bound plan parameters available to interpolation.
///
/// Returns:
/// Rendered command text or an engine error when interpolation is invalid.
pub fn interpolate(command: &str, params: &[PlanParam]) -> Result<String, EngineError> {
    let parameter_map = params
        .iter()
        .filter_map(|param| {
            param
                .value
                .as_ref()
                .or(param.default_value.as_ref())
                .map(|value| (param.name.as_str(), value.as_str()))
        })
        .collect::<HashMap<_, _>>();
    let mut output = String::with_capacity(command.len());
    let mut rest = command;

    while let Some(start) = rest.find("{{") {
        push_literal(&mut output, &rest[..start]);

        if marker_is_escaped(rest, start) {
            output.pop();
            output.push_str("{{");
            rest = &rest[start + 2..];
            continue;
        }

        let placeholder = &rest[start + 2..];
        let Some(end) = placeholder.find("}}") else {
            return Err(EngineError::Interpolation(
                "unterminated interpolation expression".to_string(),
            ));
        };

        let name = placeholder[..end].trim();
        let Some(value) = parameter_map.get(name) else {
            return Err(EngineError::Interpolation(format!(
                "undefined variable '{{{{{name}}}}}' in command"
            )));
        };

        output.push_str(value);
        rest = &placeholder[end + 2..];
    }

    push_literal(&mut output, rest);
    Ok(output)
}

fn push_literal(output: &mut String, segment: &str) {
    let mut offset = 0usize;

    while let Some(rel) = segment[offset..].find("}}") {
        let close = offset + rel;
        output.push_str(&segment[offset..close]);
        if marker_is_escaped(segment, close) {
            output.pop();
        }
        output.push_str("}}");
        offset = close + 2;
    }

    output.push_str(&segment[offset..]);
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
