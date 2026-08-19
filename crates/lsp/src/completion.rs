use only_semantic::compile_document;
use only_syntax::{DirectiveKind, GROUP_KEYWORD, GuardKind, MetadataKind};
use std::collections::BTreeSet;
use text_size::{TextRange, TextSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspCompletionKind {
    Directive,
    Guard,
    Metadata,
    Keyword,
    Task,
    Group,
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
    if let Some(dependencies) = dependency_completions(source, cursor) {
        return dependencies;
    }
    if let Some(keyword) = keyword_completion(source, cursor) {
        return keyword;
    }
    let Some(prefix) = completion_prefix(source, cursor) else {
        return Vec::new();
    };

    match prefix.marker {
        '!' if prefix.is_line_start => directive_completions(&prefix),
        '@' if prefix.is_guard => guard_completions(&prefix),
        '[' if prefix.is_line_start => metadata_completions(&prefix),
        _ => Vec::new(),
    }
}

fn dependency_completions(source: &str, cursor: usize) -> Option<Vec<LspCompletion>> {
    if !source.is_char_boundary(cursor) {
        return None;
    }

    let line_start = source[..cursor].rfind('\n').map_or(0, |index| index + 1);
    let line = &source[line_start..cursor];
    let ampersand = line.rfind('&')?;
    let before = &line[..ampersand];
    if !dependency_line(source, line_start, before) {
        return None;
    }

    let target_start = line_start + ampersand + 1;
    let target = source[target_start..cursor].trim_start();
    let start = target_start + (source[target_start..cursor].len() - target.len());
    let needs_space = start == target_start;
    let document = compile_document(source).document;
    let current_task = before
        .split('&')
        .next()
        .map(str::trim)
        .and_then(|header| header.split('(').next())
        .filter(|name| is_name_prefix(name));

    match target.split_once('.') {
        Some((group, task)) if is_name_prefix(group) && is_name_prefix(task) => {
            let tasks = document
                .tasks
                .iter()
                .filter(|candidate| candidate.namespace.as_deref() == Some(group))
                .filter(|candidate| !candidate.range.contains(TextSize::from(cursor as u32)))
                .filter(|candidate| Some(candidate.name.as_str()) != current_task)
                .filter(|candidate| candidate.name.starts_with(task))
                .map(|candidate| candidate.name.to_string())
                .collect::<BTreeSet<_>>();
            let task_start = start + group.len() + 1;
            Some(
                tasks
                    .into_iter()
                    .map(|name| LspCompletion {
                        kind: LspCompletionKind::Task,
                        label: name.clone(),
                        detail: format!("Task in group {group}."),
                        insert_text: name,
                        replace_range: text_range(task_start, cursor),
                    })
                    .collect(),
            )
        }
        None if is_name_prefix(target) => {
            let tasks = document
                .tasks
                .iter()
                .filter(|candidate| candidate.namespace.is_none())
                .filter(|candidate| !candidate.range.contains(TextSize::from(cursor as u32)))
                .filter(|candidate| Some(candidate.name.as_str()) != current_task)
                .filter(|candidate| candidate.name.starts_with(target))
                .map(|candidate| candidate.name.to_string())
                .collect::<BTreeSet<_>>();
            let groups = document
                .namespaces
                .iter()
                .filter(|candidate| candidate.name.starts_with(target))
                .map(|candidate| candidate.name.to_string())
                .collect::<BTreeSet<_>>();
            let replace_range = text_range(start, cursor);
            let tasks = tasks.into_iter().map(|name| LspCompletion {
                kind: LspCompletionKind::Task,
                label: name.clone(),
                detail: "Task dependency.".to_string(),
                insert_text: dependency_insert_text(name, needs_space),
                replace_range,
            });
            let groups = groups.into_iter().map(|name| LspCompletion {
                kind: LspCompletionKind::Group,
                label: name.clone(),
                detail: "Group dependency.".to_string(),
                insert_text: dependency_insert_text(name, needs_space),
                replace_range,
            });
            Some(tasks.chain(groups).collect())
        }
        _ => None,
    }
}

fn dependency_line(source: &str, line_start: usize, before: &str) -> bool {
    let trimmed = before.trim();
    if trimmed
        .split('&')
        .next()
        .is_some_and(|header| header.trim_end().ends_with(')'))
    {
        return true;
    }
    if !trimmed.is_empty() || !before.chars().any(char::is_whitespace) {
        return false;
    }

    let previous = source[..line_start].trim_end_matches(['\n', '\r']);
    let previous_start = previous.rfind('\n').map_or(0, |index| index + 1);
    previous[previous_start..].trim_end().ends_with(')')
}

fn dependency_insert_text(name: String, needs_space: bool) -> String {
    if needs_space {
        format!(" {name}")
    } else {
        name
    }
}

fn is_name_prefix(value: &str) -> bool {
    value.chars().all(is_name_character)
}

fn text_range(start: usize, end: usize) -> TextRange {
    TextRange::new(TextSize::from(start as u32), TextSize::from(end as u32))
}

fn keyword_completion(source: &str, cursor: usize) -> Option<Vec<LspCompletion>> {
    if !source.is_char_boundary(cursor) {
        return None;
    }

    let line_start = source[..cursor].rfind('\n').map_or(0, |index| index + 1);
    let line_prefix = &source[line_start..cursor];
    let trimmed = line_prefix.trim_start();
    if trimmed.is_empty() || trimmed.chars().any(char::is_whitespace) {
        return None;
    }

    let leading = line_prefix.len() - trimmed.len();
    if !trimmed.chars().all(is_name_character) {
        return None;
    }

    let name = trimmed;
    if !GROUP_KEYWORD.starts_with(name) {
        return Some(Vec::new());
    }

    let start = line_start + leading;
    Some(vec![LspCompletion {
        kind: LspCompletionKind::Keyword,
        label: GROUP_KEYWORD.to_string(),
        detail: "Define a task group.".to_string(),
        insert_text: "group ${1:name} {\n    ${0}\n}".to_string(),
        replace_range: TextRange::new(TextSize::from(start as u32), TextSize::from(cursor as u32)),
    }])
}

struct CompletionPrefix<'a> {
    marker: char,
    name: &'a str,
    replace_range: TextRange,
    has_closing_bracket: bool,
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
    if !matches!(marker, '!' | '@' | '[') {
        return None;
    }
    let marker_start = name_start - marker.len_utf8();
    let before_marker = &source[line_start..marker_start];
    let has_closing_bracket = marker == '[' && source[cursor..].starts_with(']');
    let replace_end = if has_closing_bracket {
        cursor + ']'.len_utf8()
    } else {
        cursor
    };

    Some(CompletionPrefix {
        marker,
        name: &source[name_start..cursor],
        replace_range: TextRange::new(
            TextSize::from(marker_start as u32),
            TextSize::from(replace_end as u32),
        ),
        has_closing_bracket,
        is_line_start: before_marker.chars().all(char::is_whitespace),
        is_guard: before_marker.contains('?'),
    })
}

