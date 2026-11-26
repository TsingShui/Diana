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

fn at(pc: u32, mut insn: diana_decomp::ir::InsnNode) -> diana_decomp::ir::InsnNode {
    insn.set_offset(pc);
    insn
}

#[test]
fn sparse_dispatch_and_overlapping_handlers_use_exact_instruction_offsets() {
    use diana_decomp::ir::{
        splitter::Splitter, ArgType, ExceptionHandler, InsnArg, InsnNode, InsnType,
    };
    let code = vec![
        at(
            0,
            InsnNode::switch(
                InsnArg::lit(1, ArgType::INT),
                vec![(1, 12), (2, 12), (3, 9)],
            ),
        ),
        at(3, InsnNode::new(InsnType::Invoke, 0)),
        at(6, InsnNode::nop()),
        at(8, InsnNode::new(InsnType::Invoke, 0)),
        at(10, InsnNode::return_void()),
        at(12, InsnNode::goto(8)),
        at(14, InsnNode::new(InsnType::MoveException, 0)),
        at(16, InsnNode::return_void()),
        at(18, InsnNode::new(InsnType::MoveException, 0)),
        at(20, InsnNode::return_void()),
    ];
    let handlers = [(7, 11, 18), (3, 9, 14), (3, 9, 14), (23, 22, 22)]
        .map(|(lo, hi, dest)| ExceptionHandler::new(lo, hi, dest, None));
    let cfg = Splitter::new("overlap")
        .instructions(code)
        .handlers(handlers.into())
        .build();
    let block = |pc| cfg.blocks_iter().find(|b| b.offset == pc).unwrap().id;
    let edges = |pc| {
        cfg.successors_with_kind(block(pc))
            .iter()
            .map(|&(target, kind)| (cfg.block(target).unwrap().offset, kind))
            .collect::<BTreeSet<_>>()
    };
    assert!(cfg.block(block(9)).unwrap().insns.is_empty());
    assert!(cfg.block(block(22)).unwrap().insns.is_empty());
    assert_eq!(
        cfg.block(block(0))
            .unwrap()
            .terminator()
            .unwrap()
            .payload
            .switch_default,
        Some(3)
    );
    assert_eq!(
        edges(0),
        [
            (12, EdgeKind::SwitchCase(1)),
            (12, EdgeKind::SwitchCase(2)),
            (9, EdgeKind::SwitchCase(3)),
            (3, EdgeKind::SwitchDefault)
        ]
        .into()
    );
    assert_eq!(
        edges(3),
        [(6, EdgeKind::Normal), (14, EdgeKind::Exception)].into()
    );
    let ordered_handlers: Vec<_> = cfg
        .successors_with_kind(block(8))
        .iter()
        .filter(|(_, kind)| *kind == EdgeKind::Exception)
        .map(|&(target, _)| cfg.block(target).unwrap().offset)
        .collect();
    assert_eq!(ordered_handlers, [18, 14]);
    assert_eq!(
        edges(8),
        [
            (10, EdgeKind::Normal),
            (14, EdgeKind::Exception),
            (18, EdgeKind::Exception)
        ]
        .into()
    );
}

#[test]
fn one_block_preserves_instruction_storage_and_binds_many_interleaved_results() {
    use diana_decomp::ir::{
        passes::{BindResults, Pass, PassResult},
        splitter::Splitter,
        ArgType, InsnNode, InsnType, RegisterArg,
    };
    let mut code = Vec::new();
    for n in 0..256 {
        code.push(at(6 * n, InsnNode::nop()));
        code.push(at(6 * n + 2, InsnNode::filled_new_array(0, Vec::new())));
        code.push(at(
            6 * n + 4,
            InsnNode::move_result(RegisterArg::new(n, ArgType::array(ArgType::INT))),
        ));
    }
    code.push(at(1536, InsnNode::return_void()));
    let buffer = code.as_ptr();
    let mut cfg = Splitter::new("results").instructions(code).build();
    let entry = cfg.entry;
    assert_eq!(cfg.block(entry).unwrap().insns.as_ptr(), buffer);
    assert_eq!(BindResults.run(&mut cfg).unwrap(), PassResult::Changed);
    let insns = &cfg.block(entry).unwrap().insns;
    assert_eq!(insns.len(), 513);
    for (n, pair) in insns[..512].as_chunks::<2>().0.iter().enumerate() {
        assert_eq!(
            (pair[0].insn_type, pair[0].offset),
            (InsnType::Nop, 6 * n as u32)
        );
        assert_eq!(
            (
                pair[1].insn_type,
                pair[1].offset,
                pair[1].result.as_ref().unwrap().reg_num
            ),
            (InsnType::FilledNewArray, 6 * n as u32 + 2, n as u32)
        );
    }
    assert_eq!(insns.last().unwrap().insn_type, InsnType::Return);
}

