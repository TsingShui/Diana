//! Reaching definitions on graphs without continuation-sensitive finally edges.

use super::{
    DefinitionSet, DefinitionsByValue, PointNumbers, ReachingStorage, SemanticFlowGraph,
    SemanticFlowPoint, SemanticReachingValues, SemanticValueDefinition,
};
use std::collections::{BTreeSet, VecDeque};
use std::ops::Range;
use std::sync::Arc;

/// A write on a chain replaces exactly the preceding write group to its value.
/// Simultaneous definitions retain input order, including repeated ordinals.
#[derive(Debug, Clone)]
pub(super) struct LinearReaching {
    points: Arc<PointNumbers>,
    target_ranks: Vec<usize>,
    offsets: Vec<usize>,
    writes: Vec<(usize, usize)>,
}

impl LinearReaching {
    pub(super) fn new(
        numbers: &Arc<PointNumbers>,
        points: &[SemanticFlowPoint],
        definitions: &[SemanticValueDefinition],
        value_count: usize,
        targets: &BTreeSet<SemanticFlowPoint>,
    ) -> Self {
        let mut ranks = vec![usize::MAX; numbers.len()];
        for (rank, &point) in points.iter().enumerate() {
            ranks[numbers.get(point).expect("indexed linear point")] = rank;
        }
        let offsets = DefinitionsByValue::offsets(definitions, value_count);
        let mut next = offsets[..value_count].to_vec();
        let mut writes = vec![(0, 0); definitions.len()];
        for def in definitions {
            let rank = numbers.get(def.point).map_or(usize::MAX, |id| ranks[id]);
            writes[next[def.value]] = (rank, def.ordinal);
            next[def.value] += 1;
        }
        for range in offsets.windows(2) {
            writes[range[0]..range[1]].sort_by_key(|&(rank, _)| rank);
        }
        for (id, rank) in ranks.iter_mut().enumerate() {
            if !targets.contains(&numbers.point(id)) {
                *rank = usize::MAX;
            }
        }
        Self {
            points: Arc::clone(numbers),
            target_ranks: ranks,
            offsets,
            writes,
        }
    }

    fn target(&self, point: SemanticFlowPoint) -> Option<usize> {
        let rank = self.target_ranks[self.points.get(point)?];
        (rank != usize::MAX).then_some(rank)
    }

    fn writes(&self, value: usize) -> Option<&[(usize, usize)]> {
        let &start = self.offsets.get(value)?;
        let &end = self.offsets.get(value.checked_add(1)?)?;
        Some(&self.writes[start..end])
    }

    pub(super) fn at(&self, value: usize, point: SemanticFlowPoint) -> Option<Vec<usize>> {
        let rank = self.target(point)?;
        let writes = self.writes(value).unwrap_or_default();
        let preceding = &writes[..writes.partition_point(|&(position, _)| position <= rank)];
        let Some(&(latest, _)) = preceding.last() else {
            return Some(Vec::new());
        };
        let group = preceding.partition_point(|&(position, _)| position < latest);
        Some(
            preceding[group..]
                .iter()
                .map(|&(_, ordinal)| ordinal)
                .collect(),
        )
    }

    pub(super) fn unchanged(
        &self,
        value: usize,
        source: SemanticFlowPoint,
        target: SemanticFlowPoint,
    ) -> Option<bool> {
        let source = self.target(source)?;
        let target = self.target(target)?;
        let writes = self.writes(value)?;
        // Distinct writes may carry the same caller-supplied ordinal.
        Some(
            writes.partition_point(|&(rank, _)| rank <= source)
                == writes.partition_point(|&(rank, _)| rank <= target),
        )
    }
}

#[derive(Clone)]
struct BlockState {
    incoming: DefinitionSet,
    pending: DefinitionSet,
    generated: DefinitionSet,
    killed: Option<DefinitionSet>,
    first: bool,
    queued: bool,
}

impl BlockState {
    fn new(width: usize) -> Self {
        Self {
            incoming: DefinitionSet::new(width),
            pending: DefinitionSet::new(width),
            generated: DefinitionSet::new(width),
            killed: None,
            first: true,
            queued: true,
        }
    }
}

#[derive(Clone, Debug)]
struct BlockInput {
    start: usize,
    incoming: DefinitionSet,
}

