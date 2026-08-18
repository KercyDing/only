use smol_str::SmolStr;
use text_size::{TextRange, TextSize};

use crate::{
    DirectiveKind, GuardKind, ShellKind, ShellOperator, ShellSelection, SyntaxKind, SyntaxNode,
    TaskShellRef,
};

/// Typed document CST wrapper.
///
/// Args:
/// None.
///
/// Returns:
/// Stable accessors for top-level syntax items and spans.
#[derive(Debug, Clone)]
pub struct DocumentNode {
    syntax: SyntaxNode,
}

/// Typed directive CST wrapper.
///
/// Args:
/// None.
///
/// Returns:
/// Stable accessors for directive name, value and span.
#[derive(Debug, Clone)]
pub struct DirectiveNode {
    syntax: SyntaxNode,
}

/// Typed doc-comment CST wrapper.
///
/// Args:
/// None.
///
/// Returns:
/// Stable accessors for doc-comment text and span.
#[derive(Debug, Clone)]
pub struct MetadataNode {
    syntax: SyntaxNode,
}

/// Typed namespace CST wrapper.
///
/// Args:
/// None.
///
/// Returns:
/// Stable accessors for namespace name and span.
#[derive(Debug, Clone)]
pub struct NamespaceNode {
    syntax: SyntaxNode,
}

/// Typed task CST wrapper.
///
/// Args:
/// None.
///
/// Returns:
/// Stable accessors for task header, commands and span.
#[derive(Debug, Clone)]
pub struct TaskNode {
    syntax: SyntaxNode,
}

#[derive(Debug, Clone)]
pub struct TaskHeaderNode {
    syntax: SyntaxNode,
}

#[derive(Debug, Clone)]
pub struct ParameterListNode {
    syntax: SyntaxNode,
}

#[derive(Debug, Clone)]
pub struct ParameterNode {
    syntax: SyntaxNode,
}

#[derive(Debug, Clone)]
pub struct ConditionClauseNode {
    syntax: SyntaxNode,
}

#[derive(Debug, Clone)]
pub struct DependencyClauseNode {
    syntax: SyntaxNode,
}

#[derive(Debug, Clone)]
pub struct ShellClauseNode {
    syntax: SyntaxNode,
}

#[derive(Debug, Clone)]
pub struct HeaderTerminatorNode {
    syntax: SyntaxNode,
}

/// One executable step read from a task body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStepNode {
    Command(TaskCommandNode),
    CommandBlock(TaskCommandBlockNode),
}

/// One ordinary command line and its source range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCommandNode {
    pub text: SmolStr,
    pub range: TextRange,
}

/// Consecutive block lines assembled into one shell input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCommandBlockNode {
    pub source: SmolStr,
    pub range: TextRange,
    pub line_ranges: Vec<TextRange>,
    pub marker_ranges: Vec<TextRange>,
}

/// One dependency reference parsed from a task header.
///
/// Args:
/// None.
///
/// Returns:
/// Dependency text and the precise source range of that reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDependencyRef {
    pub name: SmolStr,
    pub range: TextRange,
    pub arguments: Vec<TaskDependencyArgRef>,
    pub invocation_range: TextRange,
    pub stage: usize,
}

/// One positional string argument in a dependency invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDependencyArgRef {
    pub value: SmolStr,
    pub range: TextRange,
}

/// One parameter declaration parsed from a task header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskParamRef {
    pub name: SmolStr,
    pub range: TextRange,
    pub default_value: Option<SmolStr>,
    pub is_slice: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGuardRef {
    pub kind: GuardKind,
    pub argument: SmolStr,
    pub range: TextRange,
    pub name_range: TextRange,
}

/// Structured task header data parsed from the CST token stream.
///
/// Args:
/// None.
///
/// Returns:
/// Parsed task header sections and dependency references.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskHeaderInfo {
    pub params: Option<SmolStr>,
    pub param_refs: Vec<TaskParamRef>,
    pub guard: Option<SmolStr>,
    pub guards: Vec<TaskGuardRef>,
    pub dependencies: Option<SmolStr>,
    pub shell: Option<TaskShellRef>,
    pub dependency_refs: Vec<TaskDependencyRef>,
}