fn metadata_completions(prefix: &CompletionPrefix<'_>) -> Vec<LspCompletion> {
    MetadataKind::SUPPORTED
        .iter()
        .filter(|field| field.as_str().starts_with(prefix.name))
        .map(|field| LspCompletion {
            kind: LspCompletionKind::Metadata,
            label: format!("[{field}]"),
            detail: field.description().unwrap_or_default().to_string(),
            insert_text: if prefix.has_closing_bracket && !prefix.name.is_empty() {
                format!("[{field}]")
            } else {
                format!("[{field}] ${{1:text}}")
            },
            replace_range: prefix.replace_range,
        })
        .collect()
}

fn directive_completions(prefix: &CompletionPrefix<'_>) -> Vec<LspCompletion> {
    DirectiveKind::SUPPORTED
        .iter()
        .filter(|directive| directive.as_str().starts_with(prefix.name))
        .map(|directive| LspCompletion {
            kind: LspCompletionKind::Directive,
            label: format!("!{directive}"),
            detail: directive.description().unwrap_or_default().to_string(),
            insert_text: directive_snippet(directive),
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

fn directive_snippet(directive: &DirectiveKind) -> String {
    match directive {
        DirectiveKind::Version => format!("!version ${{1:{}}}", major_minor_version()),
        DirectiveKind::Var => "!var ${1:name} = \"${2:value}\"".to_string(),
        DirectiveKind::Shell => "!shell ${1|deno,bash,sh,pwsh,powershell|}".to_string(),
        DirectiveKind::Unknown(_) => String::new(),
    }
}

fn major_minor_version() -> String {
    let mut components = env!("CARGO_PKG_VERSION").split('.');
    let major = components
        .next()
        .expect("package version has a major component");
    let minor = components
        .next()
        .expect("package version has a minor component");
    format!("{major}.{minor}")
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
