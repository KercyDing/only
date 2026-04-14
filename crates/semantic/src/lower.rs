use only_diagnostic::{Diagnostic, DiagnosticCode, DiagnosticPhase, DiagnosticSeverity};
use only_syntax::{DirectiveNode, DocCommentNode, NamespaceNode, SyntaxSnapshot, TaskNode};
use smol_str::SmolStr;
use text_size::TextRange;

use crate::interpolation::scan_interpolations;
use crate::names::resolve_dependency_names;
use crate::{
    CommandAst, DependencyAst, DirectiveAst, DocumentAst, GuardAst, NamespaceAst, ParamAst, TaskAst,
};

pub(crate) fn lower_syntax(snapshot: &SyntaxSnapshot) -> (DocumentAst, Vec<Diagnostic>) {
    let document = snapshot.document();
    let mut directives = Vec::new();
    let mut namespaces = Vec::new();
    let mut tasks = Vec::new();
    let mut diagnostics = snapshot.diagnostics().to_vec();
    let mut current_namespace: Option<SmolStr> = None;
    let mut pending_doc: Option<SmolStr> = None;

    for node in document.syntax().children() {
        if let Some(directive) = DirectiveNode::cast(node.clone()) {
            match lower_directive(&directive) {
                Ok(directive) => directives.push(directive),
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
            continue;
        }

        if let Some(doc_comment) = DocCommentNode::cast(node.clone()) {
            pending_doc = lower_doc_comment(&doc_comment);
            continue;
        }

        if let Some(namespace) = NamespaceNode::cast(node.clone()) {
            match lower_namespace(&namespace, pending_doc.take()) {
                Ok(namespace) => {
                    current_namespace = Some(namespace.name.clone());
                    namespaces.push(namespace);
                }
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
            continue;
        }

        if let Some(task) = TaskNode::cast(node) {
            match lower_task(&task, current_namespace.clone(), pending_doc.take()) {
                Ok(task) => tasks.push(task),
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
        }
    }

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

fn lower_directive(node: &DirectiveNode) -> Result<DirectiveAst, Diagnostic> {
    let range = node.range();
    match (node.name().as_deref(), node.value().as_deref()) {
        (Some("echo"), Some("true")) => Ok(DirectiveAst::Echo { value: true, range }),
        (Some("echo"), Some("false")) => Ok(DirectiveAst::Echo {
            value: false,
            range,
        }),
        (Some("preview"), Some("true")) => Ok(DirectiveAst::Preview { value: true, range }),
        (Some("preview"), Some("false")) => Ok(DirectiveAst::Preview {
            value: false,
            range,
        }),
        (Some("echo"), Some(value)) => Err(lower_error(
            "lower.invalid-directive",
            &format!("invalid echo value '{value}': expected 'true' or 'false'"),
            range,
        )),
        (Some("preview"), Some(value)) => Err(lower_error(
            "lower.invalid-directive",
            &format!("invalid preview value '{value}': expected 'true' or 'false'"),
            range,
        )),
        (Some("echo"), None) => Err(lower_error(
            "lower.invalid-directive",
            "directive '!echo' requires a value",
            range,
        )),
        (Some("preview"), None) => Err(lower_error(
            "lower.invalid-directive",
            "directive '!preview' requires a value",
            range,
        )),
        (Some("shell"), Some(shell)) => Ok(DirectiveAst::Shell {
            shell: SmolStr::new(shell),
            range,
        }),
        (Some("shell"), None) => Err(lower_error(
            "lower.invalid-directive",
            "directive '!shell' requires a value",
            range,
        )),
        (Some(name), _) => Err(lower_error(
            "lower.invalid-directive",
            &format!("unknown directive '!{name}'"),
            range,
        )),
        (None, _) => Err(lower_error(
            "lower.invalid-directive",
            "failed to lower directive",
            range,
        )),
    }
}

fn lower_doc_comment(node: &DocCommentNode) -> Option<SmolStr> {
    node.text()
}

fn lower_namespace(node: &NamespaceNode, doc: Option<SmolStr>) -> Result<NamespaceAst, Diagnostic> {
    let range = node.range();
    let name = node.name().ok_or_else(|| {
        lower_error(
            "lower.invalid-namespace",
            "failed to lower namespace",
            range,
        )
    })?;

    Ok(NamespaceAst { name, doc, range })
}

fn lower_task(
    node: &TaskNode,
    namespace: Option<SmolStr>,
    doc: Option<SmolStr>,
) -> Result<TaskAst, Diagnostic> {
    let range = node.range();
    let name = node
        .name()
        .ok_or_else(|| lower_error("lower.invalid-task", "failed to lower task", range))?;
    let header = node.header_info();

    let params = header
        .params
        .as_deref()
        .map(parse_params)
        .unwrap_or_default();

    let guard = match header.guard.as_deref() {
        Some(text) => Some(parse_guard(text, range)?),
        None => None,
    };

    let dependencies = header
        .dependency_refs
        .into_iter()
        .map(|dependency| DependencyAst {
            name: dependency.name,
            range: dependency.range,
            stage: dependency.stage,
        })
        .collect();

    let commands = node
        .commands()
        .map(|line| CommandAst {
            interpolations: scan_interpolations(line.as_str()),
            text: line,
        })
        .collect();

    Ok(TaskAst {
        name,
        namespace,
        doc,
        params,
        guard,
        dependencies,
        shell: header.shell,
        shell_fallback: header.shell_fallback,
        commands,
        range,
    })
}

fn parse_params(section: &str) -> Vec<ParamAst> {
    section
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (name, default_value) = match part.split_once('=') {
                Some((name, value)) => (
                    name.trim(),
                    parse_string_literal(value.trim()).map(SmolStr::new),
                ),
                None => (part, None),
            };
            ParamAst {
                name: SmolStr::new(name),
                default_value,
            }
        })
        .collect()
}

fn parse_guard(input: &str, range: TextRange) -> Result<GuardAst, Diagnostic> {
    let trimmed = input.trim_start();
    let Some(after_at) = trimmed.strip_prefix('@') else {
        return Err(lower_error(
            "lower.invalid-guard",
            "failed to lower guard",
            range,
        ));
    };
    let Some(open) = after_at.find('(') else {
        return Err(lower_error(
            "lower.invalid-guard",
            "failed to lower guard",
            range,
        ));
    };
    let Some(close) = after_at[open + 1..].find(')') else {
        return Err(lower_error(
            "lower.invalid-guard",
            "failed to lower guard",
            range,
        ));
    };

    let kind = after_at[..open].trim();
    let argument = parse_string_literal(after_at[open + 1..open + 1 + close].trim())
        .ok_or_else(|| lower_error("lower.invalid-guard", "failed to lower guard", range))?;

    Ok(GuardAst {
        kind: SmolStr::new(kind),
        argument: SmolStr::new(argument),
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
