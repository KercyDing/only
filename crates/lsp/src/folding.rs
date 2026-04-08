use text_size::TextRange;

use crate::DocumentSnapshot;

/// Host-facing folding range kind used by the LSP crate.
///
/// Args:
/// None.
///
/// Returns:
/// Stable folding categories for editor protocol conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspFoldingRangeKind {
    Namespace,
    Task,
}

/// Host-facing folding range entry for one source item.
///
/// Args:
/// None.
///
/// Returns:
/// Foldable range detached from semantic internals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspFoldingRange {
    pub range: TextRange,
    pub kind: LspFoldingRangeKind,
}

/// Builds folding ranges from one in-memory document snapshot.
///
/// Args:
/// snapshot: In-memory document snapshot with semantic analysis.
///
/// Returns:
/// LSP host-facing folding ranges for namespaces and tasks.
pub fn folding_ranges(snapshot: &DocumentSnapshot) -> Vec<LspFoldingRange> {
    only_semantic::folding_ranges(&snapshot.semantic)
        .into_iter()
        .map(|range| LspFoldingRange {
            range: range.range,
            kind: match range.kind {
                only_semantic::FoldingRangeKind::Namespace => LspFoldingRangeKind::Namespace,
                only_semantic::FoldingRangeKind::Task => LspFoldingRangeKind::Task,
            },
        })
        .collect()
}