impl DocumentNode {
    /// Casts a raw rowan node into a typed document wrapper.
    ///
    /// Args:
    /// syntax: Raw rowan syntax node.
    ///
    /// Returns:
    /// Typed document wrapper when the kind matches `Document`.
    pub fn cast(syntax: SyntaxNode) -> Option<Self> {
        (syntax.kind() == SyntaxKind::Document).then_some(Self { syntax })
    }

    /// Returns the raw rowan node.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Borrowed raw syntax node.
    pub fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }

    /// Returns the document text range.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Full document range in source text coordinates.
    pub fn range(&self) -> TextRange {
        self.syntax.text_range()
    }

    /// Iterates directive children.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Typed directive iterator.
    pub fn directives(&self) -> impl Iterator<Item = DirectiveNode> + '_ {
        self.syntax.children().filter_map(DirectiveNode::cast)
    }

    /// Iterates doc-comment children.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Typed doc-comment iterator.
    pub fn metadata(&self) -> impl Iterator<Item = MetadataNode> + '_ {
        self.syntax.children().filter_map(MetadataNode::cast)
    }

    /// Iterates namespace children.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Typed namespace iterator.
    pub fn namespaces(&self) -> impl Iterator<Item = NamespaceNode> + '_ {
        self.syntax.children().filter_map(NamespaceNode::cast)
    }

    /// Iterates task children.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Typed task iterator.
    pub fn tasks(&self) -> impl Iterator<Item = TaskNode> + '_ {
        self.syntax.children().filter_map(TaskNode::cast)
    }
}

impl DirectiveNode {
    /// Casts a raw rowan node into a typed directive wrapper.
    ///
    /// Args:
    /// syntax: Raw rowan syntax node.
    ///
    /// Returns:
    /// Typed directive wrapper when the kind matches `Directive`.
    pub fn cast(syntax: SyntaxNode) -> Option<Self> {
        (syntax.kind() == SyntaxKind::Directive).then_some(Self { syntax })
    }

    /// Returns the directive text range.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Directive range in source text coordinates.
    pub fn range(&self) -> TextRange {
        self.syntax.text_range()
    }

    /// Returns the directive keyword range including the leading `!`.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Range covering a directive keyword such as `!shell` when present.
    pub fn keyword_range(&self) -> Option<TextRange> {
        let mut tokens = self
            .syntax
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| {
                !matches!(
                    token.kind(),
                    SyntaxKind::Whitespace | SyntaxKind::Indent | SyntaxKind::Newline
                )
            });
        let bang = tokens.find(|token| token.kind() == SyntaxKind::Bang)?;
        let keyword = tokens.next()?;
        Some(TextRange::new(
            bang.text_range().start(),
            keyword.text_range().end(),
        ))
    }

    /// Returns the directive name token text without the leading `!`.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Directive name when present.
    pub fn name(&self) -> Option<SmolStr> {
        non_trivia_token_texts(&self.syntax).nth(1)
    }

    /// Returns the typed directive kind.
    pub fn directive_kind(&self) -> Option<DirectiveKind> {
        self.name().map(|name| DirectiveKind::parse(&name))
    }

    /// Returns the directive value text after the directive name.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Joined directive value text when present.
    pub fn value(&self) -> Option<SmolStr> {
        let value = non_trivia_token_texts(&self.syntax)
            .skip(2)
            .collect::<Vec<_>>()
            .join(" ");
        (!value.is_empty()).then(|| SmolStr::new(value))
    }

    /// Returns the directive value with its original internal punctuation.
    pub fn raw_value(&self) -> Option<SmolStr> {
        let mut non_trivia = 0usize;
        let mut value = String::new();

        for token in self
            .syntax
            .children_with_tokens()
            .filter_map(|element| element.into_token())
        {
            if token.kind() == SyntaxKind::Newline {
                break;
            }
            if !matches!(
                token.kind(),
                SyntaxKind::Whitespace | SyntaxKind::Indent | SyntaxKind::Comment
            ) {
                non_trivia += 1;
            }
            if non_trivia >= 2 && !(non_trivia == 2 && token.kind() == SyntaxKind::Ident) {
                value.push_str(token.text());
            }
        }

        let value = value.trim();
        (!value.is_empty()).then(|| SmolStr::new(value))
    }

    /// Returns the first identifier range after the directive name.
    pub fn argument_name_range(&self) -> Option<TextRange> {
        self.syntax
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| matches!(token.kind(), SyntaxKind::Ident | SyntaxKind::ShellKw))
            .nth(1)
            .map(|token| token.text_range())
    }
}