/// Only block boundaries carry solved sets. Inside one run, the last local
/// write at or before a target replaces the block input for its value.
#[derive(Clone, Debug)]
pub(super) struct BlockReaching {
    points: Arc<PointNumbers>,
    target_ranks: Vec<usize>,
    inputs: Vec<BlockInput>,
    by_value: DefinitionsByValue,
    // Sorted per value by (point rank, definition bit). The bit breaks ties
    // in caller order and identifies the definition independently of ordinal.
    writes: Vec<(usize, usize)>,
}

enum PointValue<'a> {
    Unreachable,
    Incoming(&'a DefinitionSet),
    Local(&'a [(usize, usize)]),
}

impl BlockReaching {
    const UNREACHABLE: usize = usize::MAX;
    const UNREQUESTED: usize = usize::MAX - 1;

    fn rank(&self, point: SemanticFlowPoint) -> Option<usize> {
        let rank = self.target_ranks[self.points.get(point)?];
        (rank != Self::UNREQUESTED).then_some(rank)
    }

    fn value_at(&self, rank: usize, range: Range<usize>) -> PointValue<'_> {
        if rank == Self::UNREACHABLE {
            return PointValue::Unreachable;
        }
        let block = self.inputs.partition_point(|input| input.start <= rank) - 1;
        let input = &self.inputs[block];
        let writes = &self.writes[range];
        let preceding = &writes[..writes.partition_point(|&(position, _)| position <= rank)];
        if let Some(&(last, _)) = preceding.last() {
            if last >= input.start {
                let first = preceding.partition_point(|&(position, _)| position < last);
                return PointValue::Local(&preceding[first..]);
            }
        }
        PointValue::Incoming(&input.incoming)
    }

    pub(super) fn at(&self, value: usize, point: SemanticFlowPoint) -> Option<Vec<usize>> {
        let rank = self.rank(point)?;
        let range = self.by_value.range(value).unwrap_or(0..0);
        let ordinal = |bit: usize| self.by_value.ordinals[bit];
        Some(match self.value_at(rank, range.clone()) {
            PointValue::Unreachable => Vec::new(),
            PointValue::Incoming(facts) => facts.members(range).map(ordinal).collect(),
            PointValue::Local(writes) => writes.iter().map(|&(_, bit)| ordinal(bit)).collect(),
        })
    }

    pub(super) fn unchanged(
        &self,
        value: usize,
        source: SemanticFlowPoint,
        target: SemanticFlowPoint,
    ) -> Option<bool> {
        use PointValue::{Incoming, Local, Unreachable};
        let source = self.rank(source)?;
        let target = self.rank(target)?;
        let range = self.by_value.range(value)?;
        if source == target || range.is_empty() {
            return Some(true);
        }
        Some(
            match (
                self.value_at(source, range.clone()),
                self.value_at(target, range.clone()),
            ) {
                (Incoming(a), Incoming(b)) => std::ptr::eq(a, b) || a.same_in_range(b, range),
                (Local(a), Local(b)) => a[0].0 == b[0].0,
                (Incoming(facts), Local(writes)) | (Local(writes), Incoming(facts)) => {
                    facts.members(range).eq(writes.iter().map(|&(_, bit)| bit))
                }
                (Unreachable, Incoming(facts)) | (Incoming(facts), Unreachable) => {
                    facts.members(range).next().is_none()
                }
                (Unreachable, Local(_)) | (Local(_), Unreachable) => false,
                (Unreachable, Unreachable) => true,
            },
        )
    }
}

