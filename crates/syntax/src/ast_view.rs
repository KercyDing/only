use smol_str::SmolStr;
use text_size::{TextRange, TextSize};

use crate::{SyntaxKind, SyntaxNode};

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
pub struct DocCommentNode {
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
    pub stage: usize,
}

/// One parameter declaration parsed from a task header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskParamRef {
    pub name: SmolStr,
    pub range: TextRange,
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
    pub dependencies: Option<SmolStr>,
    pub shell: Option<SmolStr>,
    pub shell_fallback: bool,
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
    pub fn doc_comments(&self) -> impl Iterator<Item = DocCommentNode> + '_ {
        self.syntax.children().filter_map(DocCommentNode::cast)
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
}

impl DocCommentNode {
    /// Casts a raw rowan node into a typed doc-comment wrapper.
    ///
    /// Args:
    /// syntax: Raw rowan syntax node.
    ///
    /// Returns:
    /// Typed doc-comment wrapper when the kind matches `DocComment`.
    pub fn cast(syntax: SyntaxNode) -> Option<Self> {
        (syntax.kind() == SyntaxKind::DocComment).then_some(Self { syntax })
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

    /// Returns normalized doc-comment text without the leading `#`.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Trimmed doc-comment payload when present.
    pub fn text(&self) -> Option<SmolStr> {
        self.syntax
            .text()
            .to_string()
            .trim()
            .strip_prefix('#')
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(SmolStr::new)
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
            .text()
            .to_string()
            .trim()
            .strip_prefix('[')
            .and_then(|text| text.strip_suffix(']'))
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(SmolStr::new)
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
        self.syntax
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| token.kind() == SyntaxKind::Ident)
            .map(|token| token.text_range())
    }

