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
    queued_low: u64,
    // Only a definition first made at or after the throwing instruction
    // fails to kill the incoming register on an exception edge.
    late_definitions: Option<Box<LiveSet>>,
    live_in: LiveSet,
}

impl BlockLiveness {
    // Many successors can discover different registers in the same low word
    // while the predecessor is still queued. One visit propagates their union.
    fn enqueue(&mut self, word: usize, bits: u64) -> bool {
        if word != 0 {
            return true;
        }
        let first = self.queued_low == 0;
        self.queued_low |= bits;
        first
    }

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
        let mut blocks = Vec::with_capacity(cfg.blocks.len());
        let mut seeds = Vec::new();
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
            seeds.extend(
                state
                    .live_in
                    .words()
                    .map(|(word, bits)| (block.id, word, bits)),
            );
            blocks.push((block.id, state));
        }

        // Physical blocks arrive in identity order. An explicitly supplied CFG
        // can use different keys for the same body ID. Its last body supplies
        // the definitions and every body's initial uses seed the flow.
        if !blocks.windows(2).all(|pair| pair[0].0 < pair[1].0) {
            blocks = blocks
                .into_iter()
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect();
        }
        let mut pending = seeds
            .into_iter()
            .filter_map(|(block, word, bits)| {
                let index = Self::position(&blocks, block)?;
                blocks[index]
                    .1
                    .enqueue(word, bits)
                    .then_some((index, word, bits))
            })
            .collect::<Vec<_>>();

        // Each newly-live bit propagates once. Sparse register words keep
        // their own deltas without reserving a word array per block.
        while let Some((target, word, bits)) = pending.pop() {
            let (block, state) = &mut blocks[target];
            let bits = if word == 0 {
                std::mem::take(&mut state.queued_low)
            } else {
                bits
            };
            for &(source, edge) in cfg.predecessors_with_kind(*block) {
                let Some(index) = Self::position(&blocks, source) else {
                    continue;
                };
                let state = &mut blocks[index].1;
                let added = state.propagate(word, bits, edge);
                if added != 0 && state.enqueue(word, added) {
                    pending.push((index, word, added));
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

    fn position(blocks: &[(BlockId, BlockLiveness)], id: BlockId) -> Option<usize> {
        let index = id.raw() as usize;
        if blocks.get(index).is_some_and(|row| row.0 == id) {
            Some(index)
        } else {
            blocks.binary_search_by_key(&id, |row| row.0).ok()
        }
    }

    pub fn is_live_in(&self, block: BlockId, register: u32) -> bool {
        self.live_in
            .get(&block)
            .is_some_and(|values| values.contains(register as usize))
    }

    /// Every register has one entry per definition block, in block identity order.
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