impl MetadataNode {
    /// Casts a raw rowan node into a typed doc-comment wrapper.
    ///
    /// Args:
    /// syntax: Raw rowan syntax node.
    ///
    /// Returns:
    /// Typed wrapper for structured declaration metadata.
    pub fn cast(syntax: SyntaxNode) -> Option<Self> {
        (syntax.kind() == SyntaxKind::MetadataComment).then_some(Self { syntax })
    }

    /// Returns the doc-comment text range.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Doc-comment range in source text coordinates.
    pub fn range(&self) -> TextRange {
        self.syntax.text_range()
    }

    /// Returns the comment payload without its marker or field header.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Trimmed doc-comment payload when present.
    pub fn text(&self) -> Option<SmolStr> {
        let text = self.syntax.text().to_string();
        let text = text.trim();
        let text = if self.syntax.kind() == SyntaxKind::MetadataComment {
            let close = text.find(']')?;
            text.get(close + 1..)?.trim()
        } else {
            text.strip_prefix('#')?.trim()
        };
        (!text.is_empty()).then(|| SmolStr::new(text))
    }

    /// Returns a structured metadata field when the comment starts with `[name]`.
    pub fn field(&self) -> Option<(SmolStr, SmolStr)> {
        if self.syntax.kind() != SyntaxKind::MetadataComment {
            return None;
        }
        let text = self.syntax.text().to_string();
        let text = text.trim().strip_prefix('[')?;
        let close = text.find(']')?;
        let name = &text[..close];
        if name.is_empty()
            || !name.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
            })
        {
            return None;
        }

        Some((SmolStr::new(name), SmolStr::new(text[close + 1..].trim())))
    }

    /// Returns the source range of a structured metadata field name.
    pub fn field_range(&self) -> Option<TextRange> {
        if self.syntax.kind() != SyntaxKind::MetadataComment {
            return None;
        }
        let text = self.syntax.text().to_string();
        let text = text.trim().strip_prefix('[')?;
        let close = text.find(']')?;
        let name = &text[..close];
        if name.is_empty()
            || !name.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
            })
        {
            return None;
        }

        let start = self.syntax.text_range().start() + TextSize::from(1);
        Some(TextRange::new(
            start,
            start + TextSize::from(name.len() as u32),
        ))
    }

    /// Returns the source range of the complete metadata tag.
    pub fn tag_range(&self) -> Option<TextRange> {
        let field = self.field_range()?;
        let delimiter = TextSize::from(1);
        Some(TextRange::new(
            field.start() - delimiter,
            field.end() + delimiter,
        ))
    }
}

impl NamespaceNode {
    /// Casts a raw rowan node into a typed namespace wrapper.
    ///
    /// Args:
    /// syntax: Raw rowan syntax node.
    ///
    /// Returns:
    /// Typed namespace wrapper when the kind matches `NamespaceBlock`.
    pub fn cast(syntax: SyntaxNode) -> Option<Self> {
        (syntax.kind() == SyntaxKind::NamespaceBlock).then_some(Self { syntax })
    }

    /// Returns the namespace text range.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Namespace range in source text coordinates.
    pub fn range(&self) -> TextRange {
        self.syntax.text_range()
    }

