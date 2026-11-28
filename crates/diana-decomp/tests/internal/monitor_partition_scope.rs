use super::*;
use crate::ir::{region::CatchRegion, Block};

#[test]
fn trimming_a_monitor_boundary_preserves_an_independent_try_with_a_shared_entry() {
    let blocks = [0, 1, 2, 3].map(BlockId::new);
    let mut cfg = CFG::new("monitor_partition_scope");
    for block in blocks {
        cfg.add_block(Block::synthetic(block));
    }
    let mut tree = RegionTree::new(Some(blocks[0]));
    let ordinary = tree
        .add_child(tree.root(), RegionKind::Try, Some(blocks[1]))
        .unwrap();
    let affected = tree
        .add_child(tree.root(), RegionKind::Try, Some(blocks[2]))
        .unwrap();
    for (owner, continuation) in [(ordinary, blocks[1]), (affected, blocks[3])] {
        tree.add_child(
            owner,
            RegionKind::Catch(CatchRegion {
                exception_types: Vec::new(),
                exception_value: None,
                continuation: Some(continuation),
            }),
            None,
        )
        .unwrap();
    }
    tree.add_block(ordinary, blocks[1]).unwrap();
    tree.add_block(affected, blocks[2]).unwrap();
    tree.add_block(affected, blocks[3]).unwrap();

    SynchronizationPlacement::strip_external_handler_continuations(
        &mut tree,
        &cfg,
        &BTreeSet::from([blocks[2]]),
    )
    .expect("trim one monitor boundary");

    let independent = tree.region(ordinary).unwrap();
    assert_eq!(independent.entry, Some(blocks[1]));
    assert_eq!(independent.blocks, BTreeSet::from([blocks[1]]));
    let trimmed = tree.region(affected).unwrap();
    assert_eq!(trimmed.entry, Some(blocks[2]));
    assert_eq!(trimmed.blocks, BTreeSet::from([blocks[2]]));
}
