use only_diagnostic::{Diagnostic, DiagnosticCode, DiagnosticPhase, DiagnosticSeverity};
use rowan::SyntaxNodeChildren;
use text_size::{TextRange, TextSize};
use winnow::Parser;
use winnow::combinator::alt;
use winnow::error::{ContextError, ErrMode, ModalResult};
use winnow::token::any;

use crate::ast_view::DocumentNode;
use crate::builder::ParseTreeBuilder;
use crate::cst::SyntaxNode;
use crate::cursor::TokenCursor;
use crate::recover::{
    advance, consume_line, starts_indented_namespace_boundary, starts_indented_namespace_member,
    starts_top_level_item,
};
use crate::trivia::{is_trivia, line_contains_kind, line_has_non_trivia};
use crate::{LexToken, SyntaxKind, lex};

#[derive(Debug, Clone)]
pub struct ParseResult {
    pub root: SyntaxNode,
    diagnostics: Vec<Diagnostic>,
}

impl ParseResult {
    /// Returns the typed document CST root.
    ///
    /// Args:
    /// None.
    ///
    /// Returns:
    /// Typed document wrapper for the parse root.
    pub fn document(&self) -> DocumentNode {
        DocumentNode::cast(self.root.clone()).expect("parse root must always be a document node")
    }
}

/// Extension helpers for parse results used by hosts and tests.
pub trait ParseResultExt {
    /// Returns root CST children for top-level inspection.
    fn root_children(&self) -> SyntaxNodeChildren<crate::cst::OnlyLanguage>;

    /// Returns collected parse diagnostics.
    fn diagnostics(&self) -> &[Diagnostic];
}

impl ParseResultExt for ParseResult {
    fn root_children(&self) -> SyntaxNodeChildren<crate::cst::OnlyLanguage> {
        self.root.children()
    }

    fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Parses Onlyfile text into a shallow CST with line-level recovery.
///
/// Args:
/// source: Raw Onlyfile source text.
///
/// Returns:
/// Parse result containing CST root and collected diagnostics.
pub fn parse(source: &str) -> ParseResult {
    let tokens = lex(source);
    parse_tokens(&tokens)
}

pub(crate) fn parse_tokens(tokens: &[LexToken]) -> ParseResult {
    let mut builder = ParseTreeBuilder::new();
    let mut diagnostics = Vec::new();
    let kinds = tokens.iter().map(|token| token.kind).collect::<Vec<_>>();
    let mut cursor = TokenCursor::new(tokens, &kinds);
    let mut in_braced_namespace = false;

    loop {
        let trivia = cursor.skip_trivia();
        builder.push_tokens(trivia);

        let Some(token) = cursor.current() else {
            break;
        };
        if token.kind == SyntaxKind::Eof {
            break;
        }

        let mut input = cursor.remaining();
        let (item, consumed) =
            (|input: &mut &[SyntaxKind]| parse_top_level_item(input, in_braced_namespace))
                .with_taken()
                .parse_next(&mut input)
                .expect("top-level parser should always consume a non-EOF item");
        let token_slice = cursor.consume(consumed.len());

        match item {
            ParsedTopLevelItem::Directive { malformed } => {
                if malformed {
                    diagnostics.push(parse_error(
                        "parse.malformed-directive",
                        "invalid directive",
                        token.range,
                    ));
                    builder.push_node(SyntaxKind::Error, token_slice);
                    continue;
                }
                builder.push_node(SyntaxKind::Directive, token_slice);
            }
            ParsedTopLevelItem::DocComment => {
                builder.push_node(SyntaxKind::DocComment, token_slice);
            }
            ParsedTopLevelItem::Namespace {
                malformed,
                is_close,
                has_open_brace,
            } => {
                if malformed {
                    diagnostics.push(parse_error(
                        "parse.malformed-namespace-header",
                        "invalid namespace",
                        token.range,
                    ));
                    builder.push_node(SyntaxKind::Error, token_slice);
                    continue;
                }
                builder.push_node(SyntaxKind::NamespaceBlock, token_slice);
                in_braced_namespace = has_open_brace && !is_close;
            }
            ParsedTopLevelItem::Task {
                saw_colon,
                malformed,
            } => {
                if !saw_colon || malformed {
                    diagnostics.push(parse_error(
                        "parse.malformed-task-header",
                        "invalid task header",
                        token.range,
                    ));
                    builder.push_node(SyntaxKind::Error, token_slice);
                    continue;
                }
                builder.push_task(token_slice);
            }
            ParsedTopLevelItem::Unexpected => {
                diagnostics.push(parse_error(
                    "parse.unexpected-token",
                    "unexpected text",
                    token.range,
                ));
                builder.push_node(SyntaxKind::Error, token_slice);
            }
        }
    }

    ParseResult {
        root: builder.finish(),
        diagnostics,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedTopLevelItem {
    Directive {
        malformed: bool,
    },
    DocComment,
    Namespace {
        malformed: bool,
        is_close: bool,
        has_open_brace: bool,
    },
    Task {
        saw_colon: bool,
        malformed: bool,
    },
    Unexpected,
}

fn parse_top_level_item(
    input: &mut &[SyntaxKind],
    in_braced_namespace: bool,
) -> ModalResult<ParsedTopLevelItem> {
    alt((
        parse_directive_item,
        parse_doc_comment_item,
        parse_namespace_item,
        |input: &mut &[SyntaxKind]| parse_task_item(input, in_braced_namespace),
        parse_unexpected_item,
    ))
    .parse_next(input)
}

fn parse_directive_item(input: &mut &[SyntaxKind]) -> ModalResult<ParsedTopLevelItem> {
    token_kind(input, SyntaxKind::Bang)?;
    let malformed = !line_has_non_trivia(input) || line_contains_kind(input, SyntaxKind::Comment);
    consume_line(input);
    Ok(ParsedTopLevelItem::Directive { malformed })
}

fn parse_doc_comment_item(input: &mut &[SyntaxKind]) -> ModalResult<ParsedTopLevelItem> {
    token_kind(input, SyntaxKind::Percent)?;
    consume_line(input);
    Ok(ParsedTopLevelItem::DocComment)
}

fn parse_namespace_item(input: &mut &[SyntaxKind]) -> ModalResult<ParsedTopLevelItem> {
    if input.first() == Some(&SyntaxKind::RBrace) {
        advance(input);
        let malformed =
            line_has_non_trivia(input) || line_contains_kind(input, SyntaxKind::Comment);
        consume_line(input);
        return Ok(ParsedTopLevelItem::Namespace {
            malformed,
            is_close: true,
            has_open_brace: false,
        });
    }

    token_kind(input, SyntaxKind::LBracket)?;
    let has_open_brace = line_contains_kind(input, SyntaxKind::LBrace);
    let malformed = namespace_open_is_malformed(input);
    consume_line(input);
    Ok(ParsedTopLevelItem::Namespace {
        malformed,
        is_close: false,
        has_open_brace,
    })
}

fn namespace_open_is_malformed(input: &[SyntaxKind]) -> bool {
    let line = input
        .iter()
        .copied()
        .take_while(|kind| !matches!(kind, SyntaxKind::Newline | SyntaxKind::Eof))
        .collect::<Vec<_>>();
    let mut index = 0;

    while line.get(index) == Some(&SyntaxKind::Whitespace) {
        index += 1;
    }
    if line.get(index) == Some(&SyntaxKind::Ident) {
        index += 1;
    }
    while line.get(index) == Some(&SyntaxKind::Whitespace) {
        index += 1;
    }
    if line.get(index) != Some(&SyntaxKind::RBracket) {
        return true;
    }
    index += 1;
    while line.get(index) == Some(&SyntaxKind::Whitespace) {
        index += 1;
    }
    if index == line.len() {
        return false;
    }
    if line.get(index) != Some(&SyntaxKind::LBrace) {
        return true;
    }
    index += 1;
    while line.get(index) == Some(&SyntaxKind::Whitespace) {
        index += 1;
    }
    index != line.len()
}

fn parse_task_item(
    input: &mut &[SyntaxKind],
    in_braced_namespace: bool,
) -> ModalResult<ParsedTopLevelItem> {
    token_kind(input, SyntaxKind::Ident)?;
    let mut saw_colon = false;
    let mut header_complete = false;
    let mut line_start = false;
    let mut malformed = false;
    let mut expect_guard_at = false;
    let mut phase = TaskHeaderPhase::BeforeTail;
    let mut saw_parameter_list = false;
    let mut continuation_header = false;
    let mut expect_clause_start = false;
    let mut expect_param_indent = false;

    while let Some(kind) = input.first().copied() {
        if header_complete
            && line_start
            && (starts_top_level_item(kind)
                || starts_indented_namespace_boundary(input)
                || (in_braced_namespace && starts_indented_namespace_member(input)))
        {
            break;
        }

        if saw_colon
            && !header_complete
            && !matches!(kind, SyntaxKind::Whitespace | SyntaxKind::Newline)
        {
            malformed = true;
        }

        if !header_complete {
            if expect_param_indent {
                match kind {
                    SyntaxKind::Indent => expect_param_indent = false,
                    SyntaxKind::RParen => expect_param_indent = false,
                    _ => {
                        malformed = true;
                        break;
                    }
                }
            }

            if kind == SyntaxKind::Comment {
                malformed = true;
            }

            if continuation_header && expect_clause_start {
                match kind {
                    // Header indentation is formatting, not syntax. The first meaningful token
                    // determines whether this line is a clause or the header terminator.
                    SyntaxKind::Indent | SyntaxKind::Whitespace => {}
                    SyntaxKind::Question
                    | SyntaxKind::Amp
                    | SyntaxKind::ShellKw
                    | SyntaxKind::ShellFallbackKw => {
                        expect_clause_start = false;
                    }
                    SyntaxKind::Colon => {
                        expect_clause_start = false;
                    }
                    SyntaxKind::Newline => malformed = true,
                    _ => {
                        malformed = true;
                        expect_clause_start = false;
                    }
                }
            }

            match &mut phase {
                TaskHeaderPhase::BeforeTail => match kind {
                    SyntaxKind::LParen => {
                        saw_parameter_list = true;
                        phase = TaskHeaderPhase::Params { depth: 1 };
                    }
                    SyntaxKind::Question => {
                        phase = TaskHeaderPhase::Guard { depth: 0 };
                        expect_guard_at = true;
                    }
                    SyntaxKind::Amp => {
                        phase = TaskHeaderPhase::Dependencies {
                            group_depth: 0,
                            saw_group: false,
                        };
                    }
                    SyntaxKind::Whitespace | SyntaxKind::Indent => {}
                    SyntaxKind::At if expect_guard_at => {
                        expect_guard_at = false;
                    }
                    _ => {
                        if expect_guard_at {
                            malformed = true;
                            expect_guard_at = false;
                        }
                    }
                },
                TaskHeaderPhase::Params { depth } => match kind {
                    SyntaxKind::LParen => *depth += 1,
                    SyntaxKind::RParen => {
                        if *depth == 0 {
                            malformed = true;
                        } else {
                            *depth -= 1;
                            if *depth == 0 {
                                phase = TaskHeaderPhase::BeforeTail;
                            }
                        }
                    }
                    _ => {}
                },
                TaskHeaderPhase::Guard { depth } => match kind {
                    SyntaxKind::LParen => *depth += 1,
                    SyntaxKind::RParen => {
                        if *depth > 0 {
                            *depth -= 1;
                        }
                        if *depth == 0 {
                            phase = TaskHeaderPhase::BeforeTail;
                        }
                    }
                    SyntaxKind::At if expect_guard_at => {
                        expect_guard_at = false;
                    }
                    SyntaxKind::Whitespace | SyntaxKind::Indent => {}
                    _ => {
                        if expect_guard_at {
                            malformed = true;
                            expect_guard_at = false;
                        }
                    }
                },
                TaskHeaderPhase::Dependencies {
                    group_depth,
                    saw_group,
                } => match kind {
                    SyntaxKind::LParen => {
                        if *group_depth > 0 {
                            malformed = true;
                        }
                        *group_depth += 1;
                        *saw_group = true;
                    }
                    SyntaxKind::RParen => {
                        if *group_depth == 0 {
                            malformed = true;
                        } else {
                            *group_depth -= 1;
                        }
                    }
                    SyntaxKind::Question | SyntaxKind::At => malformed = true,
                    SyntaxKind::ShellKw | SyntaxKind::ShellFallbackKw if *group_depth == 0 => {
                        phase = TaskHeaderPhase::Shell;
                    }
                    SyntaxKind::Unknown if kind == SyntaxKind::Unknown => {}
                    _ => {}
                },
                TaskHeaderPhase::Shell => {}
            }
        }

        if kind == SyntaxKind::Colon && phase.is_balanced() {
            saw_colon = true;
        }
        advance(input);

        if kind == SyntaxKind::Eof {
            break;
        }

        if kind == SyntaxKind::Newline && !saw_colon {
            if matches!(phase, TaskHeaderPhase::Params { depth } if depth > 0) {
                expect_param_indent = true;
                line_start = true;
                continue;
            }
            if saw_parameter_list && phase.is_balanced() && !expect_guard_at {
                continuation_header = true;
                expect_clause_start = true;
                line_start = true;
                continue;
            }
            malformed |= !phase.is_balanced() || expect_guard_at;
            break;
        }

        if kind == SyntaxKind::Newline && saw_colon {
            malformed |= !phase.is_balanced() || expect_guard_at;
            header_complete = true;
        }

        line_start = kind == SyntaxKind::Newline;
    }

    Ok(ParsedTopLevelItem::Task {
        saw_colon,
        malformed,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskHeaderPhase {
    BeforeTail,
    Params { depth: usize },
    Guard { depth: usize },
    Dependencies { group_depth: usize, saw_group: bool },
    Shell,
}

impl TaskHeaderPhase {
    fn is_balanced(self) -> bool {
        match self {
            TaskHeaderPhase::BeforeTail | TaskHeaderPhase::Shell => true,
            TaskHeaderPhase::Params { depth } | TaskHeaderPhase::Guard { depth } => depth == 0,
            TaskHeaderPhase::Dependencies { group_depth, .. } => group_depth == 0,
        }
    }
}

fn parse_unexpected_item(input: &mut &[SyntaxKind]) -> ModalResult<ParsedTopLevelItem> {
    any::<_, ErrMode<ContextError>>
        .verify(|kind: &SyntaxKind| !is_trivia(*kind) && *kind != SyntaxKind::Eof)
        .value(ParsedTopLevelItem::Unexpected)
        .parse_next(input)
}

fn token_kind(input: &mut &[SyntaxKind], kind: SyntaxKind) -> ModalResult<SyntaxKind> {
    any::<_, ErrMode<ContextError>>
        .verify(move |candidate: &SyntaxKind| *candidate == kind)
        .parse_next(input)
}

fn parse_error(code: &str, message: &str, range: TextRange) -> Diagnostic {
    Diagnostic::new(
        DiagnosticSeverity::Error,
        DiagnosticCode::new(code),
        message,
        DiagnosticPhase::Parse,
        normalize_range(range),
    )
}

fn normalize_range(range: TextRange) -> TextRange {
    if range.is_empty() {
        TextRange::new(range.start(), range.start() + TextSize::from(1))
    } else {
        range
    }
}
