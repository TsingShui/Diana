//! Fact-driven canonicalization of ordinary CFG topology.

use std::collections::BTreeSet;

use crate::ir::{Block, BlockId, EdgeKind, InsnType, CFG};

use super::{Pass, PassResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalizationError {
    MissingBlock(BlockId),
    MissingTerminator(BlockId),
    NonReducing,
}

impl std::fmt::Display for CanonicalizationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBlock(block) => write!(formatter, "missing CFG block {block}"),
            Self::MissingTerminator(block) => write!(formatter, "missing terminator in {block}"),
            Self::NonReducing => formatter.write_str("canonicalization did not reduce the graph"),
        }
    }
}

impl std::error::Error for CanonicalizationError {}

#[derive(Debug, Default)]
pub struct CanonicalizeCFG;

impl Pass for CanonicalizeCFG {
    type Error = CanonicalizationError;

    fn name(&self) -> &'static str {
        "canonicalize_cfg"
    }

    fn run(&mut self, cfg: &mut CFG) -> Result<PassResult, Self::Error> {
        cfg.capture_exception_coverage();
        let mut changed = CanonicalEntry::normalize(cfg);
        changed |= CanonicalTerminator::normalize(cfg)?;
        loop {
            let edits = CanonicalFacts::analyze(cfg).independent_edits();
            if edits.is_empty() {
                break;
            }
            let before = cfg.num_blocks();
            for edit in edits {
                edit.apply(cfg)?;
            }
            if cfg.num_blocks() >= before {
                return Err(CanonicalizationError::NonReducing);
            }
            changed = true;
        }
        Ok(changed.into())
    }
}

struct CanonicalEntry;

impl CanonicalEntry {
    fn normalize(cfg: &mut CFG) -> bool {
        let original = cfg.entry;
        if cfg.predecessors_with_kind(original).is_empty() {
            return false;
        }
        let next = cfg
            .blocks
            .last_key_value()
            .map_or(0, |(&block, _)| block.raw().saturating_add(1));
        let entry = BlockId::new(next);
        cfg.add_block(Block::synthetic(entry));
        cfg.set_exception_coverage(entry, BTreeSet::new());
        cfg.add_edge(entry, original, EdgeKind::Normal);
        cfg.entry = entry;
        true
    }
}

struct CanonicalTerminator;

impl CanonicalTerminator {
    fn normalize(cfg: &mut CFG) -> Result<bool, CanonicalizationError> {
        let mut reductions = Vec::new();
        for (&block, body) in &cfg.blocks {
            if !body.terminator().is_some_and(|instruction| {
                matches!(instruction.insn_type, InsnType::If | InsnType::Switch)
            }) {
                continue;
            }
            let mut targets = cfg.normal_successors(block);
            if let Some(target) = targets.next() {
                if targets.all(|other| other == target) {
                    reductions.push((block, target));
                }
            }
        }
        for (block, target) in &reductions {
            let target_offset = cfg
                .block(*target)
                .ok_or(CanonicalizationError::MissingBlock(*target))?
                .offset;
            let body = cfg
                .block_mut(*block)
                .ok_or(CanonicalizationError::MissingBlock(*block))?;
            let terminal = body
                .insns
                .last_mut()
                .ok_or(CanonicalizationError::MissingTerminator(*block))?;
            let offset = terminal.offset;
            *terminal = crate::ir::InsnNode::goto(target_offset as i32);
            terminal.offset = offset;

            let exceptional = cfg
                .successors_with_kind(*block)
                .iter()
                .filter(|(_, kind)| *kind == EdgeKind::Exception)
                .copied()
                .collect::<Vec<_>>();
            cfg.remove_all_edges_from(*block);
            cfg.add_edge(*block, *target, EdgeKind::Normal);
            for (handler, kind) in exceptional {
                cfg.add_edge(*block, handler, kind);
            }
        }
        Ok(!reductions.is_empty())
    }
}

struct CanonicalFacts<'a> {
    cfg: &'a CFG,
    handler_entries: BTreeSet<BlockId>,
}

impl<'a> CanonicalFacts<'a> {
    fn analyze(cfg: &'a CFG) -> Self {
        let handler_offsets = cfg
            .handlers
            .iter()
            .map(|handler| handler.handler)
            .collect::<BTreeSet<_>>();
        let handler_entries = if handler_offsets.is_empty() {
            BTreeSet::new()
        } else {
            cfg.blocks
                .values()
                .filter(|block| !block.synthetic && handler_offsets.contains(&block.offset))
                .map(|block| block.id)
                .collect()
        };
        Self {
            cfg,
            handler_entries,
        }
    }

