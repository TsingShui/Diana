//! Compact indices for immutable occurrence graphs.

use super::{GraphIndex, SemanticFlowPoint, SemanticSiteId};

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
    pub(super) fn from_occurrences(
        points: impl Iterator<Item = SemanticFlowPoint> + Clone,
        observations: usize,
    ) -> Self {
        let mut lengths = [0usize; 3];
        for point in points.clone() {
            let (kind, raw) = Self::parts(point);
            let Some(end) = usize::try_from(raw).ok().and_then(|id| id.checked_add(1)) else {
                return Self::sparse(points);
            };
            lengths[kind] = lengths[kind].max(end);
        }
        let [before, after, synthetic] = lengths;
        let Some(count) = before
            .checked_add(after)
            .and_then(|sum| sum.checked_add(synthetic))
        else {
            return Self::sparse(points);
        };
        // Publicly constructed graphs can use arbitrary 64-bit IDs. Only
        // allocate a direct membership table that the input length bounds.
        if count > observations {
            return Self::sparse(points);
        }
        let dense = Self::Contiguous {
            after: before,
            synthetic: before + after,
            end: count,
        };
        let mut seen = vec![false; count];
        let mut distinct = 0;
        for point in points {
            let slot = dense.get(point).expect("within observed range");
            if !std::mem::replace(&mut seen[slot], true) {
                distinct += 1;
            }
        }
        if distinct == count {
            dense
        } else {
            Self::Sparse(
                seen.into_iter()
                    .enumerate()
                    .filter(|(_, present)| *present)
                    .map(|(id, _)| dense.point(id))
                    .collect(),
            )
        }
    }

    fn sparse(points: impl Iterator<Item = SemanticFlowPoint>) -> Self {
        let mut ordered = points.collect::<Vec<_>>();
        ordered.sort_unstable();
        ordered.dedup();
        Self::Sparse(ordered.into_boxed_slice())
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
    fn get(&self, node: usize) -> &[usize] {
        &self.targets[self.offsets[node]..self.offsets[node + 1]]
    }
}

/// Maximal straight-line runs reachable from an entry. A branch, join or
/// additional entry starts a new run even when the occurrences share a DEX site.
#[derive(Debug, Clone)]
pub(super) struct FlowBlocks {
    owner: Vec<usize>,
    points: EdgeRows,
    pub(super) order: Vec<usize>,
}

impl FlowBlocks {
    pub(super) fn new(index: &GraphIndex) -> Self {
        let mut owner = vec![usize::MAX; index.len()];
        let mut points = EdgeRows::default();
        let mut entry = vec![false; index.len()];
        for &node in &index.entries {
            entry[node] = true;
        }
        let mut pending = index.entries.clone();
        while let Some(mut node) = pending.pop() {
            if owner[node] != usize::MAX {
                continue;
            }
            let block = points.offsets.len();
            points.offsets.push(points.targets.len());
            loop {
                owner[node] = block;
                points.targets.push(node);
                let successors = index.successors(node);
                match successors {
                    [next]
                        if !entry[*next]
                            && index.predecessors(*next).len() == 1
                            && owner[*next] == usize::MAX =>
                    {
                        node = *next
                    }
                    _ => {
                        pending.extend(successors);
                        break;
                    }
                }
            }
        }
        let count = points.offsets.len();
        points.offsets.push(points.targets.len());

        // Reverse postorder visits each acyclic predecessor before its
        // successor. Only loop back edges can reopen an already solved block.
        let mut order = Vec::with_capacity(count);
        let mut visited = vec![false; count];
        let mut stack = Vec::new();
        for &entry in &index.entries {
            let root = owner[entry];
            if std::mem::replace(&mut visited[root], true) {
                continue;
            }
            stack.push((root, 0));
            while let Some((block, cursor)) = stack.last_mut() {
                let last = *points.get(*block).last().expect("nonempty flow block");
                if let Some(&successor) = index.successors(last).get(*cursor) {
                    *cursor += 1;
                    let next = owner[successor];
                    if !std::mem::replace(&mut visited[next], true) {
                        stack.push((next, 0));
                    }
                } else {
                    order.push(*block);
                    stack.pop();
                }
            }
        }
        order.reverse();
        Self {
            owner,
            points,
            order,
        }
    }

    pub(super) fn owner(&self, point: usize) -> Option<usize> {
        let block = self.owner[point];
        (block != usize::MAX).then_some(block)
    }

    pub(super) fn points(&self, block: usize) -> &[usize] {
        self.points.get(block)
    }
}
