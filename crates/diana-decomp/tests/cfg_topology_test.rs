use std::collections::BTreeSet;

use diana_decomp::ir::{analysis::DominatorTree, Block, BlockId, EdgeKind, CFG};

#[test]
fn parallel_edges_and_removed_graph_nodes_keep_their_predecessor_identities() {
    let ids = [0, 9, 31, 42, 105].map(BlockId::new);
    let [a, b, c, d, e] = ids;
    let mut cfg = CFG::new("updates");
    for &id in &ids {
        cfg.add_block(Block::new(id));
    }

    let check = |cfg: &CFG| {
        let all = cfg.predecessor_snapshot();
        let normal = cfg.normal_predecessor_snapshot();
        for &to in &ids {
            let mut expected: Vec<_> = ids
                .iter()
                .flat_map(|&from| {
                    cfg.successors_with_kind(from)
                        .iter()
                        .filter_map(move |&(target, kind)| (target == to).then_some((from, kind)))
                })
                .collect();
            expected.sort_unstable();
            expected.dedup();
            assert_eq!(cfg.incoming_edges(to), expected);
            let sources = |ordinary: bool| {
                expected
                    .iter()
                    .filter(|(_, kind)| !ordinary || *kind != EdgeKind::Exception)
                    .map(|&(from, _)| from)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
            };
            assert_eq!(cfg.get_predecessors(to), sources(false));
            assert_eq!(all.get(&to).cloned().unwrap_or_default(), sources(false));
            assert_eq!(normal.get(&to).cloned().unwrap_or_default(), sources(true));
        }
    };
    for (from, to, kind) in [
        (e, b, EdgeKind::Exception),
        (c, b, EdgeKind::True),
        (c, b, EdgeKind::False),
        (c, b, EdgeKind::True),
        (a, b, EdgeKind::Normal),
        (b, b, EdgeKind::Exception),
        (b, d, EdgeKind::Normal),
        (a, d, EdgeKind::Exception),
        (a, c, EdgeKind::SwitchCase(7)),
    ] {
        cfg.add_edge(from, to, kind);
        check(&cfg);
    }
    cfg.remove_edge_with_kind(c, b, EdgeKind::True);
    check(&cfg);
    cfg.remove_normal_edges_from(a);
    check(&cfg);
    cfg.remove_block_from_graph(b);
    check(&cfg);
    assert!(cfg.block(b).is_some());
    cfg.add_edge(d, e, EdgeKind::Exception);
    cfg.add_edge(e, d, EdgeKind::Normal);
    cfg.remove_exception_edges();
    check(&cfg);
    cfg.remove_all_edges_from(e);
    check(&cfg);
    cfg.add_edge(c, d, EdgeKind::False);
    cfg.add_edge(c, d, EdgeKind::True);
    cfg.remove_edge(c, d);
    check(&cfg);
    cfg.add_edge(e, c, EdgeKind::True);
    cfg.remove_block(c);
    check(&cfg);
}

#[test]
fn subtree_dominance_matches_reachability_with_one_vertex_removed() {
    // Every forward spine, shortcut, back edge and self-loop on three sparse IDs.
    let ids = [2, 17, 65].map(BlockId::new);
    for mask in 0..512u32 {
        let mut cfg = CFG::new("dominance");
        cfg.entry = ids[0];
        for &id in &ids {
            cfg.add_block(Block::new(id));
        }
        for (i, &from) in ids.iter().enumerate() {
            for (j, &to) in ids.iter().enumerate() {
                if mask & (1 << (i * 3 + j)) != 0 {
                    cfg.add_edge(from, to, EdgeKind::Normal);
                }
            }
        }
        if cfg.reachable().len() != ids.len() {
            continue;
        }
        let tree = DominatorTree::compute(&cfg).unwrap();
        for &dominator in &ids {
            let mut pending = vec![cfg.entry];
            let mut reachable = BTreeSet::new();
            while let Some(block) = pending.pop() {
                if block != dominator && reachable.insert(block) {
                    pending.extend(cfg.successors(block));
                }
            }
            for &node in &ids {
                assert_eq!(
                    tree.dominates(dominator, node),
                    !reachable.contains(&node),
                    "graph {mask}"
                );
            }
        }
        assert!(!tree.dominates(ids[0], BlockId::INVALID));
    }
}

#[test]
fn captured_coverage_matches_half_open_ranges_at_original_instruction_sites() {
    use diana_decomp::ir::{ExceptionHandler, InsnNode, InsnType};

    let mut cfg = CFG::new("coverage");
    for (id, offsets) in [
        (0, &[10, 25, 40][..]),
        (6, &[19, 20, 20, 4]),
        (9, &[]),
        (15, &[30, u32::MAX]),
    ] {
        let mut block = Block::new(id);
        for &offset in offsets {
            let mut insn = InsnNode::new(InsnType::Nop, 0);
            insn.offset = offset;
            block.insns.push(insn);
        }
        cfg.add_block(block);
    }
    for (start, end) in [(10, 20), (10, 20), (11, 30), (20, 21), (40, 40), (42, 40)] {
        cfg.handlers
            .push(ExceptionHandler::new(start, end, 0, None));
    }
    cfg.capture_exception_coverage();
    for block in cfg.blocks_iter() {
        assert!(cfg.has_exception_coverage(block.id));
        assert_eq!(
            *cfg.exception_coverage(block.id),
            cfg.exception_coverage_for(&block.insns)
        );
    }
}