#[test]
fn reordered_blocks_report_the_first_stream_error_before_any_result_is_bound() {
    use diana_decomp::ir::{
        passes::{BindResults, Pass, ResultBindingError},
        ArgType, InsnNode, InsnType, RegisterArg,
    };
    let result = || InsnNode::move_result(RegisterArg::new(0, ArgType::INT));
    let id = BlockId::new;
    let mut cfg = CFG::new("reordered_results");
    cfg.add_block(Block::with_instructions(
        id(0),
        8,
        vec![at(8, result()), at(10, InsnNode::nop()), at(12, result())],
    ));
    cfg.add_block(Block::with_instructions(
        id(9),
        4,
        vec![at(4, InsnNode::filled_new_array(0, Vec::new()))],
    ));
    cfg.add_edge(id(9), id(0), EdgeKind::Normal);
    assert!(matches!(
        BindResults.run(&mut cfg),
        Err(ResultBindingError::InvalidProducer {
            offset: 12,
            instruction: InsnType::Nop
        })
    ));
    assert!(cfg.block(id(9)).unwrap().insns[0].result.is_none());
}

#[test]
fn synthetic_repeated_pc_retains_its_last_successor() {
    use diana_decomp::ir::{splitter::Splitter, InsnNode, InsnType};
    let mut header = InsnNode::new(InsnType::If, 0);
    header.payload.target = Some(4);
    let cfg = Splitter::new("repeated_pc")
        .instructions(vec![
            at(4, header),
            at(6, InsnNode::nop()),
            at(6, InsnNode::nop()),
        ])
        .build();
    let last = BlockId::new(1);
    assert_eq!(cfg.successors_with_kind(last), &[(last, EdgeKind::Normal)]);
}

#[test]
fn prune_on_a_fully_reachable_graph_keeps_only_physical_handler_entries_in_order() {
    use diana_decomp::ir::{ArgType, ExceptionHandler, Pass, PassResult, PruneUnreachable};

    let mut cfg = CFG::new("handler_entries");
    let mut synthetic = Block::synthetic(1u32);
    synthetic.offset = 7;
    for block in [
        Block::new(0u32),
        synthetic,
        Block::with_offset(2u32, 4),
        Block::with_offset(3u32, 16),
    ] {
        cfg.add_block(block);
    }
    cfg.entry = BlockId::new(0);
    cfg.add_edge(cfg.entry, BlockId::new(1), EdgeKind::Normal);
    cfg.add_edge(cfg.entry, BlockId::new(2), EdgeKind::Exception);
    cfg.add_edge(cfg.entry, BlockId::new(3), EdgeKind::Exception);
    let typed =
        |offset| ExceptionHandler::new(0, 4, offset, Some(ArgType::object("java/lang/Exception")));
    cfg.handlers = vec![
        typed(7),
        typed(16),
        typed(99),
        ExceptionHandler::new(0, 4, 4, None),
    ];

    assert_eq!(PruneUnreachable.run(&mut cfg).unwrap(), PassResult::Changed);
    assert_eq!(cfg.num_blocks(), 4);
    let kept: Vec<_> = cfg
        .handlers
        .iter()
        .map(|h| (h.handler, h.is_catch_all()))
        .collect();
    assert_eq!(kept, [(16, false), (4, true)]);
    assert_eq!(
        PruneUnreachable.run(&mut cfg).unwrap(),
        PassResult::Unchanged
    );
}
