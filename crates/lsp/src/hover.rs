use text_size::{TextRange, TextSize};

use crate::DocumentSnapshot;

/// Host-facing hover category used by the LSP crate.
///
/// Args:
/// None.
///
/// Returns:
/// Stable hover categories detached from semantic internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspHoverKind {
    Dependency,
    Directive,
    DocComment,
    GuardProbe,
    Interpolation,
    Namespace,
    ShellOperator,
    Task,
}

/// Host-facing hover payload for editor protocol conversion.
///
/// Args:
/// None.
///
/// Returns:
/// Name, signature, docs and range for one hovered source item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspHover {
    pub kind: LspHoverKind,
    pub name: String,
    pub signature: String,
    pub docs: Option<String>,
    pub range: TextRange,
    pub container_name: Option<String>,
}

/// Resolves hover information from one in-memory document snapshot.
///
/// Args:
/// snapshot: In-memory document snapshot with semantic analysis.
/// offset: Source offset queried by the editor host.
///
/// Returns:
/// Host-facing hover payload when one source item matches the offset.
pub fn hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    directive_hover(snapshot, offset)
        .or_else(|| doc_comment_hover(snapshot, offset))
        .or_else(|| probe_hover(snapshot, offset))
        .or_else(|| shell_operator_hover(snapshot, offset))
        .or_else(|| interpolation_hover(snapshot, offset))
        .or_else(|| dependency_hover(snapshot, offset))
        .or_else(|| task_hover(snapshot, offset))
        .or_else(|| namespace_hover(snapshot, offset))
}

fn directive_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    for directive in snapshot.syntax.document().directives() {
        let range = directive.keyword_range()?;
        if !range.contains(offset) {
            continue;
        }

        let name = directive.name()?.to_string();
        let value = directive.value().map(|value| value.to_string());
        let docs = match name.as_str() {
            "echo" => {
                "Controls runtime output. `true` shows task output normally; `false` keeps task progress but suppresses successful command output and only replays stderr when a task fails.".to_string()
            }
            "preview" => {
                "Controls preview mode. `true` prints the selected task variant and commands before execution; `false` runs tasks without the preview output.".to_string()
            }
            "shell" => "Sets the default shell host used for task commands.".to_string(),
            _ => return None,
        };

        return Some(LspHover {
            kind: LspHoverKind::Directive,
            name: name.clone(),
            signature: format!("!{name}"),
            docs: Some(match value {
                Some(value) => format!("{docs}\n\nCurrent value: `{value}`"),
                None => docs,
            }),
            range,
            container_name: None,
        });
    }

    None
}

fn doc_comment_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    for doc_comment in snapshot.syntax.document().doc_comments() {
        if doc_comment.range().contains(offset) {
            let docs = doc_comment.text()?.to_string();
            return Some(LspHover {
                kind: LspHoverKind::DocComment,
                name: "documentation".to_string(),
                signature: String::new(),
                docs: Some(docs),
                range: doc_comment.range(),
                container_name: None,
            });
        }
    }

    None
}

fn probe_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    let tokens = &snapshot.syntax.tokens;

    for window in tokens.windows(2) {
        let at = &window[0];
        let ident = &window[1];
        if at.kind != only_syntax::SyntaxKind::At || ident.kind != only_syntax::SyntaxKind::Ident {
            continue;
        }

        let range = TextRange::new(at.range.start(), ident.range.end());
        if !range.contains(offset) {
            continue;
        }

        let name = ident.text.to_string();
        let docs = match name.as_str() {
            "os" => "Checks whether the current operating system matches the requested value.",
            "arch" => "Checks whether the current CPU architecture matches the requested value.",
            "env" => "Checks whether the named environment variable is present.",
            "has" => "Checks whether a command is available in the current environment.",
            _ => return None,
        };
        let argument = snapshot
            .semantic
            .document
            .tasks
            .iter()
            .find(|task| task.range.contains(at.range.start()))
            .and_then(|task| task.guard.as_ref())
            .filter(|guard| guard.kind.as_str() == name)
            .map(|guard| guard.argument.to_string());
        let signature = match &argument {
            Some(argument) => format!("@{name}(\"{argument}\")"),
            None => format!("@{name}(...)"),
        };
        let docs = match argument {
            Some(argument) => format!("{docs}\n\nCurrent argument: `{argument}`"),
            None => docs.to_string(),
        };

        return Some(LspHover {
            kind: LspHoverKind::GuardProbe,
            name: name.clone(),
            signature,
            docs: Some(docs),
            range,
            container_name: None,
        });
    }

    None
}

