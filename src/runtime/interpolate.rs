use std::collections::HashMap;

use crate::diagnostic::error::{OnlyError, Result};

/// Renders command text with `{{name}}` parameter interpolation.
///
/// Args:
/// command: Raw command text.
/// parameters: Bound task parameters.
///
/// Returns:
/// Rendered command string.
pub fn interpolate(command: &str, parameters: &[(String, String)]) -> Result<String> {
    let parameter_map = parameters.iter().cloned().collect::<HashMap<_, _>>();
    let mut output = String::with_capacity(command.len());
    let mut rest = command;

    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        let placeholder = &rest[start + 2..];
        let Some(end) = placeholder.find("}}") else {
            return Err(OnlyError::parse("unterminated interpolation expression"));
        };

        let name = placeholder[..end].trim();
        let Some(value) = parameter_map.get(name) else {
            return Err(OnlyError::parse(format!(
                "undefined variable '{{{{{name}}}}}' in command"
            )));
        };

        output.push_str(value);
        rest = &placeholder[end + 2..];
    }

    output.push_str(rest);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::interpolate;

    #[test]
    fn interpolates_defined_parameters() {
        let command = interpolate(
            "echo {{host}}:{{port}}",
            &[
                ("host".into(), "localhost".into()),
                ("port".into(), "3000".into()),
            ],
        )
        .expect("interpolation should succeed");

        assert_eq!(command, "echo localhost:3000");
    }

    #[test]
    fn rejects_undefined_parameters() {
        let error = interpolate("echo {{missing}}", &[]).expect_err("missing variable should fail");
        assert_eq!(
            error.to_string(),
            "undefined variable '{{missing}}' in command"
        );
    }
}