    /// Returns the namespace name without brackets.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Namespace name when present.
    pub fn name(&self) -> Option<SmolStr> {
        self.syntax
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| token.kind() == SyntaxKind::Ident)
            .map(|token| SmolStr::new(token.text()))
    }

    /// Returns the namespace name range inside the brackets.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Namespace name range when present.
    pub fn name_range(&self) -> Option<TextRange> {
        self.syntax
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| token.kind() == SyntaxKind::Ident)
            .map(|token| token.text_range())
    }

    /// Returns whether this node uses the 0.4 `group name {` syntax.
    pub fn is_group(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .any(|token| token.kind() == SyntaxKind::GroupKw)
    }

    /// Returns whether this node closes the current namespace.
    pub fn is_close(&self) -> bool {
        self.syntax.text().to_string().trim() == "}"
    }

    /// Returns whether this namespace starts a braced scope.
    pub fn has_open_brace(&self) -> bool {
        self.syntax
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .any(|token| token.kind() == SyntaxKind::LBrace)
    }

    /// Returns whether this label is empty.
    pub fn is_empty(&self) -> bool {
        if self.is_close() {
            return false;
        }
        self.name().is_none()
    }
}

impl TaskNode {
    /// Casts a raw rowan node into a typed task wrapper.
    ///
    /// Args:
    /// syntax: Raw rowan syntax node.
    ///
    /// Returns:
    /// Typed task wrapper when the kind matches `TaskDecl`.
    pub fn cast(syntax: SyntaxNode) -> Option<Self> {
        (syntax.kind() == SyntaxKind::TaskDecl).then_some(Self { syntax })
    }

    /// Returns the task text range.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Task range in source text coordinates.
    pub fn range(&self) -> TextRange {
        self.syntax.text_range()
    }

    /// Returns the task name range from the header identifier.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Range covering the task name before the parameter list.
    pub fn name_range(&self) -> Option<TextRange> {
        self.header()?.name_range()
    }

    /// Returns the task name from the header identifier.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Task name when present.
    pub fn name(&self) -> Option<SmolStr> {
        self.header()?.name()
    }

    /// Returns the normalized task header text without the trailing `:`.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Header text when present.
    pub fn header_text(&self) -> Option<SmolStr> {
        let header = self.header()?.syntax.text().to_string();
        let header = header.trim().trim_end_matches(':').trim_end();
        (!header.is_empty()).then(|| SmolStr::new(header))
    }

    pub fn header(&self) -> Option<TaskHeaderNode> {
        self.syntax.children().find_map(TaskHeaderNode::cast)
    }

    pub fn uses_multiline_header(&self) -> bool {
        self.header()
            .is_some_and(|header| header.syntax.text().to_string().contains(['\n', '\r']))
    }

    /// Returns the parsed task header sections and dependency references.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Structured header information parsed from one token stream pass.
    pub fn header_info(&self) -> TaskHeaderInfo {
        self.header()
            .map_or_else(TaskHeaderInfo::default, |header| header.info())
    }