fn shell_operator_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    let tokens = &snapshot.syntax.tokens;

    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            only_syntax::SyntaxKind::ShellFallbackKw => {
                if token.range.contains(offset) {
                    return Some(LspHover {
                        kind: LspHoverKind::ShellOperator,
                        name: "shell?=".to_string(),
                        signature: "shell?=".to_string(),
                        docs: Some(
                            "Prefers a specific shell and falls back to the default host shell when unavailable."
                                .to_string(),
                        ),
                        range: token.range,
                        container_name: None,
                    });
                }
            }
            only_syntax::SyntaxKind::ShellKw => {
                let eq = tokens.get(index + 1)?;
                if eq.kind != only_syntax::SyntaxKind::Eq {
                    continue;
                }
                let range = TextRange::new(token.range.start(), eq.range.end());
                if range.contains(offset) {
                    return Some(LspHover {
                        kind: LspHoverKind::ShellOperator,
                        name: "shell=".to_string(),
                        signature: "shell=".to_string(),
                        docs: Some(
                            "Requires a specific shell for the task without automatic fallback."
                                .to_string(),
                        ),
                        range,
                        container_name: None,
                    });
                }
            }
            _ => {}
        }
    }

    None
}

fn interpolation_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    let source = &snapshot.source;
    let target: usize = offset.into();
    let mut cursor = 0usize;

    while let Some(start_rel) = source[cursor..].find("{{") {
        let start = cursor + start_rel;
        let Some(end_rel) = source[start + 2..].find("}}") else {
            break;
        };
        let end = start + 2 + end_rel + 2;
        let range = TextRange::new((start as u32).into(), (end as u32).into());
        if range.contains(offset) {
            let name = source[start + 2..end - 2].trim().to_string();
            return Some(LspHover {
                kind: LspHoverKind::Interpolation,
                name: name.clone(),
                signature: format!("{{{{{name}}}}}"),
                docs: Some(
                    "Interpolates a task parameter into the command text at runtime.".to_string(),
                ),
                range,
                container_name: None,
            });
        }
        cursor = end;
        if cursor >= target && cursor >= source.len() {
            break;
        }
    }

    None
}

fn dependency_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    for (node, task) in snapshot
        .syntax
        .document()
        .tasks()
        .zip(snapshot.semantic.document.tasks.iter())
    {
        let header = node.header_info();
        for (index, reference) in header.dependency_refs.into_iter().enumerate() {
            if !reference.range.contains(offset) {
                continue;
            }

            let dependency = task.dependencies.get(index)?;
            let target = snapshot
                .semantic
                .document
                .tasks
                .iter()
                .find(|candidate| candidate.qualified_name() == dependency.name)?;

            return Some(LspHover {
                kind: LspHoverKind::Dependency,
                name: target.name.to_string(),
                signature: target.signature().to_string(),
                docs: target.doc.clone().map(|docs| docs.to_string()),
                range: reference.range,
                container_name: target.namespace.clone().map(|name| name.to_string()),
            });
        }
    }

    None
}

fn task_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    for (node, task) in snapshot
        .syntax
        .document()
        .tasks()
        .zip(snapshot.semantic.document.tasks.iter())
    {
        let range = node.name_range()?;
        if !range.contains(offset) {
            continue;
        }

        return Some(LspHover {
            kind: LspHoverKind::Task,
            name: task.name.to_string(),
            signature: task.name.to_string(),
            docs: task.doc.clone().map(|docs| docs.to_string()),
            range,
            container_name: task.namespace.clone().map(|name| name.to_string()),
        });
    }

    None
}

fn namespace_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    for namespace in &snapshot.semantic.document.namespaces {
        if namespace.range.contains(offset) {
            return Some(LspHover {
                kind: LspHoverKind::Namespace,
                name: namespace.name.to_string(),
                signature: format!("[{}]", namespace.name),
                docs: namespace.doc.clone().map(|docs| docs.to_string()),
                range: namespace.range,
                container_name: None,
            });
        }
    }

    None
}
