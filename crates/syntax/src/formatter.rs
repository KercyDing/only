use rowan::NodeOrToken;
use text_size::TextRange;

use crate::{
    DirectiveKind, DirectiveNode, MetadataKind, MetadataNode, NamespaceNode, ParameterNode,
    SyntaxKind, SyntaxNode, TaskHeaderNode, TaskNode, snapshot,
};

const INDENT: &str = "    ";

/// Formats an Onlyfile using the built-in deterministic style.
pub fn format_source(source: &str) -> Result<String, String> {
    let parsed = snapshot(source);
    if let Some(diagnostic) = parsed
        .diagnostics()
        .iter()
        .find(|item| item.severity == only_diagnostic::DiagnosticSeverity::Error)
    {
        return Err(diagnostic.message.clone());
    }

    let cst_source = parsed.root().text().to_string();
    let mut formatter = DocumentFormatter::new(&cst_source);
    formatter.format(parsed.root())?;
    Ok(formatter.finish())
}

/// Formats the single top-level declaration touched by a source range.
pub fn format_range(source: &str, range: TextRange) -> Result<Option<(TextRange, String)>, String> {
    let parsed = snapshot(source);
    if let Some(diagnostic) = parsed
        .diagnostics()
        .iter()
        .find(|item| item.severity == only_diagnostic::DiagnosticSeverity::Error)
    {
        return Err(diagnostic.message.clone());
    }

    let cst_source = parsed.root().text().to_string();
    let mut matches = parsed
        .root()
        .children()
        .filter(|node| is_formattable_node(node.kind()))
        .filter(|node| ranges_touch(node.text_range(), range));
    let Some(node) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Ok(None);
    }

    let syntax_range = node.text_range();
    let node_range = include_leading_indent(&cst_source, syntax_range);
    let indent = is_inside_braced_namespace(parsed.root(), syntax_range.start());
    let (_, mut formatted) = format_top_level_node(node, &cst_source)?;
    if indent {
        formatted = indent_lines(&formatted);
    }
    formatted.push('\n');
    Ok(Some((node_range, formatted)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    Directive,
    Comment,
    Metadata,
    GroupOpen,
    NamespaceClose,
    Task,
}

struct DocumentFormatter<'a> {
    source: &'a str,
    output: String,
    previous: Option<ItemKind>,
    pending_newlines: usize,
    in_braced_namespace: bool,
    pending_metadata: Vec<(usize, String)>,
}

