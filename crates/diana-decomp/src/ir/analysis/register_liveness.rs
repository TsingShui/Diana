//! Register liveness used by pruned SSA construction.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{BlockId, EdgeKind, InstructionTree, InstructionVisitor, RegisterArg, CFG};

#[derive(Debug, Clone)]
pub struct RegisterLiveness {
    live_in: BTreeMap<BlockId, BTreeSet<u32>>,
    definition_blocks: BTreeMap<u32, Vec<BlockId>>,
}

#[derive(Default)]
struct BlockLiveness {
    first_definitions: BTreeMap<u32, usize>,
    exceptional_boundary: Option<usize>,
    live_in: BTreeSet<u32>,
    pending: Vec<u32>,
}

impl BlockLiveness {
    fn observe(&mut self, register: u32) {
        if self.live_in.insert(register) {
            self.pending.push(register);
        }
    }

    fn kills(&self, register: u32, edge: EdgeKind) -> bool {
        self.first_definitions.get(&register).is_some_and(|&first| {
            !edge.is_exception()
                || self
                    .exceptional_boundary
                    .is_none_or(|boundary| first < boundary)
        })
    }
}

impl InstructionVisitor for BlockLiveness {
    fn visit_register(&mut self, register: &RegisterArg) {
        if !self.first_definitions.contains_key(&register.reg_num) {
            self.observe(register.reg_num);
        }
    }
}

impl RegisterLiveness {
    pub fn analyze(cfg: &CFG) -> Self {
        let mut blocks = BTreeMap::new();
        let mut pending = Vec::new();
        for block in cfg.blocks.values() {
            let mut state = BlockLiveness::default();
            // An exception edge sees only writes before its throwing instruction.
            if cfg
                .successors_with_kind(block.id)
                .iter()
                .any(|(_, edge)| edge.is_exception())
            {
                state.exceptional_boundary = block.insns.iter().rposition(|insn| insn.can_throw());
            }
            for (index, instruction) in block.insns.iter().enumerate() {
                InstructionTree::visit_args(instruction, &mut state);
                if let Some(result) = &instruction.result {
                    state
                        .first_definitions
                        .entry(result.reg_num)
                        .or_insert(index);
                }
            }
            if !state.pending.is_empty() {
                pending.push(block.id);
            }
            blocks.insert(block.id, state);
        }

        // Each newly-live (block, register) pair propagates once, including
        // through loops. A write blocks propagation only on the relevant edge.
        let mut delta = Vec::new();
        while let Some(target) = pending.pop() {
            std::mem::swap(
                &mut blocks.get_mut(&target).expect("queued block").pending,
                &mut delta,
            );
            for &(source, edge) in cfg.predecessors_with_kind(target) {
                let Some(state) = blocks.get_mut(&source) else {
                    continue;
                };
                let was_empty = state.pending.is_empty();
                for &register in &delta {
                    if !state.kills(register, edge) {
                        state.observe(register);
                    }
                }
                if was_empty && !state.pending.is_empty() {
                    pending.push(source);
                }
            }
            delta.clear();
        }

        let mut live_in = BTreeMap::new();
        let mut definition_blocks = BTreeMap::<u32, Vec<BlockId>>::new();
        for (id, block) in blocks {
            for register in block.first_definitions.into_keys() {
                definition_blocks.entry(register).or_default().push(id);
            }
            if !block.live_in.is_empty() {
                live_in.insert(id, block.live_in);
            }
        }
        Self {
            live_in,
            definition_blocks,
        }
    }

    pub fn is_live_in(&self, block: BlockId, register: u32) -> bool {
        self.live_in
            .get(&block)
            .is_some_and(|values| values.contains(&register))
    }

    pub fn definition_blocks(&self, register: u32) -> impl Iterator<Item = BlockId> + '_ {
        self.definition_blocks
            .get(&register)
            .into_iter()
            .flatten()
            .copied()
    }
}
