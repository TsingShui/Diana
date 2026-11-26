use super::{DominanceError, DominatorTree};
use crate::ir::analysis::ReachingConditions;
use crate::ir::{Block, BlockId, EdgeKind, CFG};

#[test]
fn ordinary_dag_dominance_matches_vertex_removal_with_sparse_unordered_ids() {
    let ids = [48, 2, 33, 95, 6].map(BlockId::new);
    // All forward edge subsets on five vertices, including cross-branch joins.
    for mask in 0..1024 {
        let mut cfg = CFG::new("dag_order");
        cfg.entry = ids[0];
        let mut bit = 0;
        for (position, &source) in ids.iter().enumerate() {
            cfg.add_block(Block::new(source));
            for &target in &ids[position + 1..] {
                if mask & (1 << bit) != 0 {
                    cfg.add_edge(source, target, EdgeKind::Normal);
                }
                bit += 1;
            }
        }
        if cfg.reachable().len() != ids.len() {
            continue;
        }
        let facts = ReachingConditions::analyze(&cfg).unwrap();
        let tree = DominatorTree::for_reaching_conditions(&cfg, &facts).unwrap();
        let reference = DominatorTree::compute(&cfg).unwrap();
        assert_eq!(tree.preorder(), reference.preorder());
        assert_eq!(tree.postorder(), reference.postorder());
        for &removed in &ids {
            let mut reached = std::collections::BTreeSet::new();
            let mut pending = vec![cfg.entry];
            while let Some(block) = pending.pop() {
                if block != removed && reached.insert(block) {
                    pending.extend(cfg.normal_successors(block));
                }
            }
            for &target in &ids {
                assert_eq!(
                    tree.dominates(removed, target),
                    !reached.contains(&target),
                    "graph {mask}"
                );
            }
        }
    }
}

#[test]
fn exceptional_arc_without_an_ordinary_counterpart_keeps_its_error() {
    let [entry, mid, exit] = [8, 4, 1].map(BlockId::new);
    let mut cfg = CFG::new("dag_exception");
    cfg.entry = entry;
    for block in [entry, mid, exit] {
        cfg.add_block(Block::new(block));
    }
    cfg.add_edge(entry, mid, EdgeKind::Normal);
    cfg.add_edge(mid, exit, EdgeKind::Normal);
    cfg.add_edge(entry, exit, EdgeKind::Exception);
    let facts = ReachingConditions::analyze(&cfg).unwrap();
    assert_eq!(
        DominatorTree::for_reaching_conditions(&cfg, &facts).unwrap_err(),
        DominanceError::InconsistentEdge {
            source: entry,
            target: exit
        }
    );
}
