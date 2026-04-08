use std::collections::HashMap;

use crate::{EngineError, PlanParam};

/// Renders one command string by replacing semantic interpolation placeholders.
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
        output.push_str(&rest[..start]);
        let placeholder = &rest[start + 2..];
        let Some(end) = placeholder.find("}}") else {
            return Err(EngineError::Runtime(
                "unterminated interpolation expression".to_string(),
            ));
        };

        let name = placeholder[..end].trim();
        let Some(value) = parameter_map.get(name) else {
            return Err(EngineError::Runtime(format!(
                "undefined variable '{{{{{name}}}}}' in command"
            )));
        };

        output.push_str(value);
        rest = &placeholder[end + 2..];
    }

    output.push_str(rest);
    Ok(output)
}
