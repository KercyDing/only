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
use crate::recover::{advance, consume_line, starts_top_level_item};
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

    loop {
        cursor.skip_trivia();

        let Some(token) = cursor.current() else {
            break;
        };
        if token.kind == SyntaxKind::Eof {
            break;
        }

        let mut input = cursor.remaining();
        let (item, consumed) = parse_top_level_item
            .with_taken()
            .parse_next(&mut input)
            .expect("top-level parser should always consume a non-EOF item");
        let token_slice = cursor.consume(consumed.len());

        match item {
            ParsedTopLevelItem::Directive { malformed } => {
                if malformed {
                    diagnostics.push(parse_error(
                        "parse.malformed-directive",
                        "malformed directive",
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
            ParsedTopLevelItem::Namespace { malformed } => {
                if malformed {
                    diagnostics.push(parse_error(
                        "parse.malformed-namespace-header",
                        "malformed namespace header",
                        token.range,
                    ));
                    builder.push_node(SyntaxKind::Error, token_slice);
                    continue;
                }
                builder.push_node(SyntaxKind::NamespaceBlock, token_slice);
            }
            ParsedTopLevelItem::Task {
                saw_colon,
                malformed,
            } => {
                if !saw_colon || malformed {
                    diagnostics.push(parse_error(
                        "parse.malformed-task-header",
                        "malformed task header",
                        token.range,
                    ));
                    builder.push_node(SyntaxKind::Error, token_slice);
                    continue;
                }
                builder.push_node(SyntaxKind::TaskDecl, token_slice);
            }
            ParsedTopLevelItem::Unexpected => {
                diagnostics.push(parse_error(
                    "parse.unexpected-token",
                    "unexpected top-level token",
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
    Directive { malformed: bool },
    DocComment,
    Namespace { malformed: bool },
    Task { saw_colon: bool, malformed: bool },
    Unexpected,
}

fn parse_top_level_item(input: &mut &[SyntaxKind]) -> ModalResult<ParsedTopLevelItem> {
    alt((
        parse_directive_item,
        parse_doc_comment_item,
        parse_namespace_item,
        parse_task_item,
        parse_unexpected_item,
    ))
    .parse_next(input)
}

fn parse_directive_item(input: &mut &[SyntaxKind]) -> ModalResult<ParsedTopLevelItem> {
    token_kind(input, SyntaxKind::Bang)?;
    let malformed = !line_has_non_trivia(input);
    consume_line(input);
    Ok(ParsedTopLevelItem::Directive { malformed })
}

fn parse_doc_comment_item(input: &mut &[SyntaxKind]) -> ModalResult<ParsedTopLevelItem> {
    token_kind(input, SyntaxKind::Percent)?;
    consume_line(input);
    Ok(ParsedTopLevelItem::DocComment)
}

fn parse_namespace_item(input: &mut &[SyntaxKind]) -> ModalResult<ParsedTopLevelItem> {
    token_kind(input, SyntaxKind::LBracket)?;
    let malformed = !line_contains_kind(input, SyntaxKind::RBracket);
    consume_line(input);
    Ok(ParsedTopLevelItem::Namespace { malformed })
}

fn parse_task_item(input: &mut &[SyntaxKind]) -> ModalResult<ParsedTopLevelItem> {
    token_kind(input, SyntaxKind::Ident)?;
    let mut saw_colon = false;
    let mut header_complete = false;
    let mut line_start = false;
    let mut malformed = false;
    let mut paren_depth = 0usize;
    let mut expect_guard_at = false;

    while let Some(kind) = input.first().copied() {
        if header_complete && line_start && starts_top_level_item(kind) {
            break;
        }

        if !header_complete {
            match kind {
                SyntaxKind::LParen => {
                    paren_depth += 1;
                }
                SyntaxKind::RParen => {
                    if paren_depth == 0 {
                        malformed = true;
                    } else {
                        paren_depth -= 1;
                    }
                }
                SyntaxKind::Question => {
                    expect_guard_at = true;
                }
                SyntaxKind::At => {
                    if expect_guard_at {
                        expect_guard_at = false;
                    }
                }
                SyntaxKind::Whitespace | SyntaxKind::Indent => {}
                _ => {
                    if expect_guard_at {
                        malformed = true;
                        expect_guard_at = false;
                    }
                }
            }
        }

        if kind == SyntaxKind::Colon {
            saw_colon = true;
        }
        advance(input);

        if kind == SyntaxKind::Eof {
            break;
        }

        if kind == SyntaxKind::Newline && !saw_colon {
            malformed |= paren_depth != 0 || expect_guard_at;
            break;
        }

        if kind == SyntaxKind::Newline && saw_colon {
            malformed |= paren_depth != 0 || expect_guard_at;
            header_complete = true;
        }

        line_start = kind == SyntaxKind::Newline;
    }

    Ok(ParsedTopLevelItem::Task {
        saw_colon,
        malformed,
    })
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