impl<'a> DocumentFormatter<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            output: String::new(),
            previous: None,
            pending_newlines: 0,
            in_braced_namespace: false,
            pending_metadata: Vec::new(),
        }
    }

    fn format(&mut self, root: &SyntaxNode) -> Result<(), String> {
        for element in root.children_with_tokens() {
            match element {
                NodeOrToken::Token(token) => match token.kind() {
                    SyntaxKind::Bom => self.output.push_str(token.text()),
                    SyntaxKind::Newline => {
                        self.flush_metadata();
                        self.pending_newlines += 1;
                    }
                    SyntaxKind::Comment => {
                        self.flush_metadata();
                        self.push_item(ItemKind::Comment, token.text().trim_end())
                    }
                    SyntaxKind::Whitespace | SyntaxKind::Indent => {}
                    SyntaxKind::Eof => self.flush_metadata(),
                    _ => self.push_raw(token.text()),
                },
                NodeOrToken::Node(node) if node.kind() == SyntaxKind::MetadataComment => {
                    self.queue_metadata(node)?;
                }
                NodeOrToken::Node(node) => {
                    self.flush_metadata();
                    self.format_node(node)?;
                }
            }
        }
        Ok(())
    }

    fn queue_metadata(&mut self, node: SyntaxNode) -> Result<(), String> {
        let comment = MetadataNode::cast(node.clone()).expect("metadata kind must cast");
        let (field, _) = comment
            .field()
            .ok_or_else(|| "invalid metadata field".to_owned())?;
        let order = match MetadataKind::parse(field.as_str()) {
            MetadataKind::Help => 0,
            MetadataKind::Desc => 1,
            MetadataKind::Pass => 2,
            MetadataKind::Fail => 3,
            MetadataKind::Unknown(_) => 4,
        };
        let (_, text) = format_top_level_node(node, self.source)?;
        self.pending_metadata.push((order, text));
        Ok(())
    }

    fn flush_metadata(&mut self) {
        self.pending_metadata.sort_by_key(|(order, _)| *order);
        let pending = std::mem::take(&mut self.pending_metadata);
        for (_, text) in pending {
            self.push_item(ItemKind::Metadata, &text);
        }
    }

    fn format_node(&mut self, node: SyntaxNode) -> Result<(), String> {
        let (kind, text) = format_top_level_node(node, self.source)?;
        self.push_item(kind, &text);
        Ok(())
    }

    fn push_item(&mut self, kind: ItemKind, text: &str) {
        if matches!(kind, ItemKind::GroupOpen | ItemKind::NamespaceClose) {
            self.in_braced_namespace = false;
        }
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }

        let line_breaks_after_comment = usize::from(self.previous == Some(ItemKind::Comment));
        let source_has_blank = self.pending_newlines > line_breaks_after_comment;
        let consecutive_directives =
            self.previous == Some(ItemKind::Directive) && kind == ItemKind::Directive;
        let namespace_boundary =
            self.previous == Some(ItemKind::GroupOpen) || kind == ItemKind::NamespaceClose;
        let metadata_boundary =
            self.previous == Some(ItemKind::Metadata) || kind == ItemKind::Metadata;
        if !self.output.is_empty()
            && ((source_has_blank
                && !consecutive_directives
                && !namespace_boundary
                && !metadata_boundary)
                || needs_structural_blank(self.previous, kind))
            && !self.output.ends_with("\n\n")
        {
            self.output.push('\n');
        }

        let text = text.trim_end_matches(['\n', '\r']);
        if self.in_braced_namespace {
            self.output.push_str(&indent_lines(text));
        } else {
            self.output.push_str(text);
        }
        self.output.push('\n');
        if kind == ItemKind::GroupOpen {
            self.in_braced_namespace = true;
        }
        self.previous = Some(kind);
        self.pending_newlines = 0;
    }

    fn push_raw(&mut self, text: &str) {
        self.output.push_str(text);
        self.pending_newlines = 0;
    }

    fn finish(mut self) -> String {
        while self.output.ends_with("\n\n") {
            self.output.pop();
        }
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.output
    }
}

fn format_top_level_node(node: SyntaxNode, source: &str) -> Result<(ItemKind, String), String> {
    match node.kind() {
        SyntaxKind::Directive => {
            let directive = DirectiveNode::cast(node).expect("directive kind must cast");
            Ok((ItemKind::Directive, format_directive(&directive, source)?))
        }
        SyntaxKind::MetadataComment => {
            let comment = MetadataNode::cast(node).expect("metadata kind must cast");
            let (field, value) = comment
                .field()
                .ok_or_else(|| "invalid metadata field".to_owned())?;
            let text = if value.is_empty() {
                format!("[{field}]")
            } else {
                format!("[{field}] {value}")
            };
            Ok((ItemKind::Metadata, text))
        }
        SyntaxKind::NamespaceBlock => {
            let namespace = NamespaceNode::cast(node).expect("namespace kind must cast");
            if namespace.is_close() {
                Ok((ItemKind::NamespaceClose, "}".to_owned()))
            } else {
                let name = namespace
                    .name()
                    .ok_or_else(|| "invalid namespace".to_owned())?;
                Ok((ItemKind::GroupOpen, format!("group {name} {{")))
            }
        }
        SyntaxKind::TaskDecl => {
            let task = TaskNode::cast(node).expect("task kind must cast");
            Ok((ItemKind::Task, format_task(&task, source)?))
        }
        SyntaxKind::Error => Err("cannot format invalid syntax".to_owned()),
        _ => Err("range does not contain a declaration".to_owned()),
    }
}

fn is_formattable_node(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Directive
            | SyntaxKind::MetadataComment
            | SyntaxKind::NamespaceBlock
            | SyntaxKind::TaskDecl
    )
}

fn ranges_touch(node: TextRange, requested: TextRange) -> bool {
    if requested.is_empty() {
        node.start() <= requested.start() && requested.start() < node.end()
    } else {
        node.start() < requested.end() && requested.start() < node.end()
    }
}

