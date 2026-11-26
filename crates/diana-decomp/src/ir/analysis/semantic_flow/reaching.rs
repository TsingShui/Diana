//! Reaching definitions on graphs without continuation-sensitive finally edges.

use super::{
    DefinitionSet, DefinitionsByValue, GraphIndex, SemanticFlowGraph, SemanticFlowPoint,
    SemanticReachingValues, SemanticValueDefinition,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

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

struct PointDefinitions {
    offsets: Vec<usize>,
    updates: Vec<(usize, usize)>,
}

impl PointDefinitions {
    fn new(
        index: &GraphIndex,
        definitions: &[SemanticValueDefinition],
        by_value: &mut DefinitionsByValue,
        mut next: Vec<usize>,
    ) -> Self {
        let mut offsets = vec![0; index.len() + 1];
        for definition in definitions {
            if let Some(point) = index.id(definition.point) {
                offsets[point + 1] += 1;
            }
        }
        for i in 1..offsets.len() {
            offsets[i] += offsets[i - 1];
        }
        let mut updates = vec![(0, 0); offsets[index.len()]];
        let mut cursor = offsets[..index.len()].to_vec();
        for definition in definitions {
            let value = definition.value;
            let bit = next[value];
            next[value] += 1;
            by_value.ordinals[bit] = definition.ordinal;
            if let Some(point) = index.id(definition.point) {
                updates[cursor[point]] = (value, bit);
                cursor[point] += 1;
            }
        }
        Self { offsets, updates }
    }

    fn at(&self, point: usize) -> &[(usize, usize)] {
        &self.updates[self.offsets[point]..self.offsets[point + 1]]
    }

    fn apply(&self, point: usize, by_value: &DefinitionsByValue, facts: &mut DefinitionSet) {
        let updates = self.at(point);
        // Definitions at one point are simultaneous. Clear all older
        // definitions before inserting any of these new ordinals.
        for &(value, _) in updates {
            let range = by_value.offsets[value]..by_value.offsets[value + 1];
            if range.len() > 1 {
                for (word, mask) in DefinitionSet::mask_words(range) {
                    facts.words_mut()[word] &= !mask;
                }
            }
        }
        for &(_, bit) in updates {
            facts.insert(bit);
        }
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
        let mut reaching = BTreeMap::new();
        for &target in targets {
            let Some(point) = index.id(target) else {
                continue;
            };
            target_points[point] = true;
            if let Some(block) = blocks.owner(point) {
                if !std::mem::replace(&mut active[block], true) {
                    ancestors.push(block);
                }
            } else {
                reaching.insert(target, DefinitionSet::new(width));
            }
        }
        while let Some(block) = ancestors.pop() {
            for &point in index.predecessors(blocks.points(block)[0]) {
                if let Some(previous) = blocks.owner(point) {
                    if !std::mem::replace(&mut active[previous], true) {
                        ancestors.push(previous);
                    }
                }
            }
        }

        let writes = PointDefinitions::new(index, definitions, &mut by_value, next);
        let mut states = active
            .iter()
            .map(|&needed| needed.then(|| BlockState::new(width)))
            .collect::<Vec<_>>();
        let mut pending = VecDeque::new();
        for &block in &blocks.order {
            let Some(state) = &mut states[block] else {
                continue;
            };
            for &point in blocks.points(block) {
                writes.apply(point, &by_value, &mut state.generated);
                for &(value, _) in writes.at(point) {
                    let range = by_value.offsets[value]..by_value.offsets[value + 1];
                    if range.len() > 1 {
                        state
                            .killed
                            .get_or_insert_with(|| DefinitionSet::new(width))
                            .insert_range(range);
                    }
                }
            }
            pending.push_back(block);
        }
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

        // Materialize the original occurrence-level facts in program order
        // once, after the block inputs reach their fixed point.
        for (block, state) in states.into_iter().enumerate() {
            let Some(state) = state else { continue };
            let mut facts = state.incoming;
            for &point in blocks.points(block) {
                writes.apply(point, &by_value, &mut facts);
                if target_points[point] {
                    reaching.insert(index.point(point), facts.clone());
                }
            }
        }
        SemanticReachingValues { reaching, by_value }
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/semantic_reaching_blocks.rs"]
mod tests;