    // The CFG owns sorted, deduplicated reverse edges. An absent source has
    // no block in this pass and must not participate in a physical rewrite.
    fn incoming(&self, block: BlockId) -> impl Iterator<Item = (BlockId, EdgeKind)> + '_ {
        self.cfg
            .predecessors_with_kind(block)
            .iter()
            .copied()
            .filter(|(source, _)| self.cfg.blocks.contains_key(source))
    }

    fn independent_edits(&self) -> Vec<GraphEdit> {
        let mut selected = Vec::new();
        let mut touched = BTreeSet::new();
        for block in self.cfg.blocks.keys().copied() {
            let Some(edit) = self.merge(block).or_else(|| self.bypass(block)) else {
                continue;
            };
            if !edit.touched().any(|block| touched.contains(&block)) {
                touched.extend(edit.touched());
                selected.push(edit);
            }
        }
        selected
    }

    fn merge(&self, head: BlockId) -> Option<GraphEdit> {
        let [(tail, EdgeKind::Normal)] = self.cfg.successors_with_kind(head) else {
            return None;
        };
        let head_body = self.cfg.block(head)?;
        if head_body.terminator().is_some_and(|terminator| {
            terminator.insn_type.is_terminal()
                || matches!(terminator.insn_type, InsnType::If | InsnType::Switch)
        }) {
            return None;
        }
        let tail_body = self.cfg.block(*tail)?;
        if tail_body.terminator().is_some_and(|terminator| {
            matches!(terminator.insn_type, InsnType::If | InsnType::Switch)
        }) && head_body
            .insns
            .iter()
            .any(|instruction| instruction.can_throw())
        {
            return None;
        }
        if *tail == self.cfg.entry || self.handler_entries.contains(tail) {
            return None;
        }
        let mut incoming = self.incoming(*tail);
        if incoming.next() != Some((head, EdgeKind::Normal))
            || incoming.next().is_some()
            || self.cfg.exception_coverage(head) != self.cfg.exception_coverage(*tail)
        {
            return None;
        }
        let outgoing = self.cfg.successors_with_kind(*tail).to_vec();
        Some(GraphEdit::Merge {
            head,
            tail: *tail,
            outgoing,
        })
    }

    fn bypass(&self, block: BlockId) -> Option<GraphEdit> {
        if block == self.cfg.entry || self.handler_entries.contains(&block) {
            return None;
        }
        let body = self.cfg.block(block)?;
        if !matches!(body.insns.as_slice(), [instruction] if instruction.insn_type == InsnType::Goto)
        {
            return None;
        }
        let [(target, EdgeKind::Normal)] = self.cfg.successors_with_kind(block) else {
            return None;
        };
        if *target == block {
            return None;
        }
        let incoming: Vec<_> = self.incoming(block).collect();
        if incoming.is_empty() {
            return None;
        }
        let coverage = self.cfg.exception_coverage(block);
        if self.cfg.exception_coverage(*target) != coverage
            || incoming
                .iter()
                .any(|(source, _)| self.cfg.exception_coverage(*source) != coverage)
        {
            return None;
        }
        Some(GraphEdit::Bypass {
            block,
            target: *target,
            incoming,
        })
    }
}

enum GraphEdit {
    Merge {
        head: BlockId,
        tail: BlockId,
        outgoing: Vec<(BlockId, EdgeKind)>,
    },
    Bypass {
        block: BlockId,
        target: BlockId,
        incoming: Vec<(BlockId, EdgeKind)>,
    },
}

