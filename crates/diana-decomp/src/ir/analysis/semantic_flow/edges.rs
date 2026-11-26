//! Frozen adjacency for occurrence-sensitive flow. The builder appends edges
//! in evaluation order; freezing sorts and deduplicates each public edge row.

use std::{collections::BTreeSet, sync::Arc};

use super::{index::PointNumbers, SemanticFlowEdgeKind, SemanticFlowPoint};

pub(super) type FlowEdge = (SemanticFlowPoint, SemanticFlowPoint, SemanticFlowEdgeKind);

#[derive(Debug, Clone)]
struct Rows<T> {
    offsets: Vec<usize>,
    items: Vec<T>,
}

impl<T: Copy + Ord> Rows<T> {
    fn build(count: usize, pairs: impl Iterator<Item = (usize, T)> + Clone) -> Self {
        let mut offsets = vec![0; count + 1];
        let Some((_, initial)) = pairs.clone().next() else {
            return Self {
                offsets,
                items: Vec::new(),
            };
        };
        for (row, _) in pairs.clone() {
            offsets[row + 1] += 1;
        }
        for i in 1..offsets.len() {
            offsets[i] += offsets[i - 1];
        }
        let mut items = vec![initial; offsets[count]];
        let mut cursor = offsets[..count].to_vec();
        for (row, item) in pairs {
            items[cursor[row]] = item;
            cursor[row] += 1;
        }
        let mut write = 0;
        for row in 0..count {
            let start = offsets[row];
            let end = offsets[row + 1];
            offsets[row] = write;
            if end == start + 1 {
                items[write] = items[start];
                write += 1;
            } else if end > start {
                items[start..end].sort_unstable();
                for read in start..end {
                    if write == offsets[row] || items[read] != items[write - 1] {
                        items[write] = items[read];
                        write += 1;
                    }
                }
            }
        }
        offsets[count] = write;
        items.truncate(write);
        Self { offsets, items }
    }

    fn get(&self, index: usize) -> &[T] {
        &self.items[self.offsets[index]..self.offsets[index + 1]]
    }
}

#[derive(Debug, Clone)]
pub(super) struct FlowEdges {
    pub(super) points: Arc<PointNumbers>,
    successors: Rows<SemanticFlowPoint>,
    predecessors: Rows<(SemanticFlowPoint, SemanticFlowEdgeKind)>,
    pub(super) cleanup: bool,
}

impl Default for FlowEdges {
    fn default() -> Self {
        Self::new(&[], &BTreeSet::new())
    }
}

impl FlowEdges {
    pub(super) fn new(edges: &[FlowEdge], entries: &BTreeSet<SemanticFlowPoint>) -> Self {
        let observations = edges.len().saturating_mul(2).saturating_add(entries.len());
        let points = Arc::new(PointNumbers::from_occurrences(
            edges
                .iter()
                .flat_map(|&(source, target, _)| [source, target])
                .chain(entries.iter().copied()),
            observations,
        ));
        let count = points.len();
        let successors = Rows::build(
            count,
            edges.iter().map(|&(source, target, _)| {
                (points.get(source).expect("edge source is indexed"), target)
            }),
        );
        let predecessors = Rows::build(
            count,
            edges.iter().map(|&(source, target, kind)| {
                (
                    points.get(target).expect("edge target is indexed"),
                    (source, kind),
                )
            }),
        );
        let cleanup = edges.iter().any(|&(_, _, edge)| {
            matches!(
                edge,
                SemanticFlowEdgeKind::FinallyEnter(_)
                    | SemanticFlowEdgeKind::FinallyResume(_)
                    | SemanticFlowEdgeKind::FinallyOverride
            )
        });
        Self {
            points,
            successors,
            predecessors,
            cleanup,
        }
    }

    pub(super) fn successors(&self, point: SemanticFlowPoint) -> &[SemanticFlowPoint] {
        self.points
            .get(point)
            .map(|row| self.successors.get(row))
            .unwrap_or_default()
    }

    pub(super) fn predecessors(
        &self,
        point: SemanticFlowPoint,
    ) -> &[(SemanticFlowPoint, SemanticFlowEdgeKind)] {
        self.points
            .get(point)
            .map(|row| self.predecessors.get(row))
            .unwrap_or_default()
    }
}
