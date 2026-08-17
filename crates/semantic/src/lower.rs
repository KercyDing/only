use only_diagnostic::{Diagnostic, DiagnosticCode, DiagnosticPhase, DiagnosticSeverity};
use only_syntax::{
    DirectiveNode, DocCommentNode, NamespaceNode, SyntaxKind, SyntaxSnapshot, TaskNode,
    parse_version_requirement,
};
use smol_str::SmolStr;
use text_size::TextRange;

use crate::interpolation::scan_interpolations;
use crate::names::resolve_dependency_names;
use crate::{
    CommandAst, CommandBlockAst, DependencyAst, DirectiveAst, DocumentAst, GuardAst, NamespaceAst,
    ParamAst, TaskAst, TaskStepAst,
};

pub(crate) fn lower_syntax(snapshot: &SyntaxSnapshot) -> (DocumentAst, Vec<Diagnostic>) {
    let document = snapshot.document();
    let mut directives = Vec::new();
    let mut namespaces = Vec::new();
    let mut tasks = Vec::new();
    let mut diagnostics = snapshot.diagnostics().to_vec();
    let mut current_namespace: Option<SmolStr> = None;
    let mut pending_doc: Option<SmolStr> = None;
    let mut saw_declaration = false;
    let mut saw_version = false;
    let mut left_directive_region = false;
    let mut uses_braced_namespaces = false;

    for node in document.syntax().children() {
        if let Some(directive) = DirectiveNode::cast(node.clone()) {
            if directive.name().as_deref() == Some("var") && left_directive_region {
                diagnostics.push(semantic_error(
                    "variable.directive-placement",
                    "`!var` must be near the top of the file",
                    directive.range(),
                ));
            }
            if directive.name().as_deref() == Some("version") {
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

        if let Some(doc_comment) = DocCommentNode::cast(node.clone()) {
            pending_doc = lower_doc_comment(&doc_comment);
            saw_declaration = true;
            continue;
        }

        if let Some(namespace) = NamespaceNode::cast(node.clone()) {
            left_directive_region = true;
            uses_braced_namespaces |= namespace.is_close() || namespace.has_open_brace();
            if namespace.is_close() {
                match current_namespace.as_deref() {
                    None => diagnostics.push(semantic_error(
                        "namespace.close-without-open",
                        "there is no namespace to close",
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
                pending_doc = None;
            } else if namespace.is_empty() {
                diagnostics.push(semantic_error(
                    "namespace.empty-label",
                    "namespace name cannot be empty",
                    namespace.range(),
                ));
            } else {
                match lower_namespace(&namespace, pending_doc.take()) {
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
            match lower_task(&task, current_namespace.clone(), pending_doc.take()) {
                Ok(task) => tasks.push(task),
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
            saw_declaration = true;
            continue;
        }

        if node.kind() == SyntaxKind::Error {
            saw_declaration = true;
        }
    }

    resolve_dependency_names(&mut tasks);

    (
        DocumentAst {
            directives,
            namespaces,
            tasks,
            uses_braced_namespaces,
        },
        diagnostics,
    )
}

fn lower_directive(node: &DirectiveNode) -> Result<DirectiveAst, Diagnostic> {
    let range = node.range();
    match (node.name().as_deref(), node.value().as_deref()) {
        (Some("version"), Some(_)) => {
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
        (Some("version"), None) => Err(lower_error(
            "version.invalid-format",
            "use `!version A.B`, for example `!version 0.1`",
            range,
        )),
        (Some("shell"), Some(shell)) => Ok(DirectiveAst::Shell {
            shell: SmolStr::new(shell),
            range,
        }),
        (Some("shell"), None) => Err(lower_error(
            "lower.invalid-directive",
            "`!shell` needs a value",
            range,
        )),
        (Some("var"), Some(value)) => lower_variable(node, value, range),
        (Some("var"), None) => Err(lower_error(
            "variable.non-literal",
            "use `!var name = \"value\"`",
            range,
        )),
        (Some(name), _) => Err(lower_error(
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

fn lower_doc_comment(node: &DocCommentNode) -> Option<SmolStr> {
    node.text()
}

fn lower_namespace(node: &NamespaceNode, doc: Option<SmolStr>) -> Result<NamespaceAst, Diagnostic> {
    let range = node.range();
    let name = node
        .name()
        .ok_or_else(|| lower_error("lower.invalid-namespace", "invalid namespace", range))?;

    Ok(NamespaceAst {
        name,
        doc,
        range,
        close_range: None,
        is_braced: node.has_open_brace(),
    })
}

fn lower_task(
    node: &TaskNode,
    namespace: Option<SmolStr>,
    doc: Option<SmolStr>,
) -> Result<TaskAst, Diagnostic> {
    let range = node.range();
    let name = node
        .name()
        .ok_or_else(|| lower_error("lower.invalid-task", "invalid task", range))?;
    let header = node.header_info();

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
        .map(|guard| parse_guard(guard.text.as_str(), guard.range))
        .collect::<Result<Vec<_>, _>>()?;

    let dependencies = header
        .dependency_refs
        .into_iter()
        .map(|dependency| DependencyAst {
            name: dependency.name,
            range: dependency.range,
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
        params,
        guards,
        dependencies,
        shell: header.shell,
        shell_fallback: header.shell_fallback,
        steps,
        range,
        uses_multiline_header: node.uses_multiline_header(),
    })
}

fn parse_guard(input: &str, range: TextRange) -> Result<GuardAst, Diagnostic> {
    let trimmed = input.trim_start();
    let Some(after_at) = trimmed.strip_prefix('@') else {
        return Err(lower_error("lower.invalid-guard", "invalid guard", range));
    };
    let Some(open) = after_at.find('(') else {
        return Err(lower_error("lower.invalid-guard", "invalid guard", range));
    };
    let Some(close) = after_at[open + 1..].find(')') else {
        return Err(lower_error("lower.invalid-guard", "invalid guard", range));
    };

    let kind = after_at[..open].trim();
    let argument = parse_string_literal(after_at[open + 1..open + 1 + close].trim())
        .ok_or_else(|| lower_error("lower.invalid-guard", "invalid guard", range))?;

    Ok(GuardAst {
        kind: SmolStr::new(kind),
        argument: SmolStr::new(argument),
        range,
    })
}

fn parse_string_literal(input: &str) -> Option<&str> {
    input.strip_prefix('"')?.strip_suffix('"')
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
