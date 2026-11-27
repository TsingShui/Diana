use super::{DominanceError, DominatorTree};
use crate::ir::analysis::ReachingConditions;
use crate::ir::{Block, BlockId, EdgeKind, CFG};

#[test]
fn ordinary_dag_dominance_matches_vertex_removal_in_physical_and_external_orders() {
    for ids in [[48, 2, 33, 95, 6], [2, 6, 33, 48, 95], [95, 48, 33, 6, 2]] {
        check_ordinary_dag(ids.map(BlockId::new));
    }
}

fn check_ordinary_dag(ids: [BlockId; 5]) {
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

#[test]
fn normal_postdominance_matches_exit_paths_after_vertex_removal() {
    use std::collections::BTreeSet;
    let ids = [8, 0, 41].map(BlockId);
    for mask in 0..512 {
        let mut cfg = CFG::new("postdominance");
        for &id in &ids {
            cfg.add_block(Block::new(id));
        }
        let mut edges = Vec::new();
        for (s, &id) in ids.iter().enumerate() {
            let mut terminal = true;
            for (t, &target) in ids.iter().enumerate() {
                if mask & (1 << (3 * s + t)) != 0 {
                    cfg.add_edge(id, target, EdgeKind::True);
                    cfg.add_edge(id, target, EdgeKind::False);
                    edges.push((id, target));
                    terminal = false;
                } else {
                    cfg.add_edge(id, target, EdgeKind::Exception);
                }
            }
            if terminal {
                edges.push((id, BlockId::INVALID));
            }
        }
        let reaches_exit = |start, avoid| {
            let mut seen = BTreeSet::new();
            let mut visit = vec![start];
            while let Some(block) = visit.pop() {
                if Some(block) == avoid || !seen.insert(block) {
                    continue;
                }
                if block == BlockId::INVALID {
                    return true;
                }
                visit.extend(
                    edges
                        .iter()
                        .filter(|(from, _)| *from == block)
                        .map(|(_, to)| *to),
                );
            }
            false
        };
        let tree = DominatorTree::compute_postdominance(&cfg).unwrap();
        for &id in &ids {
            assert_eq!(tree.contains(id), reaches_exit(id, None));
            for &dom in &ids {
                assert_eq!(
                    tree.dominates(dom, id),
                    reaches_exit(id, None) && !reaches_exit(id, Some(dom))
                );
            }
        }
    }
}

#[test]
fn physical_reference_to_the_reserved_exit_reports_its_inconsistent_virtual_edge() {
    let mut cfg = CFG::new("reserved");
    cfg.add_block(Block::new(BlockId(0)));
    cfg.add_edge(BlockId(0), BlockId::INVALID, EdgeKind::Normal);
    assert!(matches!(
        DominatorTree::compute_postdominance(&cfg),
        Err(DominanceError::InconsistentEdge {
            source: BlockId::INVALID,
            target: BlockId(0)
        })
    ));
    // A normal edge to an absent body still prevents the block being a physical exit.
    cfg.remove_all_edges_from(BlockId(0));
    cfg.add_edge(BlockId(0), BlockId(99), EdgeKind::Normal);
    assert!(!DominatorTree::compute_postdominance(&cfg)
        .unwrap()
        .contains(BlockId(0)));
}

#[test]
fn virtual_entry_preserves_all_normal_dominance_queries_without_copying_nodes() {
    let [entry, left, right, join] = [18, 2, 47, 9].map(BlockId);
    let mut cfg = CFG::new("virtual_entry");
    cfg.entry = entry;
    for block in [entry, left, right, join] {
        cfg.add_block(Block::new(block));
    }
    for (source, target) in [
        (entry, left),
        (entry, right),
        (left, join),
        (right, join),
        (join, left),
    ] {
        cfg.add_edge(source, target, EdgeKind::Normal);
    }
    let physical = DominatorTree::compute(&cfg).unwrap();
    let forest = physical.with_virtual_root().unwrap();
    let reference =
        DominatorTree::compute_normal(&cfg, cfg.block_ids(), &cfg.normal_predecessor_snapshot())
            .unwrap();
    assert!(std::sync::Arc::ptr_eq(&physical.nodes, &forest.nodes));
    assert_eq!(physical.entry(), entry);
    assert_eq!(physical.idom(entry), None);
    assert!(!physical.contains(BlockId::INVALID));
    assert_eq!(forest.entry(), reference.entry());
    assert_eq!(forest.len(), reference.len());
    assert_eq!(forest.preorder(), reference.preorder());
    assert_eq!(forest.postorder(), reference.postorder());
    let candidates = [entry, left, right, join, BlockId::INVALID, BlockId(100)];
    for block in candidates {
        assert_eq!(forest.contains(block), reference.contains(block));
        assert_eq!(forest.idom(block), reference.idom(block));
        assert_eq!(
            forest.children(block).collect::<Vec<_>>(),
            reference.children(block).collect::<Vec<_>>()
        );
        assert_eq!(forest.dominated_by(block), reference.dominated_by(block));
        for other in candidates {
            assert_eq!(
                forest.dominates(block, other),
                reference.dominates(block, other)
            );
        }
    }
}

#[test]
fn handler_forest_matches_vertex_removal_from_virtual_entries() {
    let [entry, mid, handler, exit] = [15, 2, 7, 41].map(BlockId);
    let ids = [entry, mid, handler, exit];
    for mask in 0..256 {
        let mut cfg = CFG::new("handler_forest");
        cfg.entry = entry;
        for id in ids {
            cfg.add_block(Block::new(id));
        }
        let choices = [
            (entry, mid),
            (entry, exit),
            (mid, mid),
            (mid, handler),
            (handler, exit),
            (exit, mid),
            (mid, entry),
            (exit, handler),
        ];
        let edges: Vec<_> = choices
            .into_iter()
            .enumerate()
            .filter_map(|(bit, pair)| ((1 << bit) & mask != 0).then_some(pair))
            .collect();
        for &(source, target) in &edges {
            cfg.add_edge(source, target, EdgeKind::Normal);
        }
        cfg.add_edge(entry, handler, EdgeKind::Exception);
        cfg.add_edge(mid, exit, EdgeKind::Exception);
        let reachable = |starts: &[BlockId], avoid| {
            let mut seen = std::collections::BTreeSet::new();
            let mut pending = starts.to_vec();
            while let Some(source) = pending.pop() {
                if Some(source) != avoid && seen.insert(source) {
                    pending.extend(
                        edges
                            .iter()
                            .filter(|(from, _)| *from == source)
                            .map(|(_, to)| *to),
                    );
                }
            }
            seen
        };
        let ordinary = reachable(&[entry], None);
        let roots: Vec<_> = [entry]
            .into_iter()
            .chain(
                [handler, exit]
                    .into_iter()
                    .filter(|id| !ordinary.contains(id)),
            )
            .collect();
        let full = reachable(&roots, None);
        let tree = DominatorTree::normal(&cfg);
        let supplied = DominatorTree::compute_normal(
            &cfg,
            cfg.block_ids(),
            &cfg.normal_predecessor_snapshot(),
        );
        if full.len() != ids.len() {
            let mut missing: Vec<_> = ids
                .iter()
                .copied()
                .filter(|id| !full.contains(id))
                .collect();
            missing.sort_unstable();
            assert_eq!(
                tree.unwrap_err(),
                DominanceError::Unreachable(missing.clone())
            );
            assert_eq!(supplied.unwrap_err(), DominanceError::Unreachable(missing));
            continue;
        }
        let tree = tree.unwrap();
        let supplied = supplied.unwrap();
        for id in ids {
            let reached = reachable(&roots, Some(id));
            for target in ids {
                assert_eq!(tree.dominates(id, target), !reached.contains(&target));
                assert_eq!(supplied.dominates(id, target), !reached.contains(&target));
            }
        }
    }
}

#[test]
fn cyclic_frontiers_match_predecessor_dominance_after_vertex_removal() {
    use crate::ir::analysis::DominanceFrontier;
    use std::collections::BTreeSet;

    let ids = [17, 1, 57].map(BlockId);
    for mask in 0u16..512 {
        let mut cfg = CFG::new("cyclic_frontiers");
        cfg.entry = ids[0];
        let mut edges = Vec::new();
        for (row, source) in ids.into_iter().enumerate() {
            cfg.add_block(Block::new(source));
            for (col, target) in ids.into_iter().enumerate() {
                if mask & (1 << (3 * row + col)) != 0 {
                    edges.push((source, target));
                    cfg.add_edge(source, target, EdgeKind::True);
                    cfg.add_edge(source, target, EdgeKind::Exception);
                }
            }
        }
        if cfg.reachable().len() != ids.len() {
            continue;
        }
        let tree = DominatorTree::compute(&cfg).unwrap();
        let frontiers = DominanceFrontier::compute(&cfg, &tree).unwrap();
        for removed in ids {
            let mut surviving = BTreeSet::new();
            let mut pending = vec![cfg.entry];
            while let Some(at) = pending.pop() {
                if at != removed && surviving.insert(at) {
                    pending.extend(
                        edges
                            .iter()
                            .filter_map(|&(from, to)| (from == at).then_some(to)),
                    );
                }
            }
            let expected = edges
                .iter()
                .filter_map(|&(source, target)| {
                    (!surviving.contains(&source)
                        && (removed == target || surviving.contains(&target)))
                    .then_some(target)
                })
                .collect::<BTreeSet<_>>();
            assert_eq!(
                frontiers.frontier(removed).collect::<BTreeSet<_>>(),
                expected,
                "graph {mask}"
            );
        }
    }
}

#[test]
fn virtual_root_has_stable_orders_and_checks_boundary_propagation() {
    use crate::ir::analysis::DominanceFrontier;
    let [entry, first, last] = [7, 2, 18].map(BlockId);
    let root = BlockId::INVALID;
    let mut cfg = CFG::new("virtual_frontiers");
    cfg.entry = entry;
    for block in [entry, first, last] {
        cfg.add_block(Block::new(block));
    }
    cfg.add_edge(entry, last, EdgeKind::Normal);
    cfg.add_edge(entry, first, EdgeKind::Normal);
    let tree = DominatorTree::compute(&cfg)
        .unwrap()
        .with_virtual_root()
        .unwrap();
    assert_eq!(tree.preorder(), [root, entry, first, last]);
    assert_eq!(tree.postorder(), [first, last, entry, root]);
    assert!(DominanceFrontier::compute(&cfg, &tree).unwrap().is_empty());

    // A back edge to the physical entry has its frontier capped by the virtual root.
    cfg.add_edge(first, entry, EdgeKind::Normal);
    let frontier = DominanceFrontier::compute(&cfg, &tree).unwrap();
    assert!(frontier.in_frontier(entry, entry));
    assert!(!frontier.in_frontier(entry, root));

    // A dangling edge can propagate into that absent root. A missing physical
    // child appears earlier during the tree walk and is reported first.
    cfg.add_edge(last, BlockId(80), EdgeKind::Normal);
    assert_eq!(
        DominanceFrontier::compute(&cfg, &tree).unwrap_err(),
        DominanceError::MissingTopology(root)
    );
    cfg.remove_block(first);
    assert_eq!(
        DominanceFrontier::compute(&cfg, &tree).unwrap_err(),
        DominanceError::MissingTopology(first)
    );
    cfg.add_block(Block::new(BlockId(1)));
    assert_eq!(
        DominanceFrontier::compute(&cfg, &tree).unwrap_err(),
        DominanceError::MissingTopology(BlockId(1))
    );
}