    /// Iterates normalized command lines from the task body.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Command lines in source order, without leading indentation.
    pub fn commands(&self) -> std::vec::IntoIter<SmolStr> {
        self.steps()
            .map(|step| match step {
                TaskStepNode::Command(command) => command.text,
                TaskStepNode::CommandBlock(block) => block.source,
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Iterates executable task steps with source ranges.
    pub fn steps(&self) -> std::vec::IntoIter<TaskStepNode> {
        task_body_steps(&self.syntax)
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[derive(Debug, Clone, Copy)]
struct BodyLine<'a> {
    text: &'a str,
    start: usize,
    end_with_newline: usize,
}

fn task_body_steps(node: &SyntaxNode) -> impl Iterator<Item = TaskStepNode> + '_ {
    let source = node.text().to_string();
    let body_start = node
        .children()
        .find(|child| child.kind() == SyntaxKind::TaskHeader)
        .map(|header| usize::from(header.text_range().end() - node.text_range().start()))
        .unwrap_or_else(|| first_line_end(&source).unwrap_or(source.len()));
    let base = usize::from(node.text_range().start());
    let lines = body_lines(&source, body_start).collect::<Vec<_>>();
    let mut steps = Vec::new();
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.text.trim_start_matches([' ', '\t']);
        if block_line_content(trimmed).is_none() {
            if !trimmed.is_empty() && !trimmed.starts_with("//") {
                let indent = line.text.len() - trimmed.len();
                steps.push(TaskStepNode::Command(TaskCommandNode {
                    text: SmolStr::new(trimmed),
                    range: text_range(
                        base + line.start + indent,
                        base + line.start + line.text.len(),
                    ),
                }));
            }
            index += 1;
            continue;
        }

        let block_start = line.start;
        let mut block_end = line.end_with_newline;
        let mut block_source = String::new();
        let mut line_ranges = Vec::new();
        let mut marker_ranges = Vec::new();

        while index < lines.len() {
            let block_line = lines[index];
            let trimmed = block_line.text.trim_start_matches([' ', '\t']);
            let Some(content) = block_line_content(trimmed) else {
                break;
            };
            let indent = block_line.text.len() - trimmed.len();
            let marker_start = base + block_line.start + indent;
            block_source.push_str(content);
            block_source.push('\n');
            line_ranges.push(text_range(
                base + block_line.start,
                base + block_line.start + block_line.text.len(),
            ));
            marker_ranges.push(text_range(marker_start, marker_start + 1));
            block_end = block_line.end_with_newline;
            index += 1;
        }

        steps.push(TaskStepNode::CommandBlock(TaskCommandBlockNode {
            source: SmolStr::new(block_source),
            range: text_range(base + block_start, base + block_end),
            line_ranges,
            marker_ranges,
        }));
    }

    steps.into_iter()
}

fn first_line_end(source: &str) -> Option<usize> {
    let (index, newline) = source
        .char_indices()
        .find(|(_, character)| matches!(character, '\n' | '\r'))?;
    let newline_len = if newline == '\r' && source.as_bytes().get(index + 1) == Some(&b'\n') {
        2
    } else {
        1
    };
    Some(index + newline_len)
}

fn body_lines(source: &str, start: usize) -> impl Iterator<Item = BodyLine<'_>> {
    let mut cursor = start;
    std::iter::from_fn(move || {
        if cursor >= source.len() {
            return None;
        }
        let line_start = cursor;
        let rest = &source[cursor..];
        let newline = rest
            .char_indices()
            .find(|(_, character)| matches!(character, '\n' | '\r'));
        let (line_end, newline_len) = match newline {
            Some((offset, '\r')) if rest.as_bytes().get(offset + 1) == Some(&b'\n') => {
                (cursor + offset, 2)
            }
            Some((offset, _)) => (cursor + offset, 1),
            None => (source.len(), 0),
        };
        cursor = line_end + newline_len;
        Some(BodyLine {
            text: &source[line_start..line_end],
            start: line_start,
            end_with_newline: cursor,
        })
    })
}

fn block_line_content(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('|')?;
    match rest.as_bytes().first() {
        None => Some(rest),
        Some(b' ' | b'\t') => Some(&rest[1..]),
        Some(_) => None,
    }
}

fn text_range(start: usize, end: usize) -> TextRange {
    TextRange::new(TextSize::from(start as u32), TextSize::from(end as u32))
}

impl TaskHeaderNode {
    pub fn cast(syntax: SyntaxNode) -> Option<Self> {
        (syntax.kind() == SyntaxKind::TaskHeader).then_some(Self { syntax })
    }

    pub fn range(&self) -> TextRange {
        self.syntax.text_range()
    }

    pub fn name(&self) -> Option<SmolStr> {
        self.name_node()?
            .first_token()
            .map(|token| SmolStr::new(token.text()))
    }

    pub fn name_range(&self) -> Option<TextRange> {
        self.name_node()?
            .first_token()
            .map(|token| token.text_range())
    }

    pub fn parameter_list(&self) -> Option<ParameterListNode> {
        self.syntax.children().find_map(ParameterListNode::cast)
    }

    pub fn conditions(&self) -> impl Iterator<Item = ConditionClauseNode> + '_ {
        self.syntax.children().filter_map(ConditionClauseNode::cast)
    }