fn needs_structural_blank(previous: Option<ItemKind>, current: ItemKind) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    if current == ItemKind::NamespaceClose {
        return false;
    }
    if previous == ItemKind::GroupOpen {
        return false;
    }
    if previous == ItemKind::Metadata || previous == ItemKind::Comment {
        return false;
    }
    if current == ItemKind::Metadata || current == ItemKind::Comment {
        return !matches!(previous, ItemKind::Metadata | ItemKind::Comment);
    }
    if matches!(previous, ItemKind::NamespaceClose) || matches!(current, ItemKind::GroupOpen) {
        return true;
    }
    if previous == ItemKind::Directive && current == ItemKind::Directive {
        return false;
    }
    previous == ItemKind::Task
        || current == ItemKind::Task
        || previous == ItemKind::Directive
        || current == ItemKind::Directive
}

fn is_inside_braced_namespace(root: &SyntaxNode, offset: text_size::TextSize) -> bool {
    let mut inside = false;
    for node in root.children() {
        if node.text_range().start() >= offset {
            break;
        }
        let Some(namespace) = NamespaceNode::cast(node) else {
            continue;
        };
        if namespace.is_close() {
            inside = false;
        } else {
            inside = namespace.has_open_brace();
        }
    }
    inside
}

fn indent_lines(text: &str) -> String {
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{INDENT}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn include_leading_indent(source: &str, range: TextRange) -> TextRange {
    let start = usize::from(range.start());
    let line_start = source[..start].rfind('\n').map_or(0, |index| index + 1);
    if source[line_start..start]
        .chars()
        .all(|character| matches!(character, ' ' | '\t'))
    {
        TextRange::new((line_start as u32).into(), range.end())
    } else {
        range
    }
}

fn format_directive(directive: &DirectiveNode, source: &str) -> Result<String, String> {
    let name = directive
        .name()
        .ok_or_else(|| "invalid directive".to_owned())?;
    let raw_value = directive.raw_value().unwrap_or_default();
    if directive.directive_kind() == Some(DirectiveKind::Var) {
        let (variable, value) = raw_value
            .split_once('=')
            .ok_or_else(|| "invalid variable directive".to_owned())?;
        return Ok(format!("!var {} = {}", variable.trim(), value.trim()));
    }
    if raw_value.is_empty() {
        return Ok(format!("!{name}"));
    }

    // Slice CST-owned text to retain the original string spelling.
    let raw = source_range(source, directive.range());
    let value = raw
        .trim()
        .strip_prefix('!')
        .and_then(|text| text.strip_prefix(name.as_str()))
        .map(str::trim)
        .unwrap_or(raw_value.as_str());
    Ok(format!("!{name} {value}"))
}

fn format_task(task: &TaskNode, source: &str) -> Result<String, String> {
    let header = task
        .header()
        .ok_or_else(|| "task has no header".to_owned())?;
    let body = source_range(
        source,
        TextRange::new(header.range().end(), task.range().end()),
    );
    let body = format_task_body(body);
    let mut output = format_header(&header, source, !body.is_empty())?;
    if !body.is_empty() {
        output.push('\n');
        output.push_str(&body);
    }
    Ok(output)
}

fn format_header(header: &TaskHeaderNode, source: &str, has_body: bool) -> Result<String, String> {
    let name = header
        .name()
        .ok_or_else(|| "task header has no name".to_owned())?;
    let parameters = header
        .parameter_list()
        .map(|list| {
            list.parameters()
                .map(|parameter| format_parameter(&parameter, source))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let conditions = header
        .conditions()
        .map(|guard| format_guard(guard.text().as_str()))
        .collect::<Vec<_>>();
    let dependencies = header
        .dependencies()
        .map(|dependency| format_dependency(dependency.text().as_str()))
        .collect::<Vec<_>>();
    let shell = header
        .shell()
        .map(|shell| format_shell(shell.text().as_str()));
    let params_inline = parameters.join(", ");
    let prefix = format!("{name}({params_inline})");
    let mut clauses =
        Vec::with_capacity(conditions.len() + dependencies.len() + usize::from(shell.is_some()));
    clauses.extend(conditions);
    clauses.extend(dependencies);
    if let Some(shell) = shell {
        clauses.push(shell);
    }

    let mut inline = prefix.clone();
    for clause in &clauses {
        inline.push(' ');
        inline.push_str(clause);
    }
    if has_body {
        inline.push(':');
    }
    if clauses.len() < 3 {
        return Ok(inline);
    }

    let mut output = prefix;
    for clause in clauses {
        output.push('\n');
        output.push_str(INDENT);
        output.push_str(&clause);
    }
    if has_body {
        output.push_str("\n:");
    }
    Ok(output)
}

fn format_parameter(parameter: &ParameterNode, source: &str) -> String {
    let raw = source_range(source, parameter.range()).trim();
    let Some(equal) = find_unquoted(raw, '=') else {
        return collapse_whitespace(raw);
    };
    let name = collapse_whitespace(raw[..equal].trim());
    let value = raw[equal + 1..].trim();
    format!("{name} = {value}")
}

fn format_guard(raw: &str) -> String {
    let guard = raw.trim().trim_start_matches('?').trim();
    format!("? {}", normalize_delimiters(guard))
}

fn format_dependency(raw: &str) -> String {
    let dependency = raw.trim().trim_start_matches('&').trim();
    if let Some(group) = dependency
        .strip_prefix('(')
        .and_then(|text| text.strip_suffix(')'))
    {
        let members = split_top_level(group, ',')
            .into_iter()
            .map(normalize_delimiters)
            .filter(|member| !member.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        format!("& ({members})")
    } else {
        format!("& {}", normalize_delimiters(dependency))
    }
}

fn split_top_level(input: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;

    for (index, character) in input.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }

        match character {
            '"' => quoted = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            current if current == separator && depth == 0 => {
                parts.push(input[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(input[start..].trim());
    parts
}

fn format_shell(raw: &str) -> String {
    let compact = raw
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    if let Some(shell) = compact.strip_prefix("shell~=") {
        format!("shell~={shell}")
    } else if let Some(shell) = compact.strip_prefix("shell=") {
        format!("shell={shell}")
    } else {
        compact
    }
}

fn format_task_body(raw: &str) -> String {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = normalized.split('\n').peekable();
    if lines.peek().is_some_and(|line| line.is_empty()) {
        lines.next();
    }

    let mut output = Vec::new();
    let mut pending_blank = false;
    for line in lines {
        let body = line.trim_start_matches([' ', '\t']);
        if body.is_empty() {
            pending_blank = !output.is_empty();
            continue;
        }
        if pending_blank {
            output.push(String::new());
            pending_blank = false;
        }

        let formatted = if let Some(block) = body.strip_prefix('|') {
            let content = block.strip_prefix([' ', '\t']).unwrap_or(block);
            if content.is_empty() {
                format!("{INDENT}|")
            } else {
                format!("{INDENT}| {content}")
            }
        } else {
            format!("{INDENT}{body}")
        };
        output.push(formatted);
    }
    output.join("\n")
}

fn source_range(source: &str, range: TextRange) -> &str {
    &source[usize::from(range.start())..usize::from(range.end())]
}

fn find_unquoted(input: &str, needle: char) -> Option<usize> {
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in input.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
        } else if character == '"' {
            quoted = true;
        } else if character == needle {
            return Some(index);
        }
    }
    None
}

fn collapse_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_delimiters(input: &str) -> String {
    let mut output = String::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut pending_space = false;
    for character in input.chars() {
        if quoted {
            output.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        if character == '"' {
            if pending_space && !matches!(output.chars().last(), Some('(')) {
                output.push(' ');
            }
            pending_space = false;
            quoted = true;
            output.push(character);
        } else if character.is_whitespace() {
            pending_space = true;
        } else if matches!(character, '(' | ')') {
            while output.ends_with(' ') {
                output.pop();
            }
            output.push(character);
            pending_space = false;
        } else if character == ',' {
            while output.ends_with(' ') {
                output.pop();
            }
            output.push(',');
            pending_space = true;
        } else {
            if pending_space && !output.is_empty() && !output.ends_with('(') {
                output.push(' ');
            }
            pending_space = false;
            output.push(character);
        }
    }
    output.trim().to_owned()
}
