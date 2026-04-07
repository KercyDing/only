use crate::diagnostic::error::{OnlyError, Result};
use crate::model::{
    CommandLine, Directive, Guard, Namespace, Onlyfile, Parameter, ProbeCall, ProbeKind,
    SourceSpan, TaskDefinition, TaskSignature,
};

/// Parses a growing MVP subset of the Onlyfile grammar.
///
/// Args:
/// content: Raw Onlyfile source text.
///
/// Returns:
/// Parsed Onlyfile document.
pub fn parse(content: &str) -> Result<Onlyfile> {
    let mut parser = Parser::new(content);
    parser.parse_document()
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    offsets: Vec<usize>,
    index: usize,
    pending_doc: Option<String>,
    in_namespace_section: bool,
}

impl<'a> Parser<'a> {
    fn new(content: &'a str) -> Self {
        let mut lines = Vec::new();
        let mut offsets = Vec::new();
        let mut offset = 0;

        for line in content.split_inclusive('\n') {
            lines.push(line);
            offsets.push(offset);
            offset += line.len();
        }

        if content.is_empty() {
            lines.push("");
            offsets.push(0);
        } else if !content.ends_with('\n') {
            let trailing = content.rsplit_once('\n').map_or(content, |(_, tail)| tail);
            lines.push(trailing);
            offsets.push(content.len() - trailing.len());
        }

        Self {
            lines,
            offsets,
            index: 0,
            pending_doc: None,
            in_namespace_section: false,
        }
    }

    fn parse_document(&mut self) -> Result<Onlyfile> {
        let mut document = Onlyfile::default();
        let mut current_namespace: Option<usize> = None;

        while let Some(line) = self.current_line() {
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') {
                self.index += 1;
                continue;
            }

            if !self.is_top_level(line) {
                return Err(self.error_current("unexpected indentation at top level"));
            }

            if trimmed.starts_with('!') {
                if self.in_namespace_section
                    || !document.global_tasks.is_empty()
                    || current_namespace.is_some()
                {
                    return Err(
                        self.error_current("directives must appear before tasks and namespaces")
                    );
                }

                document.directives.push(self.parse_directive(trimmed)?);
                self.index += 1;
                continue;
            }

            if let Some(doc) = trimmed.strip_prefix('%') {
                self.pending_doc = Some(doc.trim().to_owned());
                self.index += 1;
                continue;
            }

            if trimmed.starts_with('[') {
                let namespace = self.parse_namespace_header(trimmed)?;
                document.namespaces.push(namespace);
                current_namespace = Some(document.namespaces.len() - 1);
                self.in_namespace_section = true;
                self.index += 1;
                continue;
            }

            let task = self.parse_task()?;
            if let Some(namespace_index) = current_namespace {
                document.namespaces[namespace_index].tasks.push(task);
            } else {
                document.global_tasks.push(task);
            }
        }

