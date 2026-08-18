use only_semantic::{GuardKind, MetadataKind, ShellKind, ShellOperator};
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
    ConditionOperator,
    Dependency,
    DependencyOperator,
    Directive,
    Metadata,
    GuardProbe,
    Interpolation,
    Namespace,
    Parameter,
    ParallelGroup,
    ShellOperator,
    Task,
    CommandBlock,
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
        .or_else(|| metadata_hover(snapshot, offset))
        .or_else(|| condition_operator_hover(snapshot, offset))
        .or_else(|| dependency_operator_hover(snapshot, offset))
        .or_else(|| parallel_group_hover(snapshot, offset))
        .or_else(|| probe_hover(snapshot, offset))
        .or_else(|| shell_operator_hover(snapshot, offset))
        .or_else(|| parameter_hover(snapshot, offset))
        .or_else(|| interpolation_hover(snapshot, offset))
        .or_else(|| command_block_hover(snapshot, offset))
        .or_else(|| dependency_hover(snapshot, offset))
        .or_else(|| task_hover(snapshot, offset))
        .or_else(|| namespace_hover(snapshot, offset))
}

fn parallel_group_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    for task in snapshot.syntax.document().tasks() {
        let Some(header) = task.header() else {
            continue;
        };
        for dependency in header.dependencies() {
            let delimiter_ranges = dependency.parallel_group_delimiter_ranges();
            let Some(range) = delimiter_ranges
                .into_iter()
                .find(|range| range.contains(offset))
            else {
                continue;
            };
            let signature = dependency.text().trim_start_matches('&').trim().to_string();
            return Some(LspHover {
                kind: LspHoverKind::ParallelGroup,
                name: "parallel dependencies".to_string(),
                signature,
                docs: Some("Runs these dependencies in parallel.".to_string()),
                range,
                container_name: None,
            });
        }
    }

    None
}

fn condition_operator_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    for task in snapshot.syntax.document().tasks() {
        let Some(header) = task.header() else {
            continue;
        };
        for condition in header.conditions() {
            let Some(range) = condition.operator_range() else {
                continue;
            };
            if range.contains(offset) {
                return Some(LspHover {
                    kind: LspHoverKind::ConditionOperator,
                    name: "condition".to_string(),
                    signature: "?".to_string(),
                    docs: Some("Runs the task only when the guard returns true.".to_string()),
                    range,
                    container_name: None,
                });
            }
        }
    }

    None
}

fn dependency_operator_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    for task in snapshot.syntax.document().tasks() {
        let Some(header) = task.header() else {
            continue;
        };
        for dependency in header.dependencies() {
            let Some(range) = dependency.operator_range() else {
                continue;
            };
            if range.contains(offset) {
                return Some(LspHover {
                    kind: LspHoverKind::DependencyOperator,
                    name: "dependency".to_string(),
                    signature: "&".to_string(),
                    docs: Some("Runs the dependency before this task.".to_string()),
                    range,
                    container_name: None,
                });
            }
        }
    }

    None
}

fn parameter_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    for (node, task) in snapshot
        .syntax
        .document()
        .tasks()
        .zip(snapshot.semantic.document.tasks.iter())
    {
        for (index, reference) in node.header_info().param_refs.into_iter().enumerate() {
            if !reference.range.contains(offset) {
                continue;
            }

            let parameter = task.params.get(index)?;
            let mut docs = if parameter.is_slice {
                "Variadic task parameter that captures remaining positional arguments.".to_string()
            } else {
                "Task parameter.".to_string()
            };
            if let Some(default) = &parameter.default_value {
                docs.push_str(&format!("\n\nDefault: `{default}`"));
            }

            return Some(LspHover {
                kind: LspHoverKind::Parameter,
                name: parameter.name.to_string(),
                signature: parameter_signature(parameter),
                docs: Some(docs),
                range: reference.range,
                container_name: Some(task.qualified_name().to_string()),
            });
        }
    }

    None
}

fn parameter_signature(parameter: &only_semantic::ParamAst) -> String {
    let suffix = if parameter.is_slice { ".." } else { "" };
    match &parameter.default_value {
        Some(default) => format!("{}{suffix}=\"{default}\"", parameter.name),
        None => format!("{}{suffix}", parameter.name),
    }
}