    /// Returns the task name from the header identifier.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Task name when present.
    pub fn name(&self) -> Option<SmolStr> {
        self.syntax
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| token.kind() == SyntaxKind::Ident)
            .map(|token| SmolStr::new(token.text()))
    }

    /// Returns the normalized task header text without the trailing `:`.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Header text when present.
    pub fn header_text(&self) -> Option<SmolStr> {
        let mut header = String::new();

        for token in self
            .syntax
            .children_with_tokens()
            .filter_map(|element| element.into_token())
        {
            if token.kind() == SyntaxKind::Colon {
                break;
            }
            if token.kind() == SyntaxKind::Newline {
                break;
            }
            header.push_str(token.text());
        }

        let header = header.trim();
        (!header.is_empty()).then(|| SmolStr::new(header))
    }

    /// Returns the parsed task header sections and dependency references.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Structured header information parsed from one token stream pass.
    pub fn header_info(&self) -> TaskHeaderInfo {
        parse_task_header(&self.syntax)
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
    let body_start = first_line_end(&source).unwrap_or(source.len());
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderPhase {
    BeforeTail,
    Params { depth: usize },
    Guard { depth: usize },
    Dependencies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellExpectation {
    None,
    AllowEqOrName,
    NeedName,
}

#[derive(Debug, Default)]
struct PendingRef {
    name: String,
    start: Option<TextSize>,
    end: Option<TextSize>,
}

impl PendingRef {
    fn flush(&mut self, refs: &mut Vec<TaskDependencyRef>, stage: usize) {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            let name = self.name.trim();
            if !name.is_empty() {
                refs.push(TaskDependencyRef {
                    name: SmolStr::new(name),
                    range: TextRange::new(start, end),
                    stage,
                });
            }
        }
        self.name.clear();
        self.start = None;
        self.end = None;
    }

    fn extend(&mut self, token: &crate::cst::SyntaxToken) {
        self.start.get_or_insert(token.text_range().start());
        self.end = Some(token.text_range().end());
        self.name.push_str(token.text());
    }
}

fn parse_task_header(node: &SyntaxNode) -> TaskHeaderInfo {
    let mut info = TaskHeaderInfo::default();
    let mut phase = HeaderPhase::BeforeTail;
    let mut saw_name = false;
    let mut stage = 0usize;
    let mut group_depth = 0usize;
    let mut pending = PendingRef::default();
    let mut collector = String::new();
    let mut dependencies_started = false;
    let mut shell_expectation = ShellExpectation::None;
    let mut expect_param_name = false;

    for token in node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
    {
        let kind = token.kind();
        if matches!(
            kind,
            SyntaxKind::Colon | SyntaxKind::Newline | SyntaxKind::Eof
        ) {
            pending.flush(&mut info.dependency_refs, stage);
            flush_header_collector(&mut info, &phase, &collector, dependencies_started);
            break;
        }

        if !saw_name {
            if kind == SyntaxKind::Ident {
                saw_name = true;
            }
            continue;
        }

        if !matches!(shell_expectation, ShellExpectation::None) {
            match (shell_expectation, kind) {
                (_, SyntaxKind::Whitespace | SyntaxKind::Indent) => continue,
                (ShellExpectation::AllowEqOrName, SyntaxKind::Eq) => {
                    shell_expectation = ShellExpectation::NeedName;
                    continue;
                }
                (_, SyntaxKind::Ident) => {
                    info.shell = Some(SmolStr::new(token.text()));
                    shell_expectation = ShellExpectation::None;
                    continue;
                }
                _ => {
                    shell_expectation = ShellExpectation::None;
                }
            }
        }

        match &mut phase {
            HeaderPhase::BeforeTail => match kind {
                SyntaxKind::LParen => {
                    collector.clear();
                    expect_param_name = true;
                    phase = HeaderPhase::Params { depth: 1 };
                }
                SyntaxKind::Question => {
                    collector.clear();
                    phase = HeaderPhase::Guard { depth: 0 };
                }
                SyntaxKind::Amp => {
                    collector.clear();
                    dependencies_started = true;
                    phase = HeaderPhase::Dependencies;
                }
                SyntaxKind::ShellFallbackKw => {
                    info.shell_fallback = true;
                    shell_expectation = ShellExpectation::NeedName;
                }
                SyntaxKind::ShellKw => shell_expectation = ShellExpectation::AllowEqOrName,
                _ => {}
            },
            HeaderPhase::Params { depth } => {
                if *depth == 1 && expect_param_name {
                    match kind {
                        SyntaxKind::Whitespace | SyntaxKind::Indent => {}
                        SyntaxKind::Ident | SyntaxKind::ShellKw => {
                            info.param_refs.push(TaskParamRef {
                                name: SmolStr::new(token.text()),
                                range: token.text_range(),
                            });
                            expect_param_name = false;
                        }
                        _ => expect_param_name = false,
                    }
                }

                match kind {
                    SyntaxKind::LParen => {
                        *depth += 1;
                        collector.push_str(token.text());
                    }
                    SyntaxKind::RParen => {
                        *depth -= 1;
                        if *depth == 0 {
                            let trimmed = collector.trim();
                            if !trimmed.is_empty() {
                                info.params = Some(SmolStr::new(trimmed));
                            }
                            collector.clear();
                            expect_param_name = false;
                            phase = HeaderPhase::BeforeTail;
                        } else {
                            collector.push_str(token.text());
                        }
                    }
                    SyntaxKind::Unknown if *depth == 1 && token.text() == "," => {
                        collector.push_str(token.text());
                        expect_param_name = true;
                    }
                    _ => collector.push_str(token.text()),
                }
            }
            HeaderPhase::Guard { depth } => match kind {
                SyntaxKind::LParen => {
                    *depth += 1;
                    collector.push_str(token.text());
                }
                SyntaxKind::RParen => {
                    if *depth > 0 {
                        *depth -= 1;
                    }
                    collector.push_str(token.text());
                    if *depth == 0 {
                        let trimmed = collector.trim();
                        if !trimmed.is_empty() {
                            info.guard = Some(SmolStr::new(trimmed));
                        }
                        collector.clear();
                        phase = HeaderPhase::BeforeTail;
                    }
                }
                SyntaxKind::Amp => {
                    let trimmed = collector.trim();
                    if !trimmed.is_empty() {
                        info.guard = Some(SmolStr::new(trimmed));
                    }
                    collector.clear();
                    dependencies_started = true;
                    phase = HeaderPhase::Dependencies;
                }
                SyntaxKind::ShellFallbackKw => {
                    let trimmed = collector.trim();
                    if !trimmed.is_empty() {
                        info.guard = Some(SmolStr::new(trimmed));
                    }
                    collector.clear();
                    info.shell_fallback = true;
                    shell_expectation = ShellExpectation::NeedName;
                    phase = HeaderPhase::BeforeTail;
                }
                SyntaxKind::ShellKw => {
                    let trimmed = collector.trim();
                    if !trimmed.is_empty() {
                        info.guard = Some(SmolStr::new(trimmed));
                    }
                    collector.clear();
                    shell_expectation = ShellExpectation::AllowEqOrName;
                    phase = HeaderPhase::BeforeTail;
                }
                _ => collector.push_str(token.text()),
            },
            HeaderPhase::Dependencies => match kind {
                SyntaxKind::Amp if group_depth == 0 => {
                    pending.flush(&mut info.dependency_refs, stage);
                    if !info.dependency_refs.is_empty() {
                        stage += 1;
                    }
                    if !collector.trim().is_empty() {
                        if !info.dependencies.as_deref().unwrap_or_default().is_empty() {
                            collector.push(' ');
                        }
                        collector.push('&');
                    }
                }
                SyntaxKind::LParen => {
                    if group_depth > 0 {
                        pending.extend(&token);
                    }
                    group_depth += 1;
                    collector.push_str(token.text());
                }
                SyntaxKind::RParen => {
                    if group_depth > 1 {
                        pending.extend(&token);
                    } else {
                        pending.flush(&mut info.dependency_refs, stage);
                    }
                    group_depth = group_depth.saturating_sub(1);
                    collector.push_str(token.text());
                }
                SyntaxKind::ShellFallbackKw if group_depth == 0 => {
                    pending.flush(&mut info.dependency_refs, stage);
                    let trimmed = collector.trim();
                    if !trimmed.is_empty() {
                        info.dependencies = Some(SmolStr::new(trimmed));
                    }
                    collector.clear();
                    info.shell_fallback = true;
                    shell_expectation = ShellExpectation::NeedName;
                    phase = HeaderPhase::BeforeTail;
                }
                SyntaxKind::ShellKw if group_depth == 0 => {
                    pending.flush(&mut info.dependency_refs, stage);
                    let trimmed = collector.trim();
                    if !trimmed.is_empty() {
                        info.dependencies = Some(SmolStr::new(trimmed));
                    }
                    collector.clear();
                    shell_expectation = ShellExpectation::AllowEqOrName;
                    phase = HeaderPhase::BeforeTail;
                }
                SyntaxKind::Whitespace | SyntaxKind::Indent => {
                    collector.push_str(token.text());
                }
                SyntaxKind::Unknown if token.text() == "," && group_depth > 0 => {
                    pending.flush(&mut info.dependency_refs, stage);
                    collector.push_str(token.text());
                }
                _ => {
                    pending.extend(&token);
                    collector.push_str(token.text());
                }
            },
        }
    }

    if info.dependencies.is_none() {
        let trimmed = collector.trim();
        if dependencies_started && !trimmed.is_empty() {
            info.dependencies = Some(SmolStr::new(trimmed));
        }
    }

    info
}

fn flush_header_collector(
    info: &mut TaskHeaderInfo,
    phase: &HeaderPhase,
    collector: &str,
    dependencies_started: bool,
) {
    let trimmed = collector.trim();
    if trimmed.is_empty() {
        return;
    }

    match phase {
        HeaderPhase::Guard { .. } => info.guard = Some(SmolStr::new(trimmed)),
        HeaderPhase::Dependencies if dependencies_started => {
            info.dependencies = Some(SmolStr::new(trimmed))
        }
        _ => {}
    }
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