impl GraphEdit {
    fn touched(&self) -> impl Iterator<Item = BlockId> + '_ {
        let (first, second, edges) = match self {
            Self::Merge {
                head,
                tail,
                outgoing,
            } => (*head, *tail, outgoing),
            Self::Bypass {
                block,
                target,
                incoming,
            } => (*block, *target, incoming),
        };
        [first, second]
            .into_iter()
            .chain(edges.iter().map(|&(neighbour, _)| neighbour))
    }

    fn apply(self, cfg: &mut CFG) -> Result<(), CanonicalizationError> {
        match self {
            Self::Merge {
                head,
                tail,
                outgoing,
            } => {
                let tail_instructions = std::mem::take(
                    &mut cfg
                        .block_mut(tail)
                        .ok_or(CanonicalizationError::MissingBlock(tail))?
                        .insns,
                );
                let head_body = cfg
                    .block_mut(head)
                    .ok_or(CanonicalizationError::MissingBlock(head))?;
                if head_body
                    .terminator()
                    .is_some_and(|terminator| terminator.insn_type == InsnType::Goto)
                {
                    head_body.insns.pop();
                }
                head_body.insns.extend(tail_instructions);
                cfg.remove_all_edges_from(head);
                for (target, kind) in outgoing {
                    cfg.add_edge(head, target, kind);
                }
                cfg.remove_block(tail);
            }
            Self::Bypass {
                block,
                target,
                incoming,
            } => {
                for edges in incoming.chunk_by(|left, right| left.0 == right.0) {
                    let source = edges[0].0;
                    let outgoing = cfg.successors_with_kind(source).to_vec();
                    cfg.remove_all_edges_from(source);
                    for (successor, kind) in outgoing {
                        cfg.add_edge(
                            source,
                            if successor == block {
                                target
                            } else {
                                successor
                            },
                            kind,
                        );
                    }
                }
                cfg.remove_block(block);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Block, InsnNode};

    #[test]
    fn keeps_a_loop_header_with_multiple_predecessors() {
        let mut cfg = CFG::new("loop");
        let mut entry = Block::new(0u32);
        entry.push(InsnNode::goto(1));
        let mut header = Block::new(1u32);
        header.push(InsnNode::if_cmp(
            crate::ir::IfOp::Lt,
            crate::ir::InsnArg::reg(0, crate::ir::ArgType::INT),
            crate::ir::InsnArg::reg(1, crate::ir::ArgType::INT),
            3,
        ));
        let mut body = Block::new(2u32);
        body.push(InsnNode::goto(1));
        let mut exit = Block::new(3u32);
        exit.push(InsnNode::return_void());
        for block in [entry, header, body, exit] {
            cfg.add_block(block);
        }
        cfg.entry = BlockId::new(0);
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::True);
        cfg.add_edge(BlockId::new(1), BlockId::new(3), EdgeKind::False);
        cfg.add_edge(BlockId::new(2), BlockId::new(1), EdgeKind::Normal);

        CanonicalizeCFG.run(&mut cfg).unwrap();

        assert!(cfg.block(BlockId::new(1)).is_some());
        assert_eq!(cfg.get_predecessors(BlockId::new(1)).len(), 2);
    }

    #[test]
    fn gives_a_looping_method_entry_an_explicit_preheader() {
        let mut cfg = CFG::new("entry_loop");
        let mut entry = Block::new(0u32);
        entry.push(InsnNode::if_cmp(
            crate::ir::IfOp::Eq,
            crate::ir::InsnArg::reg(0, crate::ir::ArgType::INT),
            crate::ir::InsnArg::lit(0, crate::ir::ArgType::INT),
            2,
        ));
        let mut latch = Block::new(1u32);
        latch.push(InsnNode::goto(0));
        let mut exit = Block::new(2u32);
        exit.push(InsnNode::return_void());
        for block in [entry, latch, exit] {
            cfg.add_block(block);
        }
        cfg.entry = BlockId::new(0);
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::False);
        cfg.add_edge(BlockId::new(0), BlockId::new(2), EdgeKind::True);
        cfg.add_edge(BlockId::new(1), BlockId::new(0), EdgeKind::Normal);

        CanonicalizeCFG.run(&mut cfg).unwrap();

        assert_ne!(cfg.entry, BlockId::new(0));
        assert!(cfg.block(cfg.entry).is_some_and(|block| block.synthetic));
        assert_eq!(
            cfg.successors_with_kind(cfg.entry),
            &[(BlockId::new(0), EdgeKind::Normal)]
        );
        assert_eq!(cfg.get_predecessors(cfg.entry), Vec::<BlockId>::new());
        assert_eq!(cfg.get_predecessors(BlockId::new(0)).len(), 2);
    }

    #[test]
    fn keeps_a_goto_that_marks_an_exception_coverage_boundary() {
        let mut cfg = CFG::new("exception_boundary");
        let mut source = Block::new(0u32);
        source.push(InsnNode::goto(1));
        let mut boundary = Block::new(1u32);
        boundary.push(InsnNode::goto(2));
        let mut target = Block::new(2u32);
        target.push(InsnNode::return_void());
        for block in [source, boundary, target] {
            cfg.add_block(block);
        }
        cfg.entry = BlockId::new(0);
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::Normal);
        cfg.capture_exception_coverage();
        let protected = BTreeSet::from([(10, 20)]);
        cfg.set_exception_coverage(BlockId::new(0), protected.clone());
        cfg.set_exception_coverage(BlockId::new(2), protected);

        CanonicalizeCFG.run(&mut cfg).unwrap();

        assert!(cfg.block(BlockId::new(1)).is_some());
    }
}
