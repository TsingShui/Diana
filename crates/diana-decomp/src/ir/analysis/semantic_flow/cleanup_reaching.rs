//! Reaching definitions through matched finally entries and resumes.
use super::{
    index::FlowBlocks, DefinitionSet, DefinitionsByValue, GraphIndex, SemanticFlowEdgeKind,
    SemanticFlowPoint, SemanticReachingValues, SemanticValueDefinition,
};
use std::collections::VecDeque;

struct DefinitionMasks {
    generated: DefinitionSet,
    killed: DefinitionSet,
}

pub(super) fn analyze(
    index: &GraphIndex,
    definitions: &[SemanticValueDefinition],
    targets: &DefinitionSet,
    mut by_value: DefinitionsByValue,
    mut next: Vec<usize>,
) -> SemanticReachingValues {
    let width = definitions.len();
    let mut mask_ids = vec![None; index.len()];
    let mut masks = Vec::new();
    for definition in definitions {
        let value = definition.value;
        let bit = next[value];
        next[value] += 1;
        by_value.ordinals[bit] = definition.ordinal;
        let Some(point) = index.id(definition.point) else {
            continue;
        };
        let mask = *mask_ids[point].get_or_insert_with(|| {
            let id = masks.len();
            masks.push(DefinitionMasks {
                generated: DefinitionSet::new(width),
                killed: DefinitionSet::new(width),
            });
            id
        });
        masks[mask].generated.insert(bit);
        masks[mask]
            .killed
            .insert_range(by_value.offsets[value]..by_value.offsets[value + 1]);
    }
    let graph = CleanupGraph::new(index, targets);
    let local = graph
        .points
        .iter()
        .map(|&point| mask_ids[point].map(|id| &masks[id]))
        .collect::<Vec<_>>();
    let mut target_ids = vec![None; index.len()];
    let mut rows = Vec::new();
    for point in targets.members(0..index.len()) {
        target_ids[point] = Some(rows.len());
        rows.push((index.point(point), DefinitionSet::new(width)));
    }
    let output = graph
        .points
        .iter()
        .map(|&point| target_ids[point])
        .collect::<Vec<_>>();
    graph.propagate(width.div_ceil(64), &local, &output, &mut rows);
    SemanticReachingValues::dataflow(rows.into_iter().collect(), by_value)
}

struct CleanupGraph {
    points: Vec<usize>,
    successors: Vec<smallvec::SmallVec<[usize; 2]>>,
    blocks: FlowBlocks,
}

impl CleanupGraph {
    fn new(index: &GraphIndex, targets: &DefinitionSet) -> Self {
        let mut active = targets.clone();
        let mut outgoing =
            vec![smallvec::SmallVec::<[(usize, SemanticFlowEdgeKind); 1]>::new(); index.len()];
        let mut pending = targets.members(0..index.len()).collect::<Vec<_>>();
        while let Some(target) = pending.pop() {
            for &(previous, edge) in index.incoming_edges(target) {
                outgoing[previous].push((target, edge));
                if !active.contains(previous) {
                    active.insert(previous);
                    pending.push(previous);
                }
            }
        }

        let mut stacks = CleanupContinuations::default();
        let mut states = Vec::new();
        let mut state_ids = foldhash::HashMap::default();
        let mut successors = Vec::<smallvec::SmallVec<[usize; 2]>>::new();
        for &entry in &index.entries {
            let state = (entry, 0);
            if active.contains(entry) && !state_ids.contains_key(&state) {
                state_ids.insert(state, states.len());
                states.push(state);
            }
        }
        let entries = (0..states.len()).collect::<Vec<_>>();
        let mut cursor = 0;
        while cursor < states.len() {
            let (point, continuation) = states[cursor];
            let mut next_states = smallvec::SmallVec::new();
            for &(next, edge) in &outgoing[point] {
                let Some(stack) = stacks.transfer(continuation, edge) else {
                    continue;
                };
                let state = (next, stack);
                let id = *state_ids.entry(state).or_insert_with(|| {
                    let id = states.len();
                    states.push(state);
                    id
                });
                if !next_states.contains(&id) {
                    next_states.push(id);
                }
            }
            successors.push(next_states);
            cursor += 1;
        }

        let mut indegree = vec![0; states.len()];
        for &target in successors.iter().flatten() {
            indegree[target] += 1;
        }
        let blocks = FlowBlocks::from_edges(
            states.len(),
            &entries,
            |state| &successors[state],
            |state| indegree[state],
        );

        Self {
            points: states.into_iter().map(|(point, _)| point).collect(),
            successors,
            blocks,
        }
    }