        Ok(document)
    }

    fn parse_directive(&self, trimmed: &str) -> Result<Directive> {
        let span = self.span_for_line(self.index, trimmed);
        let Some(value) = trimmed.strip_prefix("!verbose ") else {
            return Err(
                self.error_current("unsupported directive; MVP only supports !verbose true|false")
            );
        };

        match value.trim() {
            "true" => Ok(Directive::Verbose { value: true, span }),
            "false" => Ok(Directive::Verbose { value: false, span }),
            _ => Err(self.error_current("invalid !verbose value; expected true or false")),
        }
    }

    fn parse_namespace_header(&self, trimmed: &str) -> Result<Namespace> {
        if !trimmed.ends_with(']') {
            return Err(self.error_current("unterminated namespace header"));
        }

        let name = &trimmed[1..trimmed.len() - 1];
        if name.is_empty() || !is_valid_namespace_name(name) {
            return Err(self.error_current("invalid namespace name"));
        }

        Ok(Namespace {
            name: name.to_owned(),
            span: self.span_for_line(self.index, trimmed),
            tasks: Vec::new(),
        })
    }

    fn parse_task(&mut self) -> Result<TaskDefinition> {
        let line = self
            .current_line()
            .expect("parser index should be in bounds");
        let trimmed = line.trim();
        let Some(signature_text) = trimmed.strip_suffix(':') else {
            return Err(self.error_current("task definition must end with ':'"));
        };

        let start_index = self.index;
        let signature = self.parse_signature(signature_text.trim(), start_index)?;
        let doc = self.pending_doc.take();
        self.index += 1;

        let mut commands = Vec::new();
        while let Some(next_line) = self.current_line() {
            let next_trimmed = next_line.trim();
            if next_trimmed.is_empty() {
                self.index += 1;
                continue;
            }

            if self.is_top_level(next_line) {
                break;
            }

            commands.push(CommandLine {
                text: next_line.trim_start().trim_end_matches('\n').to_owned(),
                span: self.span_for_line(self.index, next_trimmed),
            });
            self.index += 1;
        }

        let end_span = commands
            .last()
            .map_or(signature.span, |command| command.span);
        let task_span = SourceSpan::new(
            signature.span.offset,
            end_span.offset + end_span.length - signature.span.offset,
        );

        Ok(TaskDefinition {
            signature,
            doc,
            commands,
            span: task_span,
        })
    }

    fn parse_signature(&self, input: &str, line_index: usize) -> Result<TaskSignature> {
        let mut rest = input.trim();
        let name_end = rest.find(['(', '?', '&', ' ']).unwrap_or(rest.len());
        let name = &rest[..name_end];
        if name.is_empty() || !is_valid_identifier(name) {
            return Err(self.error_at(line_index, "invalid task name"));
        }

        rest = rest[name_end..].trim_start();
        let mut parameters = Vec::new();
        let mut guard = None;
        let mut dependencies = Vec::new();

        if rest.starts_with('(') {
            let (parameter_section, next_rest) = split_balanced(rest, '(', ')')
                .ok_or_else(|| self.error_at(line_index, "unterminated parameter list"))?;
            parameters = self.parse_parameters(parameter_section, line_index)?;
            rest = next_rest.trim_start();
        }

        if rest.starts_with('?') {
            let (parsed_guard, next_rest) = self.parse_guard(rest, line_index)?;
            guard = Some(parsed_guard);
            rest = next_rest.trim_start();
        }

        if rest.starts_with('&') {
            dependencies = self.parse_dependencies(rest, line_index)?;
            rest = "";
        }

        if !rest.is_empty() {
            return Err(self.error_at(
                line_index,
                format!("unexpected trailing signature content: {rest}"),
            ));
        }

        Ok(TaskSignature {
            name: name.to_owned(),
            parameters,
            guard,
            dependencies,
            span: self.span_for_line(line_index, input),
        })
    }

    fn parse_parameters(&self, input: &str, line_index: usize) -> Result<Vec<Parameter>> {
        let inner = &input[1..input.len() - 1];
        if inner.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut parameters = Vec::new();
        for raw_parameter in split_top_level(inner, ',') {
            let raw_parameter = raw_parameter.trim();
            if raw_parameter.is_empty() {
                return Err(self.error_at(line_index, "empty parameter in parameter list"));
            }

            let (name, default_value) = match raw_parameter.split_once('=') {
                Some((name, value)) => (
                    name.trim(),
                    Some(parse_string_literal(value.trim()).map_err(|message| {
                        self.error_at(line_index, format!("invalid default value: {message}"))
                    })?),
                ),
                None => (raw_parameter, None),
            };

            if !is_valid_identifier(name) {
                return Err(self.error_at(line_index, format!("invalid parameter name '{name}'")));
            }

            parameters.push(Parameter {
                name: name.to_owned(),
                default_value,
                span: self.span_for_line(line_index, raw_parameter),
            });
        }

        Ok(parameters)
    }

    fn parse_guard<'b>(&self, input: &'b str, line_index: usize) -> Result<(Guard, &'b str)> {
        let rest = input[1..].trim_start();
        let Some(rest) = rest.strip_prefix('@') else {
            return Err(self.error_at(line_index, "guard must start with @probe(...)"));
        };

        let probe_name_end = rest
            .find('(')
            .ok_or_else(|| self.error_at(line_index, "guard probe must be followed by (...)"))?;
        let probe_name = rest[..probe_name_end].trim();
        let kind = match probe_name {
            "os" => ProbeKind::Os,
            "arch" => ProbeKind::Arch,
            "env" => ProbeKind::Env,
            "cmd" => ProbeKind::Cmd,
            _ => return Err(self.error_at(line_index, format!("unknown probe '{probe_name}'"))),
        };

        let probe_args = &rest[probe_name_end..];
        let (argument_section, next_rest) = split_balanced(probe_args, '(', ')')
            .ok_or_else(|| self.error_at(line_index, "unterminated guard probe"))?;
        let argument = parse_string_literal(&argument_section[1..argument_section.len() - 1])
            .map_err(|message| {
                self.error_at(line_index, format!("invalid guard argument: {message}"))
            })?;
        let span = self.span_for_line(line_index, input.trim());

        Ok((
            Guard {
                probe: ProbeCall {
                    kind,
                    argument,
                    span,
                },
                span,
            },
            next_rest,
        ))
    }

    fn parse_dependencies(&self, input: &str, line_index: usize) -> Result<Vec<String>> {
        let mut dependencies = Vec::new();

        for raw_dependency in input.split('&').skip(1) {
            let dependency = raw_dependency.trim();
            if dependency.is_empty() {
                return Err(self.error_at(line_index, "empty dependency reference"));
            }

            if !is_valid_dependency_name(dependency) {
                return Err(self.error_at(
                    line_index,
                    format!("invalid dependency reference '{dependency}'"),
                ));
            }

            dependencies.push(dependency.to_owned());
        }

        Ok(dependencies)
    }

    fn current_line(&self) -> Option<&'a str> {
        self.lines.get(self.index).copied()
    }

    fn is_top_level(&self, line: &str) -> bool {
        !line.starts_with(' ') && !line.starts_with('\t')
    }

    fn span_for_line(&self, index: usize, trimmed: &str) -> SourceSpan {
        let raw_line = self.lines[index];
        let line_without_newline = raw_line.trim_end_matches('\n');
        let leading = line_without_newline.len() - line_without_newline.trim_start().len();
        SourceSpan::new(self.offsets[index] + leading, trimmed.len())
    }

    fn error_current(&self, message: impl Into<String>) -> OnlyError {
        self.error_at(self.index, message)
    }

    fn error_at(&self, line_index: usize, message: impl Into<String>) -> OnlyError {
        OnlyError::parse(format!("line {}: {}", line_index + 1, message.into()))
    }
}

