use text_size::TextRange;

use crate::SemanticSnapshot;

/// Kind of folding range produced for editor consumers.
///
/// Args:
/// None.
///
/// Returns:
/// Stable folding categories for namespace and task blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldingRangeKind {
    Namespace,
    Task,
}

/// Foldable source span for editor consumers.
///
/// Args:
/// None.
///
/// Returns:
/// One semantic folding range and its category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldingRange {
    pub range: TextRange,
    pub kind: FoldingRangeKind,
}

/// Builds folding ranges from one semantic snapshot.
///
/// Args:
/// snapshot: Immutable semantic snapshot for one document version.
///
/// Returns:
/// Foldable ranges for namespace blocks and task blocks.
pub fn folding_ranges(snapshot: &SemanticSnapshot) -> Vec<FoldingRange> {
    let mut ranges = snapshot
        .document
        .tasks
        .iter()
        .map(|task| FoldingRange {
            range: task.range,
            kind: FoldingRangeKind::Task,
        })
        .collect::<Vec<_>>();

    for namespace in &snapshot.document.namespaces {
        let namespace_range = snapshot
            .document
            .tasks
            .iter()
            .filter(|task| task.namespace.as_deref() == Some(namespace.name.as_str()))
            .fold(namespace.range, |range, task| {
                TextRange::new(range.start(), range.end().max(task.range.end()))
            });

        ranges.push(FoldingRange {
            range: namespace_range,
            kind: FoldingRangeKind::Namespace,
        });
    }

    ranges
}
