use std::collections::HashMap;

use only_diagnostic::{Diagnostic, DiagnosticCode, DiagnosticPhase, DiagnosticSeverity};
use only_syntax::{
    DirectiveKind, DirectiveNode, MetadataKind, MetadataNode, NamespaceNode, ShellKind, SyntaxKind,
    SyntaxSnapshot, TaskNode, parse_version_requirement,
};
use smol_str::SmolStr;
use text_size::TextRange;

use crate::interpolation::scan_interpolations;
use crate::names::resolve_dependency_names;
use crate::{
    CommandAst, CommandBlockAst, DependencyArgumentAst, DependencyAst, DirectiveAst, DocumentAst,
    GuardAst, NamespaceAst, ParamAst, ShellAst, TaskAst, TaskMetadataAst, TaskStepAst,
};

pub(crate) fn lower_syntax(snapshot: &SyntaxSnapshot) -> (DocumentAst, Vec<Diagnostic>) {
    let document = snapshot.document();
    let source = document.syntax().text().to_string();
    let mut directives = Vec::new();
    let mut namespaces = Vec::new();
    let mut tasks = Vec::new();
    let mut diagnostics = snapshot.diagnostics().to_vec();
    let mut current_namespace: Option<SmolStr> = None;
    let mut pending_docs = Vec::new();
    let mut saw_declaration = false;
    let mut saw_version = false;
    let mut left_directive_region = false;

    for node in document.syntax().children() {
        if let Some(directive) = DirectiveNode::cast(node.clone()) {
            pending_docs.clear();
            let directive_kind = directive.directive_kind();
            if directive_kind == Some(DirectiveKind::Var) && left_directive_region {
                diagnostics.push(semantic_error(
                    "variable.directive-placement",
                    "`!var` must be near the top of the file",
                    directive.range(),
                ));
            }
            if directive_kind == Some(DirectiveKind::Version) {
                if saw_version {
                    diagnostics.push(semantic_error(
                        "version.duplicate",
                        "`!version` is used more than once",
                        directive.range(),
                    ));
                } else if saw_declaration {
                    diagnostics.push(semantic_error(
                        "version.not-first-declaration",
                        "`!version` must come before all other declarations",
                        directive.range(),
                    ));
                }
                saw_version = true;
            }
            match lower_directive(&directive) {
                Ok(directive) => directives.push(directive),
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
            saw_declaration = true;
            continue;
        }

        if let Some(doc_comment) = MetadataNode::cast(node.clone()) {
            pending_docs.push(doc_comment);
            saw_declaration = true;
            continue;
        }

        if let Some(namespace) = NamespaceNode::cast(node.clone()) {
            left_directive_region = true;
            if namespace.is_close() {
                match current_namespace.as_deref() {
                    None => diagnostics.push(semantic_error(
                        "namespace.close-without-open",
                        "there is no group to close",
                        namespace.range(),
                    )),
                    Some(current) => {
                        if let Some(open) = namespaces
                            .iter_mut()
                            .rev()
                            .find(|open: &&mut NamespaceAst| open.name == current)
                        {
                            open.close_range = Some(namespace.range());
                        }
                        current_namespace = None;
                    }
                }
                pending_docs.clear();
            } else if namespace.is_empty() {
                diagnostics.push(semantic_error(
                    "namespace.empty-label",
                    "group name cannot be empty",
                    namespace.range(),
                ));
            } else {
                discard_detached_metadata(&mut pending_docs, &source, namespace.range().start());
                let docs = std::mem::take(&mut pending_docs);
                match lower_namespace(&namespace, docs) {
                    Ok(namespace) => {
                        current_namespace = Some(namespace.name.clone());
                        namespaces.push(namespace);
                    }
                    Err(diagnostic) => diagnostics.push(diagnostic),
                }
            }
            saw_declaration = true;
            continue;
        }

        if let Some(task) = TaskNode::cast(node.clone()) {
            left_directive_region = true;
            discard_detached_metadata(&mut pending_docs, &source, task.range().start());
            let docs = std::mem::take(&mut pending_docs);
            let empty_body_range = task
                .header()
                .and_then(|header| header.terminator().map(|terminator| terminator.range()))
                .filter(|_| task.steps().next().is_none());
            match lower_task(&task, current_namespace.clone(), docs) {
                Ok(task) => {
                    if let Some(range) = empty_body_range {
                        diagnostics.push(semantic_error(
                            "semantic.empty-task-body",
                            "task body is empty; add a command or remove ':'",
                            range,
                        ));
                    }
                    tasks.push(task);
                }
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
            saw_declaration = true;
            continue;
        }

        if node.kind() == SyntaxKind::Error {
            pending_docs.clear();
            saw_declaration = true;
        }
    }

    inherit_variant_metadata(&mut tasks);
    resolve_dependency_names(&mut tasks);

    (
        DocumentAst {
            directives,
            namespaces,
            tasks,
        },
        diagnostics,
    )
}

fn inherit_variant_metadata(tasks: &mut [TaskAst]) {
    let mut base_metadata = HashMap::new();

    for task in tasks {
        let base = base_metadata
            .entry(task.qualified_name())
            .or_insert_with(|| task.metadata.clone());
        task.metadata = inherit_and_override_metadata(base, &task.metadata);
        task.doc = task.metadata.help.clone();
    }
}

fn inherit_and_override_metadata(
    inherited: &TaskMetadataAst,
    overrides: &TaskMetadataAst,
) -> TaskMetadataAst {
    let mut metadata = overrides.clone();
    metadata.help = overrides.help.clone().or_else(|| inherited.help.clone());
    metadata.desc = overrides.desc.clone().or_else(|| inherited.desc.clone());
    metadata.pass = overrides.pass.clone().or_else(|| inherited.pass.clone());
    metadata.fail = overrides.fail.clone().or_else(|| inherited.fail.clone());
    metadata
}

fn discard_detached_metadata(
    pending: &mut Vec<MetadataNode>,
    source: &str,
    declaration_start: text_size::TextSize,
) {
    let Some(last) = pending.last() else {
        return;
    };
    let start = usize::from(last.range().end());
    let end = usize::from(declaration_start);
    let gap = source.get(start..end).unwrap_or_default();
    let detached = gap.contains('\n')
        || gap.contains('\r')
        || gap.lines().any(|line| {
            let line = line.trim_start();
            line.starts_with('#') || line.starts_with("//")
        });
    if detached {
        pending.clear();
    }
}

fn lower_directive(node: &DirectiveNode) -> Result<DirectiveAst, Diagnostic> {
    let range = node.range();
    match (node.directive_kind(), node.value()) {
        (Some(DirectiveKind::Version), Some(_)) => {
            let value = node.raw_value().ok_or_else(|| {
                lower_error(
                    "version.invalid-format",
                    "use `!version A.B`, for example `!version 0.1`",
                    range,
                )
            })?;
            let requirement = parse_version_requirement(&value, range)?;
            Ok(DirectiveAst::Version {
                major: requirement.major,
                minor: requirement.minor,
                range,
            })
        }
        (Some(DirectiveKind::Version), None) => Err(lower_error(
            "version.invalid-format",
            "use `!version A.B`, for example `!version 0.1`",
            range,
        )),
        (Some(DirectiveKind::Shell), Some(shell)) => Ok(DirectiveAst::Shell {
            shell: ShellKind::parse(&shell),
            range,
        }),
        (Some(DirectiveKind::Shell), None) => Err(lower_error(
            "lower.invalid-directive",
            "`!shell` needs a value",
            range,
        )),
        (Some(DirectiveKind::Var), Some(value)) => lower_variable(node, &value, range),
        (Some(DirectiveKind::Var), None) => Err(lower_error(
            "variable.non-literal",
            "use `!var name = \"value\"`",
            range,
        )),
        (Some(DirectiveKind::Unknown(name)), _) => Err(lower_error(
            "lower.invalid-directive",
            &format!("`!{name}` is not supported"),
            range,
        )),
        (None, _) => Err(lower_error(
            "lower.invalid-directive",
            "invalid directive",
            range,
        )),
    }
}

fn lower_variable(
    node: &DirectiveNode,
    value: &str,
    range: TextRange,
) -> Result<DirectiveAst, Diagnostic> {
    let Some((name, raw_value)) = value.split_once('=') else {
        return Err(lower_error(
            "variable.non-literal",
            "use `!var name = \"value\"`",
            range,
        ));
    };
    let name = name.trim();
    if !valid_identifier(name) {
        return Err(lower_error(
            "variable.invalid-name",
            "variable name is invalid",
            range,
        ));
    }
    let raw_value = raw_value.trim();
    let Some(value) = parse_string_literal(raw_value) else {
        return Err(lower_error(
            "variable.non-literal",
            "variable value must be a string",
            range,
        ));
    };

    let name_range = node.argument_name_range().unwrap_or(range);
    Ok(DirectiveAst::Variable {
        name: SmolStr::new(name),
        value: SmolStr::new(value),
        name_range,
        range,
    })
}

fn valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || matches!(first, '_' | '-'))
        && chars
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

fn semantic_error(code: &str, message: &str, range: TextRange) -> Diagnostic {
    Diagnostic::new(
        DiagnosticSeverity::Error,
        DiagnosticCode::new(code),
        message,
        DiagnosticPhase::Semantic,
        range,
    )
}

fn lower_task_comments(nodes: &[MetadataNode]) -> (Option<SmolStr>, TaskMetadataAst) {
    let metadata = lower_metadata_comments(nodes);
    (metadata.help.clone(), metadata)
}

fn lower_metadata_comments(nodes: &[MetadataNode]) -> TaskMetadataAst {
    let mut help = Vec::new();
    let mut desc = Vec::new();
    let mut pass = Vec::new();
    let mut fail = Vec::new();
    let mut unknown_fields = Vec::new();
    let mut has_structured_fields = false;
    let mut help_count = 0;
    let mut help_ranges = Vec::new();

    for node in nodes {
        if let Some((field, value)) = node.field() {
            match MetadataKind::parse(field.as_str()) {
                MetadataKind::Help => {
                    has_structured_fields = true;
                    help_count += 1;
                    help_ranges.push(node.tag_range().unwrap_or_else(|| node.range()));
                    help.push(value);
                }
                MetadataKind::Desc => {
                    has_structured_fields = true;
                    desc.push(value);
                }
                MetadataKind::Pass => {
                    has_structured_fields = true;
                    pass.push(value);
                }
                MetadataKind::Fail => {
                    has_structured_fields = true;
                    fail.push(value);
                }
                MetadataKind::Unknown(_) => unknown_fields.push(field),
            }
        }
    }

    TaskMetadataAst {
        help: join_comment_lines(&help),
        help_count,
        help_ranges,
        desc: join_comment_lines(&desc),
        pass: join_comment_lines(&pass),
        fail: join_comment_lines(&fail),
        unknown_fields,
        has_structured_fields,
    }
}

fn join_comment_lines(lines: &[SmolStr]) -> Option<SmolStr> {
    (!lines.is_empty()).then(|| {
        SmolStr::new(
            lines
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<_>>()
                .join("\n"),
        )
    })
}

fn lower_namespace(
    node: &NamespaceNode,
    docs: Vec<MetadataNode>,
) -> Result<NamespaceAst, Diagnostic> {
    let range = node.range();
    let name = node
        .name()
        .ok_or_else(|| lower_error("lower.invalid-namespace", "invalid group", range))?;

    let metadata = lower_metadata_comments(&docs);
    Ok(NamespaceAst {
        name,
        doc: metadata.help.clone(),
        metadata,
        range,
        close_range: None,
    })
}

fn lower_task(
    node: &TaskNode,
    namespace: Option<SmolStr>,
    docs: Vec<MetadataNode>,
) -> Result<TaskAst, Diagnostic> {
    let range = node.range();
    let name = node
        .name()
        .ok_or_else(|| lower_error("lower.invalid-task", "invalid task", range))?;
    let (doc, metadata) = lower_task_comments(&docs);
    let header = node.header_info();
    let shell = header.shell.as_ref().map(|shell| ShellAst {
        selection: shell.selection.clone(),
        range: shell.range,
    });

    let params = header
        .param_refs
        .iter()
        .map(|parameter| ParamAst {
            name: parameter.name.clone(),
            default_value: parameter.default_value.clone(),
            is_slice: parameter.is_slice,
            range: parameter.range,
        })
        .collect();

    let guards = header
        .guards
        .iter()
        .map(|guard| GuardAst {
            kind: guard.kind.clone(),
            argument: guard.argument.clone(),
            range: guard.range,
        })
        .collect();

    let dependencies = header
        .dependency_refs
        .into_iter()
        .map(|dependency| DependencyAst {
            name: dependency.name,
            range: dependency.range,
            arguments: dependency
                .arguments
                .into_iter()
                .map(|argument| DependencyArgumentAst {
                    value: argument.value,
                    range: argument.range,
                })
                .collect(),
            invocation_range: dependency.invocation_range,
            stage: dependency.stage,
        })
        .collect();

    let steps = node
        .steps()
        .map(|step| match step {
            only_syntax::TaskStepNode::Command(command) => TaskStepAst::Command(CommandAst {
                interpolations: scan_interpolations(command.text.as_str()),
                text: command.text,
                range: command.range,
            }),
            only_syntax::TaskStepNode::CommandBlock(block) => {
                TaskStepAst::CommandBlock(CommandBlockAst {
                    interpolations: scan_interpolations(block.source.as_str()),
                    source: block.source,
                    range: block.range,
                    line_ranges: block.line_ranges,
                })
            }
        })
        .collect();

    Ok(TaskAst {
        name,
        namespace,
        doc,
        metadata,
        params,
        guards,
        dependencies,
        shell,
        steps,
        range,
        uses_multiline_header: node.uses_multiline_header(),
    })
}

fn lower_error(code: &str, message: &str, range: TextRange) -> Diagnostic {
    Diagnostic::new(
        DiagnosticSeverity::Error,
        DiagnosticCode::new(code),
        message,
        DiagnosticPhase::Lower,
        range,
    )
}

fn parse_string_literal(input: &str) -> Option<&str> {
    input.strip_prefix('"')?.strip_suffix('"')
}
