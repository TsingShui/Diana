//! Compact indices for immutable occurrence graphs.

use super::{GraphIndex, SemanticFlowPoint, SemanticSiteId};

/// Site ordinals occupy separate before, after and synthetic ranges. The
/// compact graph index uses their public ordering even when some sites have
/// no reachable after-point (return, throw or transfer).
#[derive(Debug)]
pub(super) enum PointNumbers {
    Contiguous(PointLayout),
    Gapped {
        layout: PointLayout,
        words: Box<[RankWord]>,
        slots: Box<[usize]>,
    },
    Sparse(Box<[SemanticFlowPoint]>),
}

#[derive(Debug)]
pub(super) struct PointLayout {
    after: usize,
    synthetic: usize,
    end: usize,
}

impl PointLayout {
    fn slot(&self, point: SemanticFlowPoint) -> Option<usize> {
        let (base, length, raw) = match point {
            SemanticFlowPoint::Before(site) => (0, self.after, site.0),
            SemanticFlowPoint::After(site) => (self.after, self.synthetic - self.after, site.0),
            SemanticFlowPoint::Synthetic(id) => (self.synthetic, self.end - self.synthetic, id),
        };
        (raw < length as u64).then(|| base + raw as usize)
    }

    fn point(&self, slot: usize) -> SemanticFlowPoint {
        if slot < self.after {
            SemanticFlowPoint::Before(SemanticSiteId(slot as u64))
        } else if slot < self.synthetic {
            SemanticFlowPoint::After(SemanticSiteId((slot - self.after) as u64))
        } else {
            SemanticFlowPoint::Synthetic((slot - self.synthetic) as u64)
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub(super) struct RankWord {
    present: u64,
    preceding: usize,
}

impl PointNumbers {
    pub(super) fn from_occurrences(
        points: impl Iterator<Item = SemanticFlowPoint> + Clone,
        observations: usize,
    ) -> Self {
        let mut lengths = [0usize; 3];
        for point in points.clone() {
            let (kind, raw) = match point {
                SemanticFlowPoint::Before(site) => (0, site.0),
                SemanticFlowPoint::After(site) => (1, site.0),
                SemanticFlowPoint::Synthetic(id) => (2, id),
            };
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
        if count > observations {
            return Self::sparse(points);
        }
        let layout = PointLayout {
            after: before,
            synthetic: before + after,
            end: count,
        };
        let mut words = smallvec::SmallVec::<[RankWord; 2]>::new();
        words.resize(count.div_ceil(64), RankWord::default());
        for point in points {
            let slot = layout.slot(point).expect("within observed range");
            words[slot / 64].present |= 1 << (slot % 64);
        }
        let mut distinct = 0;
        for word in &mut words {
            word.preceding = distinct;
            distinct += word.present.count_ones() as usize;
        }
        if distinct == count {
            return Self::Contiguous(layout);
        }
        let mut slots = Vec::with_capacity(distinct);
        for (index, word) in words.iter().enumerate() {
            let mut bits = word.present;
            while bits != 0 {
                slots.push(index * 64 + bits.trailing_zeros() as usize);
                bits &= bits - 1;
            }
        }
        Self::Gapped {
            layout,
            words: words.into_boxed_slice(),
            slots: slots.into_boxed_slice(),
        }
    }

    fn sparse(points: impl Iterator<Item = SemanticFlowPoint>) -> Self {
        let mut ordered = points.collect::<Vec<_>>();
        ordered.sort_unstable();
        ordered.dedup();
        Self::Sparse(ordered.into_boxed_slice())
    }

    pub(super) fn len(&self) -> usize {
        match self {
            Self::Contiguous(layout) => layout.end,
            Self::Gapped { slots, .. } => slots.len(),
            Self::Sparse(points) => points.len(),
        }
    }

    pub(super) fn get(&self, point: SemanticFlowPoint) -> Option<usize> {
        match self {
            Self::Contiguous(layout) => layout.slot(point),
            Self::Gapped { layout, words, .. } => {
                let slot = layout.slot(point)?;
                let word = &words[slot / 64];
                let bit = 1 << (slot % 64);
                (word.present & bit != 0)
                    .then(|| word.preceding + (word.present & (bit - 1)).count_ones() as usize)
            }
            Self::Sparse(points) => points.binary_search(&point).ok(),
        }
    }

    pub(super) fn point(&self, node: usize) -> SemanticFlowPoint {
        match self {
            Self::Contiguous(layout) => layout.point(node),
            Self::Gapped { layout, slots, .. } => layout.point(slots[node]),
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
