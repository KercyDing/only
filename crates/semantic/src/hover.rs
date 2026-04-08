use smol_str::SmolStr;
use text_size::{TextRange, TextSize};

use crate::SemanticSnapshot;

/// Kind of hover result produced for editor consumers.
///
/// Args:
/// None.
///
/// Returns:
/// Stable hover categories for namespace and task items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverKind {
    Namespace,
    Task,
}

/// Hover payload for one source location.
///
/// Args:
/// None.
///
/// Returns:
/// Name, signature, docs and selected range for the hovered item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverInfo {
    pub kind: HoverKind,
    pub name: SmolStr,
    pub signature: SmolStr,
    pub docs: Option<SmolStr>,
    pub range: TextRange,
    pub container_name: Option<SmolStr>,
}

/// Resolves hover information at one source offset.
///
/// Args:
/// snapshot: Immutable semantic snapshot for one document version.
/// offset: Source offset queried by the editor host.
///
/// Returns:
/// Hover information for the matching namespace or task item.
pub fn hover_at(snapshot: &SemanticSnapshot, offset: TextSize) -> Option<HoverInfo> {
    for namespace in &snapshot.document.namespaces {
        if namespace.range.contains(offset) {
            return Some(HoverInfo {
                kind: HoverKind::Namespace,
                name: namespace.name.clone(),
                signature: SmolStr::from(format!("[{}]", namespace.name)),
                docs: namespace.doc.clone(),
                range: namespace.range,
                container_name: None,
            });
        }
    }

    for task in &snapshot.document.tasks {
        if task.range.contains(offset) {
            return Some(HoverInfo {
                kind: HoverKind::Task,
                name: task.name.clone(),
                signature: task.signature(),
                docs: task.doc.clone(),
                range: task.range,
                container_name: task.namespace.clone(),
            });
        }
    }

    None
}
