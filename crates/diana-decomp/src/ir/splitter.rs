//! Block Splitter
//!
//! Splits instruction list into basic blocks and builds CFG.

use smallvec::SmallVec;
use std::collections::HashMap;

use super::block::{Block, BlockId, ExceptionHandler};
use super::cfg::{EdgeKind, MethodContext, CFG};
use super::insn::{InsnNode, InsnType};

/// Block splitter - builds CFG from instruction list
pub struct Splitter {
    method: MethodContext,
    insns: Vec<InsnNode>,
    handlers: Vec<ExceptionHandler>,
    registers: u32,
    ins: u32,
}

impl Splitter {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            method: MethodContext::synthetic(name),
            insns: Vec::new(),
            handlers: Vec::new(),
            registers: 0,
            ins: 0,
        }
    }

    pub fn instructions(mut self, insns: Vec<InsnNode>) -> Self {
        self.insns = insns;
        self
    }

    pub fn handlers(mut self, handlers: Vec<ExceptionHandler>) -> Self {
        self.handlers = handlers;
        self
    }

    pub fn registers(mut self, count: u32) -> Self {
        self.registers = count;
        self
    }

    pub fn ins(mut self, count: u32) -> Self {
        self.ins = count;
        self
    }
    pub fn method(mut self, method: MethodContext) -> Self {
        self.method = method;
        self
    }
    /// Build CFG
    pub fn build(mut self) -> CFG {
        let mut cfg = CFG::with_method(self.method);
        cfg.registers = self.registers;
        cfg.ins = self.ins;
        if !self.insns.is_empty() {
            let (leaders, ordered) = Self::find_leaders(&mut self.insns, &self.handlers);
            Self::create_blocks(&mut cfg, self.insns, &leaders, ordered);
            Self::build_edges(&mut cfg, &leaders, ordered, &self.handlers);
        }
        cfg.handlers = self.handlers;
        cfg
    }

    fn find_leaders(
        insns: &mut [InsnNode],
        handlers: &[ExceptionHandler],
    ) -> (SmallVec<[u32; 8]>, bool) {
        let mut leaders = SmallVec::new();
        leaders.push(insns[0].offset);
        let mut boundaries = handlers
            .iter()
            .flat_map(|h| [h.start, h.end])
            .collect::<Vec<_>>();
        boundaries.sort_unstable();
        leaders.extend(handlers.iter().map(|h| h.handler));

        // The coverage test uses the union. Handler identities and DEX order
        // remain separate when building the exceptional edges.
        let mut ranges: Vec<_> = handlers
            .iter()
            .filter(|h| h.start < h.end)
            .map(|h| (h.start, h.end))
            .collect();
        ranges.sort_unstable();
        let mut merged = 0;
        for i in 0..ranges.len() {
            let (start, end) = ranges[i];
            if merged != 0 && start <= ranges[merged - 1].1 {
                ranges[merged - 1].1 = ranges[merged - 1].1.max(end);
            } else {
                ranges[merged] = (start, end);
                merged += 1;
            }
        }
        ranges.truncate(merged);
        let mut ordered = true;
        for i in 0..insns.len() {
            let next = insns.get(i + 1).map(|insn| insn.offset);
            let insn = &mut insns[i];
            ordered &= next.is_none_or(|pc| insn.offset < pc);
            if !boundaries.is_empty() && boundaries.binary_search(&insn.offset).is_ok() {
                leaders.push(insn.offset);
            }
            match insn.insn_type {
                InsnType::Goto | InsnType::If => {
                    leaders.push(insn.offset);
                    leaders.extend(insn.get_target().map(|target| target as u32));
                }
                InsnType::Switch => {
                    if insn.payload.switch_default.is_none() {
                        insn.payload.switch_default = next.and_then(|pc| i32::try_from(pc).ok());
                    }
                    leaders.extend(insn.payload.switch_default.map(|target| target as u32));
                    leaders.push(insn.offset);
                    if let Some(cases) = insn.get_switch_cases() {
                        leaders.extend(cases.iter().map(|&(_, target)| target as u32));
                    }
                }
                InsnType::Return | InsnType::Throw => {}
                _ => {
                    if ranges.is_empty() || !insn.can_throw() {
                        continue;
                    }
                    let range = ranges.partition_point(|&(_, end)| end <= insn.offset);
                    if !ranges
                        .get(range)
                        .is_some_and(|&(start, _)| start <= insn.offset)
                    {
                        continue;
                    }
                }
            }
            leaders.extend(next);
        }
        leaders.sort_unstable();
        leaders.dedup();
        (leaders, ordered)
    }

    fn create_blocks(cfg: &mut CFG, mut insns: Vec<InsnNode>, leaders: &[u32], ordered: bool) {
        if let [leader] = leaders {
            // A single block can own the original DEX instruction allocation.
            // u32::MAX is the exclusive end of the final interval.
            if !ordered || insns.last().is_some_and(|i| i.offset == u32::MAX) {
                if let Some(end) = insns.iter().position(|i| i.offset == u32::MAX) {
                    insns.truncate(end);
                }
            }
            cfg.add_block(Block::with_instructions(BlockId::new(0), *leader, insns));
            return;
        }
        let mut remaining = insns.into_iter();
        let mut entry = None;
        for (index, &leader) in leaders.iter().enumerate() {
            let end = leaders.get(index + 1).copied().unwrap_or(u32::MAX);
            let count = remaining
                .as_slice()
                .iter()
                .take_while(|insn| insn.offset < end)
                .count();
            let block = BlockId::new(index as u32);
            let instructions = remaining.by_ref().take(count).collect();
            cfg.add_block(Block::with_instructions(block, leader, instructions));
            if count != 0 && entry.is_none() {
                entry = Some(block);
            }
        }
        if let Some(entry) = entry {
            cfg.entry = entry;
        }
    }

    fn build_edges(cfg: &mut CFG, leaders: &[u32], ordered: bool, handlers: &[ExceptionHandler]) {
        let block_for = |offset| {
            leaders
                .binary_search(&offset)
                .ok()
                .map(|i| BlockId::new(i as u32))
        };
        // DEX offsets increase in instruction order. A block terminator's
        // fallthrough PC is the first instruction of the next nonempty block.
        // Synthetic streams with repeated PCs retain their last recorded successor.
        let next_offsets = (!ordered).then(|| {
            let mut offsets = cfg
                .blocks
                .values()
                .flat_map(|block| block.insns.iter().map(|i| i.offset));
            let mut next = HashMap::new();
            if let Some(mut previous) = offsets.next() {
                for offset in offsets {
                    next.insert(previous, offset);
                    previous = offset;
                }
            }
            next
        });
        // Collect edges first to avoid borrow issues
        let mut edges: Vec<(BlockId, BlockId, EdgeKind)> = Vec::new();

        let mut blocks = cfg
            .blocks
            .values()
            .filter(|block| !block.insns.is_empty())
            .peekable();
        while let Some(block) = blocks.next() {
            let Some(term) = block.terminator() else {
                continue;
            };
            let mut next_offset = || match &next_offsets {
                Some(offsets) => offsets.get(&term.offset).copied(),
                None => blocks.peek()?.insns.first().map(|i| i.offset),
            };
            let block_id = block.id;

            match term.insn_type {
                InsnType::Goto => {
                    if let Some(target) = term.get_target() {
                        // target is already absolute address
                        let abs = target as u32;
                        if let Some(to) = block_for(abs) {
                            edges.push((block_id, to, EdgeKind::Normal));
                        }
                    }
                }
                InsnType::If => {
                    if let Some(target) = term.get_target() {
                        // target is already absolute address
                        let abs = target as u32;
                        if let Some(to) = block_for(abs) {
                            edges.push((block_id, to, EdgeKind::True));
                        }
                    }
                    if let Some(next) = next_offset() {
                        if let Some(to) = block_for(next) {
                            edges.push((block_id, to, EdgeKind::False));
                        }
                    }
                }
                InsnType::Switch => {
                    if let Some(cases) = term.get_switch_cases() {
                        for &(value, target) in cases {
                            // target is already absolute address
                            let abs = target as u32;
                            if let Some(to) = block_for(abs) {
                                edges.push((block_id, to, EdgeKind::SwitchCase(value)));
                            }
                        }
                    }
                    if let Some(to) = term
                        .payload
                        .switch_default
                        .and_then(|target| block_for(target as u32))
                    {
                        edges.push((block_id, to, EdgeKind::SwitchDefault));
                    }
                }
                InsnType::Return | InsnType::Throw => {}
                _ => {
                    if let Some(next) = next_offset() {
                        if let Some(to) = block_for(next) {
                            edges.push((block_id, to, EdgeKind::Normal));
                        }
                    }
                }
            }
        }

        // Edge identity includes its kind. This preserves degenerate branches
        // whose true and false edges intentionally share one target.
        for (from, to, kind) in edges {
            cfg.add_edge(from, to, kind);
        }

        if !handlers.is_empty() {
            let mut throwing: Vec<_> = cfg
                .blocks
                .values()
                .flat_map(|block| {
                    block
                        .insns
                        .iter()
                        .filter(|insn| insn.can_throw())
                        .map(move |insn| (insn.offset, block.id))
                })
                .collect();
            if !ordered {
                throwing.sort_unstable();
            }
            for handler in handlers {
                let Some(target) = block_for(handler.handler) else {
                    continue;
                };
                let start = throwing.partition_point(|&(pc, _)| pc < handler.start);
                let mut previous = None;
                for &(pc, block) in &throwing[start..] {
                    if pc >= handler.end {
                        break;
                    }
                    if previous != Some(block) {
                        cfg.add_edge(block, target, EdgeKind::Exception);
                        previous = Some(block);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::block::ExceptionHandler as RawHandler;

    fn make_insn(offset: u32, ty: InsnType) -> InsnNode {
        let mut insn = InsnNode::new(ty, 0);
        insn.offset = offset;
        insn
    }

    #[test]
    fn test_linear() {
        let insns = vec![
            make_insn(0, InsnType::Const),
            make_insn(2, InsnType::Const),
            make_insn(4, InsnType::Return),
        ];
        let cfg = Splitter::new("test").instructions(insns).build();
        assert_eq!(cfg.num_blocks(), 1);
    }

    #[test]
    fn test_branch() {
        let mut if_insn = InsnNode::new(InsnType::If, 0);
        if_insn.offset = 0;
        if_insn.payload.target = Some(4); // jump to offset 4

        let insns = vec![
            if_insn,
            make_insn(2, InsnType::Return),
            make_insn(4, InsnType::Return),
        ];
        let cfg = Splitter::new("test").instructions(insns).build();

        // Leaders: 0, 2, 4
        assert_eq!(cfg.num_blocks(), 3);

        let succs: Vec<_> = cfg.successors(0u32).collect();
        assert_eq!(succs.len(), 2);
    }

    #[test]
    fn test_exception_edges() {
        // Test: try { invoke() } catch { ... }
        // Layout:
        //   0: invoke  <- can throw, in try region
        //   2: return
        //   4: move-exception <- handler entry
        //   6: return
        let insns = vec![
            make_insn(0, InsnType::Invoke), // can_throw = true
            make_insn(2, InsnType::Return),
            make_insn(4, InsnType::MoveException),
            make_insn(6, InsnType::Return),
        ];

        let handlers = vec![RawHandler::new(
            0,
            4,
            4,
            Some("Ljava/lang/Exception;".parse().unwrap()),
        )];

        let cfg = Splitter::new("test")
            .instructions(insns)
            .handlers(handlers)
            .build();

        // Should have multiple blocks:
        // - Block 0: offset 0 (invoke)
        // - Block 1: offset 2 (return after invoke - because invoke can throw)
        // - Block 2: offset 4 (handler entry)
        assert!(
            cfg.num_blocks() >= 3,
            "should have at least 3 blocks, got {}",
            cfg.num_blocks()
        );

        // Find the block containing invoke instruction
        let invoke_block = cfg
            .blocks
            .values()
            .find(|b| {
                b.insns
                    .iter()
                    .any(|i| matches!(i.insn_type, InsnType::Invoke))
            })
            .expect("should have invoke block");

        // Find the handler block
        let handler_block = cfg
            .blocks
            .values()
            .find(|b| b.offset == 4)
            .expect("should have handler block");

        // Check that there's an exception edge from invoke block to handler
        let has_exception_edge = cfg
            .successors_with_kind(invoke_block.id)
            .iter()
            .any(|(to, kind)| *to == handler_block.id && *kind == EdgeKind::Exception);

        assert!(
            has_exception_edge,
            "should have exception edge from invoke block to handler block"
        );
    }

    #[test]
    fn test_exception_edge_kind() {
        // Verify that exception edges are marked as EdgeKind::Exception
        let insns = vec![
            make_insn(0, InsnType::Invoke),
            make_insn(2, InsnType::Return),
            make_insn(4, InsnType::MoveException),
            make_insn(6, InsnType::Return),
        ];

        let handlers = vec![
            RawHandler::new(0, 4, 4, None), // catch-all
        ];

        let cfg = Splitter::new("test")
            .instructions(insns)
            .handlers(handlers)
            .build();

        // Count exception edges
        let exception_edge_count: usize = cfg
            .blocks
            .values()
            .flat_map(|b| cfg.successors_with_kind(b.id))
            .filter(|(_, kind)| *kind == EdgeKind::Exception)
            .count();

        assert!(
            exception_edge_count >= 1,
            "should have at least one exception edge"
        );
    }

    #[test]
    fn test_handler_is_leader() {
        // Handler entry should always be a leader
        let insns = vec![
            make_insn(0, InsnType::Const),
            make_insn(2, InsnType::Return),
            make_insn(4, InsnType::MoveException),
            make_insn(6, InsnType::Return),
        ];

        let handlers = vec![RawHandler::new(0, 2, 4, None)];

        let cfg = Splitter::new("test")
            .instructions(insns)
            .handlers(handlers)
            .build();

        // Handler at offset 4 should be its own block
        let handler_block = cfg.blocks.values().find(|b| b.offset == 4);

        assert!(
            handler_block.is_some(),
            "handler entry should be a leader and have its own block"
        );
    }

    #[test]
    fn test_try_boundaries_split_mid_block() {
        let insns = vec![
            make_insn(0, InsnType::Sget),
            make_insn(2, InsnType::MonitorEnter),
            make_insn(4, InsnType::Arith),
            make_insn(6, InsnType::Return),
            make_insn(8, InsnType::MoveException),
            make_insn(10, InsnType::Return),
        ];

        let handlers = vec![RawHandler::new(4, 6, 8, None)];

        let cfg = Splitter::new("mid_block_try")
            .instructions(insns)
            .handlers(handlers)
            .build();

        assert!(
            cfg.blocks.values().any(|block| block.offset == 0),
            "expected pre-try prefix block at offset 0"
        );
        assert!(
            cfg.blocks.values().any(|block| block.offset == 4),
            "expected try body block starting at protected offset 4"
        );

        let prefix_block = cfg
            .blocks
            .values()
            .find(|block| block.offset == 0)
            .expect("missing prefix block");
        assert!(
            prefix_block.insns.iter().all(|insn| insn.offset < 4),
            "prefix block should not contain protected instructions: {:?}",
            prefix_block.insns
        );

        let try_block = cfg
            .blocks
            .values()
            .find(|block| block.offset == 4)
            .expect("missing try block");
        assert!(
            try_block.insns.iter().any(|insn| insn.offset == 4),
            "try block should contain protected throwing instruction"
        );
        assert!(
            try_block.insns.iter().all(|insn| insn.offset >= 4),
            "try block should not contain pre-try instructions: {:?}",
            try_block.insns
        );
    }
}
