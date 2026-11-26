use super::*;
use crate::ir::Block;

fn tree(ids: &[u32]) -> (CFG, RegionTree) {
    let mut cfg = CFG::new("region owners");
    let mut tree = RegionTree::new(ids.first().copied().map(BlockId::new));
    let root = tree.root();
    for &id in ids {
        let id = BlockId::new(id);
        cfg.add_block(Block::new(id));
        tree.add_block(root, id).unwrap();
    }
    (cfg, tree)
}

#[test]
fn root_interval_handles_the_last_block_id_and_excludes_its_predecessor() {
    let (cfg, tree) = tree(&[u32::MAX - 1, u32::MAX]);
    let owners = tree.verified_owners(&cfg).unwrap();
    assert_eq!(owners.get(BlockId::new(u32::MAX - 2)), None);
    assert_eq!(owners.get(BlockId::new(u32::MAX - 1)), Some(tree.root()));
    assert_eq!(owners.get(BlockId::new(u32::MAX)), Some(tree.root()));
}

#[test]
fn deepest_owner_overrides_only_actual_blocks_in_gapped_and_sparse_ranges() {
    for end in [12, u32::MAX] {
        let (cfg, mut tree) = tree(&[9, 10, end]);
        let child = tree
            .add_child(tree.root(), RegionKind::Try, Some(BlockId::new(end)))
            .unwrap();
        tree.add_block(child, BlockId::new(end)).unwrap();
        let owners = tree.verified_owners(&cfg).unwrap();
        for id in [8, 11] {
            assert_eq!(owners.get(BlockId::new(id)), None);
        }
        for id in [9, 10] {
            assert_eq!(owners.get(BlockId::new(id)), Some(tree.root()));
        }
        assert_eq!(owners.get(BlockId::new(end)), Some(child));
    }
}
