//! Compact indices for immutable occurrence graphs.

use super::{SemanticFlowGraph, SemanticFlowPoint, SemanticSiteId};
use std::sync::Arc;

/// Numbered method sites and synthetic program points each occupy contiguous
/// ranges. Publicly constructed graphs can also use arbitrary 64-bit IDs.
#[derive(Debug)]
pub(super) enum PointNumbers {
    Contiguous {
        after: usize,
        synthetic: usize,
        end: usize,
    },
    Sparse(Box<[SemanticFlowPoint]>),
}

impl PointNumbers {
    pub(super) fn new(points: impl Iterator<Item = SemanticFlowPoint> + Clone) -> Self {
        let mut counts = [0usize; 3];
        let mut maxima = [0u64; 3];
        for point in points.clone() {
            let (kind, raw) = Self::parts(point);
            counts[kind] += 1;
            maxima[kind] = maxima[kind].max(raw);
        }
        // All points are distinct, so a nonempty range is dense iff its
        // largest ID is exactly its cardinality minus one.
        if counts
            .iter()
            .zip(maxima)
            .all(|(&count, max)| count == 0 || max == (count - 1) as u64)
        {
            let [before, after, synthetic] = counts;
            Self::Contiguous {
                after: before,
                synthetic: before + after,
                end: before + after + synthetic,
            }
        } else {
            let mut points = points.collect::<Vec<_>>();
            points.sort_unstable();
            Self::Sparse(points.into_boxed_slice())
        }
    }

    fn parts(point: SemanticFlowPoint) -> (usize, u64) {
        match point {
            SemanticFlowPoint::Before(site) => (0, site.0),
            SemanticFlowPoint::After(site) => (1, site.0),
            SemanticFlowPoint::Synthetic(id) => (2, id),
        }
    }

    pub(super) fn len(&self) -> usize {
        match self {
            Self::Contiguous { end, .. } => *end,
            Self::Sparse(points) => points.len(),
        }
    }

    pub(super) fn get(&self, point: SemanticFlowPoint) -> Option<usize> {
        match self {
            Self::Contiguous {
                after,
                synthetic,
                end,
            } => {
                let (base, length, raw) = match point {
                    SemanticFlowPoint::Before(site) => (0, *after, site.0),
                    SemanticFlowPoint::After(site) => (*after, synthetic - after, site.0),
                    SemanticFlowPoint::Synthetic(id) => (*synthetic, end - synthetic, id),
                };
                (raw < length as u64).then(|| base + raw as usize)
            }
            Self::Sparse(points) => points.binary_search(&point).ok(),
        }
    }

    pub(super) fn point(&self, node: usize) -> SemanticFlowPoint {
        match self {
            Self::Contiguous { after, .. } if node < *after => {
                SemanticFlowPoint::Before(SemanticSiteId(node as u64))
            }
            Self::Contiguous {
                after, synthetic, ..
            } if node < *synthetic => {
                SemanticFlowPoint::After(SemanticSiteId((node - after) as u64))
            }
            Self::Contiguous { synthetic, .. } => {
                SemanticFlowPoint::Synthetic((node - synthetic) as u64)
            }
            Self::Sparse(points) => points[node],
        }
    }
}

#[derive(Debug, Clone, Default)]
struct EdgeRows {
    offsets: Vec<usize>,
    targets: Vec<usize>,
}

impl EdgeRows {
    fn push(&mut self, targets: impl Iterator<Item = usize>) {
        self.offsets.push(self.targets.len());
        self.targets.extend(targets);
    }

    fn get(&self, node: usize) -> &[usize] {
        &self.targets[self.offsets[node]..self.offsets[node + 1]]
    }
}

#[derive(Debug, Clone)]
pub(in crate::ir::analysis) struct GraphIndex {
    pub(super) points: Arc<PointNumbers>,
    successors: EdgeRows,
    predecessors: EdgeRows,
    pub(super) entries: Vec<usize>,
}

impl GraphIndex {
    pub(in crate::ir::analysis) fn len(&self) -> usize {
        self.points.len()
    }

    pub(in crate::ir::analysis) fn point(&self, node: usize) -> SemanticFlowPoint {
        self.points.point(node)
    }

    pub(in crate::ir::analysis) fn id(&self, point: SemanticFlowPoint) -> Option<usize> {
        self.points.get(point)
    }

    pub(super) fn new(graph: &SemanticFlowGraph) -> Self {
        let points = Arc::new(PointNumbers::new(graph.ordered_points()));
        let mut successors = EdgeRows::default();
        let mut predecessors = EdgeRows::default();
        for node in 0..points.len() {
            let point = points.point(node);
            successors.push(
                graph
                    .successors(point)
                    .iter()
                    .map(|&target| points.get(target).expect("indexed successor")),
            );
            predecessors.push(
                graph
                    .predecessors(point)
                    .map(|source| points.get(source).expect("indexed predecessor")),
            );
        }
        successors.offsets.push(successors.targets.len());
        predecessors.offsets.push(predecessors.targets.len());
        let entries = graph
            .entries
            .iter()
            .map(|&entry| points.get(entry).expect("indexed entry"))
            .collect();
        Self {
            points,
            successors,
            predecessors,
            entries,
        }
    }

    pub(super) fn successors(&self, node: usize) -> &[usize] {
        self.successors.get(node)
    }

    pub(in crate::ir::analysis) fn predecessors(&self, node: usize) -> &[usize] {
        self.predecessors.get(node)
    }
}
