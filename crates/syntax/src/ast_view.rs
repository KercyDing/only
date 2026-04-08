use smol_str::SmolStr;
use text_size::TextRange;

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

    /// Returns normalized doc-comment text without the leading `%`.
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
            .strip_prefix('%')
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

    /// Returns the raw parameter section inside `(...)`.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Parameter section text without surrounding parentheses.
    pub fn params_text(&self) -> Option<SmolStr> {
        self.header_sections().params
    }

    /// Returns the raw guard expression without the leading `?`.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Guard section text when present.
    pub fn guard_text(&self) -> Option<SmolStr> {
        self.header_sections().guard
    }

    /// Returns the raw dependency section after `&`.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Dependency section text when present.
    pub fn dependencies_text(&self) -> Option<SmolStr> {
        self.header_sections().dependencies
    }

    /// Returns the explicit shell name when present.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Shell name from `shell=` or `shell?=`.
    pub fn shell_name(&self) -> Option<SmolStr> {
        self.header_sections().shell
    }

    /// Returns whether the task uses `shell?=`.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// `true` when the shell is a fallback shell.
    pub fn shell_fallback(&self) -> bool {
        self.header_sections().shell_fallback
    }

    /// Iterates normalized command lines from the task body.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Command lines in source order, without leading indentation.
    pub fn commands(&self) -> std::vec::IntoIter<SmolStr> {
        self.syntax
            .text()
            .to_string()
            .lines()
            .skip(1)
            .map(str::trim_start)
            .filter(|line| !line.is_empty())
            .map(SmolStr::new)
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn header_sections(&self) -> TaskHeaderSections {
        let Some(header) = self.header_text() else {
            return TaskHeaderSections::default();
        };
        let header = header.as_str();
        let Some(name) = self.name() else {
            return TaskHeaderSections::default();
        };
        let mut rest = &header[name.len()..];
        let mut sections = TaskHeaderSections::default();

        if rest.trim_start().starts_with('(') {
            let trimmed = rest.trim_start();
            let Some(close) = trimmed.find(')') else {
                return sections;
            };
            let params = trimmed[1..close].trim();
            if !params.is_empty() {
                sections.params = Some(SmolStr::new(params));
            }
            rest = &trimmed[close + 1..];
        }

        let trimmed = rest.trim_start();
        if let Some(after_question) = trimmed.strip_prefix('?') {
            let after_question = after_question.trim_start();
            let boundary = find_section_boundary(after_question);
            let guard = after_question[..boundary].trim();
            if !guard.is_empty() {
                sections.guard = Some(SmolStr::new(guard));
            }
            rest = &after_question[boundary..];
        }

        let trimmed = rest.trim_start();
        if let Some(after_amp) = trimmed.strip_prefix('&') {
            let boundary = find_shell_boundary(after_amp);
            let dependencies = after_amp[..boundary].trim();
            if !dependencies.is_empty() {
                sections.dependencies = Some(SmolStr::new(dependencies));
            }
            rest = &after_amp[boundary..];
        }

        let trimmed = rest.trim_start();
        if let Some(value) = trimmed.strip_prefix("shell?=") {
            let shell = value.split_whitespace().next().unwrap_or_default().trim();
            if !shell.is_empty() {
                sections.shell = Some(SmolStr::new(shell));
                sections.shell_fallback = true;
            }
        } else if let Some(value) = trimmed.strip_prefix("shell=") {
            let shell = value.split_whitespace().next().unwrap_or_default().trim();
            if !shell.is_empty() {
                sections.shell = Some(SmolStr::new(shell));
            }
        }

        sections
    }
}

#[derive(Debug, Default)]
struct TaskHeaderSections {
    params: Option<SmolStr>,
    guard: Option<SmolStr>,
    dependencies: Option<SmolStr>,
    shell: Option<SmolStr>,
    shell_fallback: bool,
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

fn find_section_boundary(input: &str) -> usize {
    input
        .find(" &")
        .or_else(|| input.find(" shell?="))
        .or_else(|| input.find(" shell="))
        .unwrap_or(input.len())
}

fn find_shell_boundary(input: &str) -> usize {
    input
        .find(" shell?=")
        .or_else(|| input.find(" shell="))
        .unwrap_or(input.len())
}