fn command_block_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    let default_shell = snapshot
        .semantic
        .document
        .directives
        .iter()
        .find_map(|directive| match directive {
            only_semantic::DirectiveAst::Shell { shell, .. } => Some(shell.clone()),
            only_semantic::DirectiveAst::Version { .. } => None,
            only_semantic::DirectiveAst::Variable { .. } => None,
        })
        .unwrap_or(ShellKind::Deno);

    for task in &snapshot.semantic.document.tasks {
        let shell = task
            .shell
            .as_ref()
            .map(|shell| &shell.selection.kind)
            .unwrap_or(&default_shell);
        for step in &task.steps {
            let only_semantic::TaskStepAst::CommandBlock(block) = step else {
                continue;
            };
            if !block.range.contains(offset) {
                continue;
            }
            return Some(LspHover {
                kind: LspHoverKind::CommandBlock,
                name: "command block".to_string(),
                signature: format!("block ({shell})"),
                docs: Some(format!("Runs this block once with `{shell}`.")),
                range: block.range,
                container_name: Some(task.qualified_name().to_string()),
            });
        }
    }

    None
}

fn directive_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    for directive in snapshot.syntax.document().directives() {
        let range = directive.keyword_range()?;
        if !range.contains(offset) {
            continue;
        }

        let kind = directive.directive_kind()?;
        let name = kind.as_str().to_string();
        let value = directive.value().map(|value| value.to_string());
        let docs = kind.description()?.to_string();

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

fn metadata_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    for metadata in snapshot.syntax.document().metadata() {
        if metadata.range().contains(offset) {
            let (field, value) = metadata.field()?;
            let kind = MetadataKind::parse(field.as_str());
            let mut docs = kind
                .description()
                .unwrap_or("Unknown metadata field.")
                .to_string();
            if !value.is_empty() {
                docs.push_str(&format!("\n\nCurrent text: {value}"));
            }
            return Some(LspHover {
                kind: LspHoverKind::Metadata,
                name: field.to_string(),
                signature: format!("[{field}]"),
                docs: Some(docs),
                range: metadata.range(),
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

        let kind = GuardKind::parse(&ident.text);
        let name = kind.as_str().to_string();
        let docs = kind.description()?;
        let argument = snapshot
            .semantic
            .document
            .tasks
            .iter()
            .find(|task| task.range.contains(at.range.start()))
            .and_then(|task| {
                task.guards
                    .iter()
                    .find(|guard| guard.range.contains(at.range.start()))
                    .or_else(|| task.guards.iter().find(|guard| guard.kind == kind))
            })
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
    for task in snapshot.syntax.document().tasks() {
        let Some(clause) = task.header().and_then(|header| header.shell()) else {
            continue;
        };
        let Some(range) = clause.content_range() else {
            continue;
        };
        if !range.contains(offset) {
            continue;
        }

        let shell = ShellKind::parse(&clause.shell_name()?);
        let operator = clause.operator()?;
        let signature = format!("{operator}{shell}");
        let docs = match operator {
            ShellOperator::Required => {
                format!("Uses {shell}. The task fails if it is unavailable.")
            }
            ShellOperator::Fallback => match shell.fallback() {
                Some(fallback) => {
                    format!("Prefers {shell} and falls back to {fallback} when unavailable.")
                }
                None => format!("{shell} has no fallback. Use `shell={shell}`."),
            },
        };

        return Some(LspHover {
            kind: LspHoverKind::ShellOperator,
            name: signature.clone(),
            signature,
            docs: Some(docs),
            range,
            container_name: None,
        });
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
            signature: task.signature().to_string(),
            docs: task.doc.clone().map(|docs| docs.to_string()),
            range,
            container_name: task.namespace.clone().map(|name| name.to_string()),
        });
    }

    None
}

fn namespace_hover(snapshot: &DocumentSnapshot, offset: TextSize) -> Option<LspHover> {
    for syntax in snapshot.syntax.document().namespaces() {
        if syntax.range().contains(offset) {
            let namespace = if syntax.is_close() {
                snapshot
                    .semantic
                    .document
                    .namespaces
                    .iter()
                    .find(|namespace| namespace.close_range == Some(syntax.range()))
            } else {
                let name = syntax.name()?;
                snapshot
                    .semantic
                    .document
                    .namespaces
                    .iter()
                    .find(|namespace| namespace.name == name)
            }?;
            let name = namespace.name.to_string();
            return Some(LspHover {
                kind: LspHoverKind::Namespace,
                name: name.clone(),
                signature: if syntax.is_close() {
                    "}".to_owned()
                } else if syntax.has_open_brace() {
                    if namespace.is_group {
                        format!("group {name} {{")
                    } else {
                        format!("namespace [{name}] {{")
                    }
                } else {
                    format!("namespace [{name}]")
                },
                docs: namespace.doc.clone().map(|docs| docs.to_string()),
                range: syntax.range(),
                container_name: None,
            });
        }
    }

    None
}
