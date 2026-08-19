use std::collections::HashMap;

use only_semantic::InterpolationAst;

use crate::{EngineError, PlanParam};

/// Renders a command using interpolation ranges produced by semantic analysis.
pub(crate) fn interpolate_with_parts(
    command: &str,
    interpolations: &[InterpolationAst],
    params: &[PlanParam],
) -> Result<String, EngineError> {
    let parameter_map = parameter_map(params);
    let mut output = String::with_capacity(command.len());
    let mut cursor = 0usize;

    for interpolation in interpolations {
        let start = usize::from(interpolation.range.start());
        let end = usize::from(interpolation.range.end());
        if start < cursor
            || start >= end
            || end > command.len()
            || !command.is_char_boundary(start)
            || !command.is_char_boundary(end)
        {
            return Err(EngineError::Interpolation(
                "invalid interpolation range".to_string(),
            ));
        }

        push_literal(&mut output, &command[cursor..start]);
        let Some(value) = parameter_map.get(interpolation.name.as_str()) else {
            return Err(EngineError::Interpolation(format!(
                "variable '{{{{{}}}}}' is not defined",
                interpolation.name
            )));
        };
        output.push_str(value);
        cursor = end;
    }

    push_literal(&mut output, &command[cursor..]);
    Ok(output)
}

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
    let parameter_map = parameter_map(params);
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
                "missing `}}` in command".to_string(),
            ));
        };

        let name = placeholder[..end].trim();
        let Some(value) = parameter_map.get(name) else {
            return Err(EngineError::Interpolation(format!(
                "variable '{{{{{name}}}}}' is not defined"
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

    while offset < segment.len() {
        let Some((rel, marker)) = ["{{", "}}"]
            .iter()
            .filter_map(|marker| segment[offset..].find(marker).map(|index| (index, *marker)))
            .min_by_key(|(index, _)| *index)
        else {
            output.push_str(&segment[offset..]);
            break;
        };

        let marker_start = offset + rel;
        output.push_str(&segment[offset..marker_start]);
        if marker_is_escaped(segment, marker_start) {
            output.pop();
        }
        output.push_str(marker);
        offset = marker_start + marker.len();
    }
}

fn parameter_map(params: &[PlanParam]) -> HashMap<&str, &str> {
    params
        .iter()
        .filter_map(|param| {
            param
                .value
                .as_ref()
                .or(param.default_value.as_ref())
                .map(|value| (param.name.as_str(), value.as_str()))
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
