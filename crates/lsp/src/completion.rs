use only_syntax::{DirectiveKind, GuardKind};
use text_size::{TextRange, TextSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspCompletionKind {
    Directive,
    Guard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspCompletion {
    pub kind: LspCompletionKind,
    pub label: String,
    pub detail: String,
    pub insert_text: String,
    pub replace_range: TextRange,
}

pub fn completions(source: &str, offset: TextSize) -> Vec<LspCompletion> {
    let cursor = usize::min(usize::from(offset), source.len());
    let Some(prefix) = completion_prefix(source, cursor) else {
        return Vec::new();
    };

    match prefix.marker {
        '!' if prefix.is_line_start => directive_completions(&prefix),
        '@' if prefix.is_guard => guard_completions(&prefix),
        _ => Vec::new(),
    }
}

struct CompletionPrefix<'a> {
    marker: char,
    name: &'a str,
    replace_range: TextRange,
    is_line_start: bool,
    is_guard: bool,
}

fn completion_prefix(source: &str, cursor: usize) -> Option<CompletionPrefix<'_>> {
    if !source.is_char_boundary(cursor) {
        return None;
    }

    let line_start = source[..cursor].rfind('\n').map_or(0, |index| index + 1);
    let mut name_start = cursor;
    while name_start > line_start {
        let character = source[..name_start].chars().next_back()?;
        if !is_name_character(character) {
            break;
        }
        name_start -= character.len_utf8();
    }

    let marker = source[..name_start].chars().next_back()?;
    if !matches!(marker, '!' | '@') {
        return None;
    }
    let marker_start = name_start - marker.len_utf8();
    let before_marker = &source[line_start..marker_start];

    Some(CompletionPrefix {
        marker,
        name: &source[name_start..cursor],
        replace_range: TextRange::new(
            TextSize::from(marker_start as u32),
            TextSize::from(cursor as u32),
        ),
        is_line_start: before_marker.chars().all(char::is_whitespace),
        is_guard: before_marker.contains('?'),
    })
}

fn directive_completions(prefix: &CompletionPrefix<'_>) -> Vec<LspCompletion> {
    DirectiveKind::SUPPORTED
        .iter()
        .filter(|directive| directive.as_str().starts_with(prefix.name))
        .map(|directive| LspCompletion {
            kind: LspCompletionKind::Directive,
            label: format!("!{directive}"),
            detail: directive.description().unwrap_or_default().to_string(),
            insert_text: directive_snippet(directive).to_string(),
            replace_range: prefix.replace_range,
        })
        .collect()
}

fn guard_completions(prefix: &CompletionPrefix<'_>) -> Vec<LspCompletion> {
    GuardKind::SUPPORTED
        .iter()
        .filter(|guard| guard.as_str().starts_with(prefix.name))
        .map(|guard| LspCompletion {
            kind: LspCompletionKind::Guard,
            label: format!("@{guard}"),
            detail: guard.description().unwrap_or_default().to_string(),
            insert_text: guard_snippet(guard).to_string(),
            replace_range: prefix.replace_range,
        })
        .collect()
}

fn directive_snippet(directive: &DirectiveKind) -> &'static str {
    match directive {
        DirectiveKind::Version => "!version ${1:0.3}",
        DirectiveKind::Var => "!var ${1:name} = \"${2:value}\"",
        DirectiveKind::Shell => "!shell ${1|deno,bash,sh,pwsh,powershell|}",
        DirectiveKind::Unknown(_) => "",
    }
}

fn guard_snippet(guard: &GuardKind) -> &'static str {
    match guard {
        GuardKind::Os => "@os(\"${1:linux}\")",
        GuardKind::Arch => "@arch(\"${1:x86_64}\")",
        GuardKind::Env => "@env(\"${1:NAME}\")",
        GuardKind::Has => "@has(\"${1:command}\")",
        GuardKind::Unknown(_) => "",
    }
}

fn is_name_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
}
