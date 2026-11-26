//! Register liveness used by pruned SSA construction.

use std::collections::BTreeMap;

use super::live_set::LiveSet;

use crate::ir::{BlockId, EdgeKind, InstructionTree, InstructionVisitor, RegisterArg, CFG};

#[derive(Debug, Clone)]
pub struct RegisterLiveness {
    live_in: BTreeMap<BlockId, LiveSet>,
    definition_blocks: BTreeMap<u32, Vec<BlockId>>,
}

#[derive(Default)]
struct BlockLiveness {
    definitions: LiveSet,
    // Only a definition first made at or after the throwing instruction
    // fails to kill the incoming register on an exception edge.
    late_definitions: Option<Box<LiveSet>>,
    live_in: LiveSet,
}

impl BlockLiveness {
    fn propagate(&mut self, word: usize, bits: u64, edge: EdgeKind) -> u64 {
        let mut killed = self.definitions.word(word);
        if edge.is_exception() {
            if let Some(late) = &self.late_definitions {
                killed &= !late.word(word);
            }
        }
        self.live_in.add(word, bits & !killed)
    }
}

impl InstructionVisitor for BlockLiveness {
    fn visit_register(&mut self, register: &RegisterArg) {
        let id = register.reg_num as usize;
        if !self.definitions.contains(id) {
            self.live_in.insert(id);
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
            let exceptional_boundary = cfg
                .successors_with_kind(block.id)
                .iter()
                .any(|(_, edge)| edge.is_exception())
                .then(|| block.insns.iter().rposition(|insn| insn.can_throw()))
                .flatten();
            for (index, instruction) in block.insns.iter().enumerate() {
                InstructionTree::visit_args(instruction, &mut state);
                if let Some(result) = &instruction.result {
                    let id = result.reg_num as usize;
                    let first = state.definitions.add(id / 64, 1 << (id % 64));
                    if first != 0 && exceptional_boundary.is_some_and(|boundary| index >= boundary)
                    {
                        state
                            .late_definitions
                            .get_or_insert_with(Default::default)
                            .insert(id);
                    }
                }
            }
            pending.extend(
                state
                    .live_in
                    .words()
                    .map(|(word, bits)| (block.id, word, bits)),
            );
            blocks.insert(block.id, state);
        }

        // Each newly-live (block, register) pair propagates once, including
        // through loops. A write blocks propagation only on the relevant edge.
        while let Some((target, word, bits)) = pending.pop() {
            for &(source, edge) in cfg.predecessors_with_kind(target) {
                let Some(state) = blocks.get_mut(&source) else {
                    continue;
                };
                let added = state.propagate(word, bits, edge);
                if added != 0 {
                    pending.push((source, word, added));
                }
            }
        }

        let mut live_in = BTreeMap::new();
        let mut definition_blocks = BTreeMap::<u32, Vec<BlockId>>::new();
        for (id, block) in blocks {
            for register in block.definitions.ids() {
                definition_blocks
                    .entry(register as u32)
                    .or_default()
                    .push(id);
            }
            if block.live_in.words().next().is_some() {
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
            .is_some_and(|values| values.contains(register as usize))
    }

    pub(crate) fn definitions(&self) -> impl Iterator<Item = (u32, &[BlockId])> {
        self.definition_blocks
            .iter()
            .map(|(&register, blocks)| (register, blocks.as_slice()))
    }

    pub fn definition_blocks(&self, register: u32) -> impl Iterator<Item = BlockId> + '_ {
        self.definition_blocks
            .get(&register)
            .into_iter()
            .flatten()
            .copied()
    }
}
