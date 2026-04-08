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
    Namespace,
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
    only_semantic::hover_at(&snapshot.semantic, offset).map(|hover| LspHover {
        kind: match hover.kind {
            only_semantic::HoverKind::Namespace => LspHoverKind::Namespace,
            only_semantic::HoverKind::Task => LspHoverKind::Task,
        },
        name: hover.name.to_string(),
        signature: hover.signature.to_string(),
        docs: hover.docs.map(|docs| docs.to_string()),
        range: hover.range,
        container_name: hover.container_name.map(|name| name.to_string()),
    })
}
