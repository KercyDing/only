use text_size::TextRange;

use crate::{SemanticSnapshot, TaskStepAst};

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
    CommandBlock,
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

    ranges.extend(snapshot.document.tasks.iter().flat_map(|task| {
        task.steps.iter().filter_map(|step| match step {
            TaskStepAst::CommandBlock(block) if block.line_ranges.len() > 1 => Some(FoldingRange {
                range: TextRange::new(
                    block.line_ranges[0].start(),
                    block
                        .line_ranges
                        .last()
                        .expect("multi-line block must have a final line")
                        .end(),
                ),
                kind: FoldingRangeKind::CommandBlock,
            }),
            _ => None,
        })
    }));

    for namespace in &snapshot.document.namespaces {
        let namespace_range = snapshot
            .document
            .tasks
            .iter()
            .filter(|task| task.namespace.as_deref() == Some(namespace.name.as_str()))
            .fold(namespace.range, |range, task| {
                TextRange::new(range.start(), range.end().max(task.range.end()))
            });
        let namespace_range = namespace.close_range.map_or(namespace_range, |close| {
            TextRange::new(namespace_range.start(), close.end())
        });

        ranges.push(FoldingRange {
            range: namespace_range,
            kind: FoldingRangeKind::Namespace,
        });
    }

    ranges
}
