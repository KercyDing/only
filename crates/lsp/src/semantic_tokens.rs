use text_size::TextRange;

use crate::DocumentSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspSemanticTokenKind {
    Directive,
    Namespace,
    Task,
    Parameter,
    Guard,
    Dependency,
    Shell,
    Variable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspSemanticToken {
    pub range: TextRange,
    pub kind: LspSemanticTokenKind,
}

pub fn semantic_tokens(snapshot: &DocumentSnapshot) -> Vec<LspSemanticToken> {
    let mut tokens = Vec::new();
    for directive in snapshot.syntax.document().directives() {
        if let Some(range) = directive.keyword_range() {
            tokens.push(LspSemanticToken {
                range,
                kind: LspSemanticTokenKind::Directive,
            });
        }
        if directive.name().as_deref() == Some("var")
            && let Some(range) = directive.argument_name_range()
        {
            tokens.push(LspSemanticToken {
                range,
                kind: LspSemanticTokenKind::Variable,
            });
        }
    }
    for namespace in snapshot.syntax.document().namespaces() {
        if let Some(range) = namespace.name_range() {
            tokens.push(LspSemanticToken {
                range,
                kind: LspSemanticTokenKind::Namespace,
            });
        }
    }
    for task in snapshot.syntax.document().tasks() {
        if let Some(range) = task.name_range() {
            tokens.push(LspSemanticToken {
                range,
                kind: LspSemanticTokenKind::Task,
            });
        }
        let header = task.header_info();
        tokens.extend(
            header
                .param_refs
                .into_iter()
                .map(|parameter| LspSemanticToken {
                    range: parameter.range,
                    kind: LspSemanticTokenKind::Parameter,
                }),
        );
        tokens.extend(header.guards.into_iter().map(|guard| LspSemanticToken {
            range: guard.range,
            kind: LspSemanticTokenKind::Guard,
        }));
        tokens.extend(
            header
                .dependency_refs
                .into_iter()
                .map(|dependency| LspSemanticToken {
                    range: dependency.range,
                    kind: LspSemanticTokenKind::Dependency,
                }),
        );
        if let Some(shell) = task.header().and_then(|header| header.shell())
            && let Some(range) = shell.content_range()
        {
            tokens.push(LspSemanticToken {
                range,
                kind: LspSemanticTokenKind::Shell,
            });
        }
    }
    tokens.sort_by_key(|token| token.range.start());
    tokens
}