    // A block transfers (input & !killed) | generated. Its straight-line
    // prefixes use the same rule to recover each requested point. Independent
    // words share the graph, and only targets need full-width output rows.
    fn propagate(
        &self,
        words: usize,
        local: &[Option<&DefinitionMasks>],
        output: &[Option<usize>],
        rows: &mut [(SemanticFlowPoint, DefinitionSet)],
    ) {
        let mut inputs = vec![0u64; self.blocks.order.len()];
        let mut changes = vec![0u64; self.blocks.order.len()];
        let mut kills = vec![0u64; self.blocks.order.len()];
        let mut work = VecDeque::new();
        for word in 0..words {
            inputs.fill(0);
            for &block in &self.blocks.order {
                let mut generated = 0;
                let mut killed = 0;
                for &state in self.blocks.points(block) {
                    if let Some(mask) = local[state] {
                        let removed = mask.killed.words()[word];
                        generated = (generated & !removed) | mask.generated.words()[word];
                        killed |= removed;
                    }
                }
                changes[block] = generated;
                kills[block] = killed;
                if generated != 0 {
                    work.push_back(block);
                }
            }
            while let Some(block) = work.pop_front() {
                let delta = std::mem::take(&mut changes[block]);
                let last = *self
                    .blocks
                    .points(block)
                    .last()
                    .expect("nonempty reaching block");
                for &state in &self.successors[last] {
                    let next = self.blocks.owner(state).expect("reachable successor");
                    let arrival = delta & !inputs[next];
                    inputs[next] |= arrival;
                    let added = arrival & !kills[next];
                    if added != 0 {
                        if changes[next] == 0 {
                            work.push_back(next);
                        }
                        changes[next] |= added;
                    }
                }
            }
            for &block in &self.blocks.order {
                let mut value = inputs[block];
                for &state in self.blocks.points(block) {
                    if let Some(mask) = local[state] {
                        value = (value & !mask.killed.words()[word]) | mask.generated.words()[word];
                    }
                    if let Some(row) = output[state] {
                        rows[row].1.words_mut()[word] |= value;
                    }
                }
            }
        }
    }
}

/// A finally continuation is a persistent stack. Ordinary edges carry its
/// index unchanged; every distinct push shares the same immutable tail.
#[derive(Default)]
struct CleanupContinuations {
    frames: Vec<(usize, u64)>,
    ids: foldhash::HashMap<(usize, u64), usize>,
}

impl CleanupContinuations {
    fn transfer(&mut self, stack: usize, edge: SemanticFlowEdgeKind) -> Option<usize> {
        match edge {
            SemanticFlowEdgeKind::Normal
            | SemanticFlowEdgeKind::Exceptional
            | SemanticFlowEdgeKind::LoopBack => Some(stack),
            SemanticFlowEdgeKind::FinallyEnter(token) => {
                if let Some(&id) = self.ids.get(&(stack, token)) {
                    return Some(id);
                }
                let mut cursor = stack;
                while cursor > 0 {
                    let (parent, top) = self.frames[cursor - 1];
                    if token == top {
                        return None;
                    }
                    cursor = parent;
                }
                self.frames.push((stack, token));
                let id = self.frames.len();
                self.ids.insert((stack, token), id);
                Some(id)
            }
            SemanticFlowEdgeKind::FinallyResume(token) => {
                let (parent, top) = self.frames[stack.checked_sub(1)?];
                (top == token).then_some(parent)
            }
            SemanticFlowEdgeKind::FinallyOverride => Some(self.frames[stack.checked_sub(1)?].0),
        }
    }
}
