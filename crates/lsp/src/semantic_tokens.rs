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
    Metadata,
    BlockMarker,
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
        if directive.directive_kind() == Some(only_syntax::DirectiveKind::Var)
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
            range: guard.name_range,
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
        for step in task.steps() {
            match step {
                only_syntax::TaskStepNode::Command(command) => {
                    add_interpolation_tokens(snapshot, command.range, &mut tokens);
                }
                only_syntax::TaskStepNode::CommandBlock(block) => {
                    add_interpolation_tokens(snapshot, block.range, &mut tokens);
                    tokens.extend(
                        block
                            .marker_ranges
                            .into_iter()
                            .map(|range| LspSemanticToken {
                                range,
                                kind: LspSemanticTokenKind::BlockMarker,
                            }),
                    );
                }
            }
        }
    }
    for comment in snapshot.syntax.document().metadata() {
        if let Some(range) = comment.tag_range() {
            tokens.push(LspSemanticToken {
                range,
                kind: LspSemanticTokenKind::Metadata,
            });
        }
        add_interpolation_tokens(snapshot, comment.range(), &mut tokens);
    }
    tokens.sort_by_key(|token| token.range.start());
    tokens
}

fn add_interpolation_tokens(
    snapshot: &DocumentSnapshot,
    source_range: TextRange,
    tokens: &mut Vec<LspSemanticToken>,
) {
    let start = usize::from(source_range.start());
    let end = usize::from(source_range.end());
    let Some(source) = snapshot.source.get(start..end) else {
        return;
    };

    for local_range in only_semantic::interpolation_name_ranges(source) {
        let offset = text_size::TextSize::from(start as u32);
        tokens.push(LspSemanticToken {
            range: TextRange::new(offset + local_range.start(), offset + local_range.end()),
            kind: LspSemanticTokenKind::Variable,
        });
    }
}
