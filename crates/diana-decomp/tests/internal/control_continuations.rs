use super::ControlContinuations;
use crate::ir::{Block, BlockId, EdgeKind, CFG};
use std::collections::BTreeSet;

#[test]
fn shared_synthetic_tails_preserve_the_first_cycle_entry_and_phi_boundaries() {
    let mut cfg = CFG::new("cycle_entries");
    for i in 0..10 {
        cfg.add_block(Block::synthetic(BlockId(i)));
    }
    for (from, to) in [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 4),
        (4, 2),
        (5, 3),
        (6, 5),
        (7, 1),
        (8, 8),
        (9, 8),
    ] {
        cfg.add_edge(BlockId(from), BlockId(to), EdgeKind::Normal);
    }
    for (boundaries, expected) in [
        (BTreeSet::new(), [2, 2, 2, 3, 4, 3, 3, 2, 8, 8]),
        ([BlockId(3)].into(), [3, 3, 3, 3, 3, 3, 3, 3, 8, 8]),
    ] {
        let flow = ControlContinuations::preserving(&cfg, &boundaries);
        for (i, target) in expected.into_iter().enumerate() {
            assert_eq!(flow.destination(BlockId(i as u32)), BlockId(target));
        }
    }
    cfg.add_edge(BlockId(6), BlockId(3), EdgeKind::Exception);
    cfg.add_edge(BlockId(1), BlockId(2), EdgeKind::False);
    let flow = ControlContinuations::preserving(&cfg, &BTreeSet::new());
    assert_eq!(flow.destination(BlockId(0)), BlockId(1));
    assert_eq!(flow.destination(BlockId(2)), BlockId(3));
    assert_eq!(flow.destination(BlockId(6)), BlockId(6));
}

#[test]
fn canonical_merge_counts_only_predecessors_with_physical_blocks() {
    use crate::ir::passes::{CanonicalizeCFG, Pass};
    use crate::ir::InsnNode;
    let mut cfg = CFG::new("physical_predecessors");
    cfg.add_block(Block::new(0u32));
    let mut tail = Block::new(1u32);
    tail.push(InsnNode::return_void());
    cfg.add_block(tail);
    cfg.add_edge(BlockId(0), BlockId(1), EdgeKind::Normal);
    cfg.add_edge(BlockId(55), BlockId(1), EdgeKind::False);
    CanonicalizeCFG.run(&mut cfg).unwrap();
    assert_eq!(cfg.blocks.keys().copied().collect::<Vec<_>>(), [BlockId(0)]);
    assert_eq!(cfg.block(BlockId(0)).unwrap().insns.len(), 1);
}