    pub fn dependencies(&self) -> impl Iterator<Item = DependencyClauseNode> + '_ {
        self.syntax
            .children()
            .filter_map(DependencyClauseNode::cast)
    }

    pub fn shell(&self) -> Option<ShellClauseNode> {
        self.syntax.children().find_map(ShellClauseNode::cast)
    }

    pub fn terminator(&self) -> Option<HeaderTerminatorNode> {
        self.syntax.children().find_map(HeaderTerminatorNode::cast)
    }

    pub fn info(&self) -> TaskHeaderInfo {
        parse_task_header(self)
    }

    fn name_node(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|node| node.kind() == SyntaxKind::TaskName)
    }
}

impl ParameterListNode {
    pub fn cast(syntax: SyntaxNode) -> Option<Self> {
        (syntax.kind() == SyntaxKind::ParameterList).then_some(Self { syntax })
    }

    pub fn range(&self) -> TextRange {
        self.syntax.text_range()
    }

    pub fn parameters(&self) -> impl Iterator<Item = ParameterNode> + '_ {
        self.syntax.children().filter_map(ParameterNode::cast)
    }
}

impl ParameterNode {
    pub fn cast(syntax: SyntaxNode) -> Option<Self> {
        (syntax.kind() == SyntaxKind::Parameter).then_some(Self { syntax })
    }

    pub fn range(&self) -> TextRange {
        self.syntax.text_range()
    }

    pub fn name(&self) -> Option<SmolStr> {
        self.name_token().map(|token| SmolStr::new(token.text()))
    }

    pub fn name_range(&self) -> Option<TextRange> {
        self.name_token().map(|token| token.text_range())
    }

    pub fn default_value(&self) -> Option<SmolStr> {
        node_tokens(&self.syntax)
            .find(|token| token.kind() == SyntaxKind::String)
            .and_then(|token| {
                token
                    .text()
                    .strip_prefix('"')?
                    .strip_suffix('"')
                    .map(SmolStr::new)
            })
    }

    pub fn is_slice(&self) -> bool {
        self.syntax
            .text()
            .to_string()
            .split('=')
            .next()
            .is_some_and(|name| name.trim_end().ends_with(".."))
    }

    fn name_token(&self) -> Option<crate::cst::SyntaxToken> {
        node_tokens(&self.syntax)
            .find(|token| matches!(token.kind(), SyntaxKind::Ident | SyntaxKind::ShellKw))
    }
}

macro_rules! clause_node {
    ($type:ident, $kind:ident) => {
        impl $type {
            pub fn cast(syntax: SyntaxNode) -> Option<Self> {
                (syntax.kind() == SyntaxKind::$kind).then_some(Self { syntax })
            }

            pub fn range(&self) -> TextRange {
                self.syntax.text_range()
            }

            pub fn text(&self) -> SmolStr {
                SmolStr::new(self.syntax.text().to_string().trim())
            }
        }
    };
}

clause_node!(ConditionClauseNode, ConditionClause);
clause_node!(DependencyClauseNode, DependencyClause);
clause_node!(ShellClauseNode, ShellClause);
clause_node!(HeaderTerminatorNode, HeaderTerminator);

impl ConditionClauseNode {
    /// Returns the range of the leading conditional operator.
    pub fn operator_range(&self) -> Option<TextRange> {
        node_tokens(&self.syntax)
            .find(|token| token.kind() == SyntaxKind::Question)
            .map(|token| token.text_range())
    }
}

impl DependencyClauseNode {
    /// Returns the range of the leading dependency operator.
    pub fn operator_range(&self) -> Option<TextRange> {
        node_tokens(&self.syntax)
            .find(|token| token.kind() == SyntaxKind::Amp)
            .map(|token| token.text_range())
    }

    /// Returns the delimiter ranges for a parallel dependency group.
    pub fn parallel_group_delimiter_ranges(&self) -> Vec<TextRange> {
        let tokens = node_tokens(&self.syntax).collect::<Vec<_>>();
        if !tokens
            .iter()
            .any(|token| token.kind() == SyntaxKind::LParen)
            || !tokens
                .iter()
                .any(|token| token.kind() == SyntaxKind::RParen)
        {
            return Vec::new();
        }

        tokens
            .into_iter()
            .filter(|token| {
                matches!(
                    token.kind(),
                    SyntaxKind::LParen | SyntaxKind::Comma | SyntaxKind::RParen
                )
            })
            .map(|token| token.text_range())
            .collect()
    }
}

