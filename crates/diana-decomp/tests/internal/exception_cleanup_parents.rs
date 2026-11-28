use super::*;
use crate::ir::{region::CatchRegion, ArgType};

fn catch_all_cleanup() -> RegionKind {
    RegionKind::Cleanup(CatchRegion {
        exception_types: vec![ArgType::throwable()],
        exception_value: None,
        continuation: None,
    })
}

#[test]
fn normal_exits_leave_the_handler_without_reentering_its_protected_child() {
    let mut tree = RegionTree::new(Some(BlockId(0)));
    let root = tree.root();
    let enclosing = tree
        .add_child(root, RegionKind::Try, Some(BlockId(1)))
        .unwrap();
    let cleanup = tree
        .add_child(enclosing, catch_all_cleanup(), Some(BlockId(2)))
        .unwrap();
    let protected_in_cleanup = tree
        .add_child(cleanup, RegionKind::Try, Some(BlockId(3)))
        .unwrap();
    let finally = tree
        .add_child(root, RegionKind::Finally, Some(BlockId(4)))
        .unwrap();
    let handlers = BTreeMap::from([
        (enclosing, vec![finally]),
        (protected_in_cleanup, vec![cleanup]),
    ]);
    let chain = CleanupChainAnalysis::new(&tree, &handlers);

    assert_eq!(chain.between(cleanup, root).unwrap(), [finally]);
    assert_eq!(
        chain.between(protected_in_cleanup, root).unwrap(),
        [finally]
    );
    assert!(chain
        .between(protected_in_cleanup, cleanup)
        .unwrap()
        .is_empty());
}

#[test]
fn a_cross_handler_dispatch_cycle_cannot_loop_while_resolving_a_normal_exit() {
    let mut tree = RegionTree::new(Some(BlockId(0)));
    let root = tree.root();
    let left = tree
        .add_child(root, catch_all_cleanup(), Some(BlockId(1)))
        .unwrap();
    let right = tree
        .add_child(root, catch_all_cleanup(), Some(BlockId(2)))
        .unwrap();
    let left_body = tree
        .add_child(left, RegionKind::Try, Some(BlockId(3)))
        .unwrap();
    let right_body = tree
        .add_child(right, RegionKind::Try, Some(BlockId(4)))
        .unwrap();
    let handlers = BTreeMap::from([(left_body, vec![right]), (right_body, vec![left])]);
    let chain = CleanupChainAnalysis::new(&tree, &handlers);

    assert!(matches!(
        chain.between(left, root),
        Err(RegionInvariantError::RegionParentCycle(path))
            if path == [left, right_body, right, left_body, left]
    ));
}