impl SemanticFlowGraph {
    pub(super) fn reaching_values_plain(
        &self,
        definitions: &[SemanticValueDefinition],
        targets: &BTreeSet<SemanticFlowPoint>,
        mut by_value: DefinitionsByValue,
        next: Vec<usize>,
    ) -> SemanticReachingValues {
        let width = definitions.len();
        let index = self.index();
        let blocks = index.blocks();
        let mut target_points = vec![false; index.len()];
        let mut active = vec![false; blocks.order.len()];
        let mut ancestors = Vec::new();
        for &target in targets {
            let Some(point) = index.id(target) else {
                continue;
            };
            target_points[point] = true;
            if let Some(block) = blocks.owner(point) {
                if !std::mem::replace(&mut active[block], true) {
                    ancestors.push(block);
                }
            }
        }
        while let Some(block) = ancestors.pop() {
            for point in index.predecessors(blocks.points(block)[0]) {
                if let Some(previous) = blocks.owner(point) {
                    if !std::mem::replace(&mut active[previous], true) {
                        ancestors.push(previous);
                    }
                }
            }
        }

        // Each active block owns a disjoint contiguous rank interval. A
        // definition on an inactive or unknown point never reaches a target.
        let mut ranks = vec![BlockReaching::UNREACHABLE; index.len()];
        let mut starts = Vec::with_capacity(active.len() + 1);
        let mut rank = 0;
        for (block, &needed) in active.iter().enumerate() {
            starts.push(rank);
            if needed {
                for &point in blocks.points(block) {
                    ranks[point] = rank;
                    rank += 1;
                }
            }
        }
        starts.push(rank);
        let mut states = active
            .iter()
            .map(|&needed| needed.then(|| BlockState::new(width)))
            .collect::<Vec<_>>();
        let mut writes = vec![(0, 0); width];
        let mut next = next;
        for definition in definitions {
            let bit = next[definition.value];
            next[definition.value] += 1;
            by_value.ordinals[bit] = definition.ordinal;
            let position = index
                .id(definition.point)
                .map_or(BlockReaching::UNREACHABLE, |point| ranks[point]);
            writes[bit] = (position, bit);
        }

        // For one (block, value) pair, only the final simultaneous writes
        // generate outgoing bits and the value's full range is killed once.
        for value in by_value.offsets.windows(2) {
            let range = value[0]..value[1];
            let row = &mut writes[range.clone()];
            row.sort_unstable();
            let mut remaining: &[_] = row;
            while let Some(&(first, _)) = remaining.first() {
                if first == BlockReaching::UNREACHABLE {
                    break;
                }
                let block = starts.partition_point(|&start| start <= first) - 1;
                let end = remaining.partition_point(|&(position, _)| position < starts[block + 1]);
                let (local, rest) = remaining.split_at(end);
                remaining = rest;
                let state = states[block].as_mut().expect("active definition block");
                let last = local[end - 1].0;
                for &(_, bit) in local
                    .iter()
                    .rev()
                    .take_while(|&&(position, _)| position == last)
                {
                    state.generated.insert(bit);
                }
                if range.len() > 1 {
                    state
                        .killed
                        .get_or_insert_with(|| DefinitionSet::new(width))
                        .insert_range(range.clone());
                }
            }
        }
        let mut pending = blocks
            .order
            .iter()
            .copied()
            .filter(|&block| active[block])
            .collect::<VecDeque<_>>();
        let mut delta = DefinitionSet::new(width);
        while let Some(block) = pending.pop_front() {
            let state = states[block].as_mut().expect("active flow block");
            state.queued = false;
            std::mem::swap(&mut delta, &mut state.pending);
            if let Some(killed) = &state.killed {
                delta.remove_with(killed);
            }
            if state.first {
                delta.union_with(&state.generated);
                state.first = false;
            }
            let last = *blocks.points(block).last().expect("nonempty flow block");
            for &next in index.successors(last) {
                let successor = blocks.owner(next).expect("reachable successor");
                if let Some(state) = &mut states[successor] {
                    if state.incoming.union_delta(&delta, &mut state.pending) && !state.queued {
                        state.queued = true;
                        pending.push_back(successor);
                    }
                }
            }
            delta.clear();
        }

        // The same local history now serves every target in its block.
        let mut inputs = Vec::new();
        for (block, state) in states.into_iter().enumerate() {
            let Some(state) = state else { continue };
            if blocks
                .points(block)
                .iter()
                .any(|&point| target_points[point])
            {
                inputs.push(BlockInput {
                    start: starts[block],
                    incoming: state.incoming,
                });
            }
        }
        for (rank, requested) in ranks.iter_mut().zip(target_points) {
            if !requested {
                *rank = BlockReaching::UNREQUESTED;
            }
        }
        SemanticReachingValues(ReachingStorage::Blocks(BlockReaching {
            points: Arc::clone(&index.points),
            target_ranks: ranks,
            by_value,
            inputs,
            writes,
        }))
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/semantic_reaching_blocks.rs"]
mod tests;