fn split_balanced(input: &str, open: char, close: char) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            continue;
        }

        if ch == open {
            depth += 1;
            continue;
        }

        if ch == close {
            depth -= 1;
            if depth == 0 {
                let end = index + ch.len_utf8();
                return Some((&input[..end], &input[end..]));
            }
        }
    }

    None
}

fn split_top_level(input: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            continue;
        }

        if ch == separator {
            parts.push(&input[start..index]);
            start = index + ch.len_utf8();
        }
    }

    parts.push(&input[start..]);
    parts
}

fn parse_string_literal(input: &str) -> std::result::Result<String, &'static str> {
    if !input.starts_with('"') || !input.ends_with('"') || input.len() < 2 {
        return Err("string literal must use double quotes");
    }

    let mut output = String::new();
    let mut chars = input[1..input.len() - 1].chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }

        let Some(escaped) = chars.next() else {
            return Err("unterminated escape sequence");
        };

        match escaped {
            '"' => output.push('"'),
            '\\' => output.push('\\'),
            'n' => output.push('\n'),
            't' => output.push('\t'),
            'r' => output.push('\r'),
            _ => return Err("unsupported escape sequence"),
        }
    }

    Ok(output)
}

fn is_valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(ch) if ch == '_' || ch.is_ascii_alphabetic() => {}
        _ => return false,
    }

    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_valid_namespace_name(value: &str) -> bool {
    value
        .split('.')
        .all(|segment| !segment.is_empty() && is_valid_identifier(segment))
}

fn is_valid_dependency_name(value: &str) -> bool {
    value
        .split('.')
        .all(|segment| !segment.is_empty() && is_valid_identifier(segment))
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::model::{Directive, ProbeKind};

    #[test]
    fn parses_empty_document() {
        let document = parse("").expect("empty document should parse");
        assert!(document.directives.is_empty());
        assert!(document.global_tasks.is_empty());
        assert!(document.namespaces.is_empty());
    }

    #[test]
    fn parses_directive_global_task_and_namespace_task() {
        let source = "!verbose true\n% clean outputs\nclean():\n    rm -rf dist\n\n[frontend]\n% build app\nbuild():\n    npm run build\n";
        let document = parse(source).expect("document should parse");

        assert_eq!(document.directives.len(), 1);
        assert!(matches!(
            document.directives[0],
            Directive::Verbose { value: true, .. }
        ));
        assert_eq!(document.global_tasks[0].signature.name, "clean");
        assert_eq!(document.namespaces[0].tasks[0].signature.name, "build");
    }

    #[test]
    fn parses_signature_with_parameters_guard_and_dependencies() {
        let source =
            "deploy(tag=\"v1\", env=\"prod\") ? @os(\"linux\") & build & smoke:\n    echo ok\n";
        let document = parse(source).expect("signature should parse");
        let signature = &document.global_tasks[0].signature;

        assert_eq!(signature.name, "deploy");
        assert_eq!(signature.parameters.len(), 2);
        assert_eq!(signature.parameters[0].name, "tag");
        assert_eq!(signature.parameters[0].default_value.as_deref(), Some("v1"));
        assert_eq!(signature.parameters[1].name, "env");
        assert_eq!(
            signature.parameters[1].default_value.as_deref(),
            Some("prod")
        );
        assert_eq!(
            signature.guard.as_ref().map(|guard| &guard.probe.kind),
            Some(&ProbeKind::Os)
        );
        assert_eq!(
            signature
                .guard
                .as_ref()
                .map(|guard| guard.probe.argument.as_str()),
            Some("linux")
        );
        assert_eq!(signature.dependencies, vec!["build", "smoke"]);
    }

    #[test]
    fn rejects_unknown_directive() {
        let error = parse("!shell bash\n").expect_err("unknown directive should fail");
        assert_eq!(
            error.to_string(),
            "line 1: unsupported directive; MVP only supports !verbose true|false"
        );
    }
}