impl ShellClauseNode {
    /// Returns the shell selection operator.
    pub fn operator(&self) -> Option<ShellOperator> {
        node_tokens(&self.syntax).find_map(|token| match token.kind() {
            SyntaxKind::ShellKw => Some(ShellOperator::Required),
            SyntaxKind::ShellFallbackKw => Some(ShellOperator::Fallback),
            _ => None,
        })
    }

    /// Returns the selected shell name.
    pub fn shell_name(&self) -> Option<SmolStr> {
        node_tokens(&self.syntax)
            .find(|token| token.kind() == SyntaxKind::Ident)
            .map(|token| SmolStr::new(token.text()))
    }

    /// Returns the clause range without surrounding whitespace.
    pub fn content_range(&self) -> Option<TextRange> {
        let mut tokens = node_tokens(&self.syntax).filter(|token| {
            !matches!(
                token.kind(),
                SyntaxKind::Whitespace | SyntaxKind::Indent | SyntaxKind::Newline
            )
        });
        let first = tokens.next()?;
        let end = tokens.last().unwrap_or_else(|| first.clone());
        Some(TextRange::new(
            first.text_range().start(),
            end.text_range().end(),
        ))
    }
}

fn parse_task_header(node: &TaskHeaderNode) -> TaskHeaderInfo {
    let mut info = TaskHeaderInfo::default();

    if let Some(parameters) = node.parameter_list() {
        let refs = parameters
            .parameters()
            .filter_map(|parameter| {
                Some(TaskParamRef {
                    name: parameter.name()?,
                    range: parameter.name_range()?,
                    default_value: parameter.default_value(),
                    is_slice: parameter.is_slice(),
                })
            })
            .collect::<Vec<_>>();
        if !refs.is_empty() {
            info.params = Some(SmolStr::new(
                refs.iter()
                    .map(render_param_ref)
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }
        info.param_refs = refs;
    }

    info.guards = node.conditions().filter_map(parse_guard_ref).collect();
    info.guard = info
        .guards
        .first()
        .map(|guard| SmolStr::new(format!("@{}(\"{}\")", guard.kind, guard.argument)));

    let mut dependency_text = Vec::new();
    for (stage, clause) in node.dependencies().enumerate() {
        dependency_text.push(clause.text().trim_start_matches('&').trim().to_string());
        parse_dependency_clause(&clause.syntax, stage, &mut info.dependency_refs);
    }
    if !dependency_text.is_empty() {
        info.dependencies = Some(SmolStr::new(dependency_text.join(" & ")));
    }

    if let Some(shell) = node.shell() {
        let tokens = node_tokens(&shell.syntax)
            .filter(|token| {
                !matches!(
                    token.kind(),
                    SyntaxKind::Whitespace | SyntaxKind::Indent | SyntaxKind::Newline
                )
            })
            .collect::<Vec<_>>();
        let operator = tokens.first().and_then(|token| match token.kind() {
            SyntaxKind::ShellKw => Some(ShellOperator::Required),
            SyntaxKind::ShellFallbackKw => Some(ShellOperator::Fallback),
            _ => None,
        });
        let kind = tokens
            .iter()
            .rev()
            .find(|token| token.kind() == SyntaxKind::Ident)
            .map(|token| ShellKind::parse(token.text()));
        info.shell = operator.zip(kind).map(|(operator, kind)| TaskShellRef {
            selection: ShellSelection { kind, operator },
            range: shell.content_range().unwrap_or_else(|| shell.range()),
        });
    }

    info
}

fn render_param_ref(parameter: &TaskParamRef) -> String {
    let suffix = if parameter.is_slice { ".." } else { "" };
    match &parameter.default_value {
        Some(value) => format!("{}{suffix}=\"{value}\"", parameter.name),
        None => format!("{}{suffix}", parameter.name),
    }
}

fn parse_guard_ref(clause: ConditionClauseNode) -> Option<TaskGuardRef> {
    let tokens = node_tokens(&clause.syntax)
        .filter(|token| {
            !matches!(
                token.kind(),
                SyntaxKind::Whitespace | SyntaxKind::Indent | SyntaxKind::Newline
            )
        })
        .collect::<Vec<_>>();
    let name_token = tokens
        .iter()
        .find(|token| token.kind() == SyntaxKind::Ident)?;
    let name = name_token.text();
    let name_start = tokens
        .iter()
        .find(|token| token.kind() == SyntaxKind::At)
        .map_or_else(
            || name_token.text_range().start(),
            |token| token.text_range().start(),
        );
    let argument = tokens
        .iter()
        .find(|token| token.kind() == SyntaxKind::String)?
        .text()
        .strip_prefix('"')?
        .strip_suffix('"')?;

    Some(TaskGuardRef {
        kind: GuardKind::parse(name),
        argument: SmolStr::new(argument),
        range: clause.range(),
        name_range: TextRange::new(name_start, name_token.text_range().end()),
    })
}

fn parse_dependency_clause(node: &SyntaxNode, stage: usize, refs: &mut Vec<TaskDependencyRef>) {
    let mut tokens = node_tokens(node)
        .filter(|token| {
            !matches!(
                token.kind(),
                SyntaxKind::Whitespace | SyntaxKind::Indent | SyntaxKind::Newline
            )
        })
        .collect::<Vec<_>>();
    if tokens
        .first()
        .is_some_and(|token| token.kind() == SyntaxKind::Amp)
    {
        tokens.remove(0);
    }
    if tokens
        .first()
        .is_some_and(|token| token.kind() == SyntaxKind::LParen)
        && tokens
            .last()
            .is_some_and(|token| token.kind() == SyntaxKind::RParen)
    {
        tokens.remove(0);
        tokens.pop();
    }

    let mut invocation_start = 0usize;
    let mut depth = 0usize;
    for index in 0..=tokens.len() {
        let at_separator =
            index == tokens.len() || (tokens[index].kind() == SyntaxKind::Comma && depth == 0);
        if at_separator {
            if let Some(reference) =
                parse_dependency_invocation(&tokens[invocation_start..index], stage)
            {
                refs.push(reference);
            }
            invocation_start = index + 1;
            continue;
        }

        match tokens[index].kind() {
            SyntaxKind::LParen => depth += 1,
            SyntaxKind::RParen => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
}

fn parse_dependency_invocation(
    tokens: &[crate::cst::SyntaxToken],
    stage: usize,
) -> Option<TaskDependencyRef> {
    let first = tokens.first()?;
    let invocation_end = tokens.last()?.text_range().end();
    let argument_start = tokens
        .iter()
        .position(|token| token.kind() == SyntaxKind::LParen)
        .unwrap_or(tokens.len());
    let name_tokens = &tokens[..argument_start];
    let name_start = name_tokens.first()?.text_range().start();
    let name_end = name_tokens.last()?.text_range().end();
    let name = name_tokens
        .iter()
        .map(|token| token.text())
        .collect::<String>();
    let arguments = tokens
        .iter()
        .skip(argument_start.saturating_add(1))
        .filter(|token| token.kind() == SyntaxKind::String)
        .filter_map(|token| {
            let value = token.text().strip_prefix('"')?.strip_suffix('"')?;
            Some(TaskDependencyArgRef {
                value: SmolStr::new(value),
                range: token.text_range(),
            })
        })
        .collect();

    Some(TaskDependencyRef {
        name: SmolStr::new(name),
        range: TextRange::new(name_start, name_end),
        arguments,
        invocation_range: TextRange::new(first.text_range().start(), invocation_end),
        stage,
    })
}

fn node_tokens(node: &SyntaxNode) -> impl Iterator<Item = crate::cst::SyntaxToken> + '_ {
    node.descendants_with_tokens()
        .filter_map(|element| element.into_token())
}

fn non_trivia_token_texts(node: &SyntaxNode) -> impl Iterator<Item = SmolStr> + '_ {
    node.children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| {
            !matches!(
                token.kind(),
                SyntaxKind::Whitespace | SyntaxKind::Indent | SyntaxKind::Newline
            )
        })
        .map(|token| SmolStr::new(token.text()))
}
