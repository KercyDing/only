use logos::Logos;
use smol_str::SmolStr;
use text_size::{TextRange, TextSize};

use crate::{LexToken, SyntaxKind};

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
enum RawTokenKind {
    #[token("shell?=")]
    ShellFallbackKw,
    #[token("shell")]
    ShellKw,
    #[token("!")]
    Bang,
    #[token("%")]
    Percent,
    #[token(":")]
    Colon,
    #[token("?")]
    Question,
    #[token("&")]
    Amp,
    #[token("=")]
    Eq,
    #[token("@")]
    At,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[regex(r#""([^"\n]|\\.)*""#)]
    String,
    #[regex(r"[A-Za-z_-][A-Za-z0-9_-]*")]
    Ident,
    #[regex(r"#[^\n]*", allow_greedy = true)]
    Comment,
    #[regex(r"[ ]+")]
    Spaces,
    #[token("\n")]
    Newline,
}

/// Lexes source text into tokens while preserving trivia.
///
/// Args:
/// source: Raw Onlyfile source text.
///
/// Returns:
/// Token stream including whitespace, comments and EOF.
pub fn lex(source: &str) -> Vec<LexToken> {
    let mut lexer = RawTokenKind::lexer(source);
    let mut tokens = Vec::new();
    let mut line_start = true;

    while let Some(result) = lexer.next() {
        let span = lexer.span();
        let text = &source[span.clone()];
        let start = TextSize::from(span.start as u32);
        let end = TextSize::from(span.end as u32);
        let kind = match result {
            Ok(RawTokenKind::ShellFallbackKw) => SyntaxKind::ShellFallbackKw,
            Ok(RawTokenKind::ShellKw) => SyntaxKind::ShellKw,
            Ok(RawTokenKind::Bang) => SyntaxKind::Bang,
            Ok(RawTokenKind::Percent) => SyntaxKind::Percent,
            Ok(RawTokenKind::Colon) => SyntaxKind::Colon,
            Ok(RawTokenKind::Question) => SyntaxKind::Question,
            Ok(RawTokenKind::Amp) => SyntaxKind::Amp,
            Ok(RawTokenKind::Eq) => SyntaxKind::Eq,
            Ok(RawTokenKind::At) => SyntaxKind::At,
            Ok(RawTokenKind::LParen) => SyntaxKind::LParen,
            Ok(RawTokenKind::RParen) => SyntaxKind::RParen,
            Ok(RawTokenKind::LBracket) => SyntaxKind::LBracket,
            Ok(RawTokenKind::RBracket) => SyntaxKind::RBracket,
            Ok(RawTokenKind::String) => SyntaxKind::String,
            Ok(RawTokenKind::Ident) => SyntaxKind::Ident,
            Ok(RawTokenKind::Comment) => SyntaxKind::Comment,
            Ok(RawTokenKind::Newline) => SyntaxKind::Newline,
            Ok(RawTokenKind::Spaces) if line_start => SyntaxKind::Indent,
            Ok(RawTokenKind::Spaces) => SyntaxKind::Whitespace,
            Err(_) => SyntaxKind::Unknown,
        };

        tokens.push(LexToken {
            kind,
            text: SmolStr::new(text),
            range: TextRange::new(start, end),
        });

        line_start = kind == SyntaxKind::Newline;
    }

    let eof = TextSize::from(source.len() as u32);
    tokens.push(LexToken {
        kind: SyntaxKind::Eof,
        text: SmolStr::new(""),
        range: TextRange::new(eof, eof),
    });

    tokens
}
