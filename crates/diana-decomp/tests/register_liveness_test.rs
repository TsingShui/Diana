use diana_decomp::ir::{
    analysis::RegisterLiveness, ArgType, ArithOp, Block, BlockId, EdgeKind, IfOp, InsnArg,
    InsnNode, RegisterArg, CFG,
};

#[test]
fn exception_backedge_observes_the_last_value_defined_before_the_throw() {
    let reg = |n| RegisterArg::new(n, ArgType::INT);
    let arg = |n| InsnArg::reg(n, ArgType::INT);
    let mut cfg = CFG::new("retry");
    cfg.add_block(Block::new(0));
    let length = InsnNode::array_length(reg(1), InsnArg::reg(2, ArgType::array(ArgType::INT)));
    cfg.add_block(Block::with_instructions(
        10,
        10,
        vec![InsnNode::const_value(reg(0), 3), length],
    ));
    cfg.add_block(Block::with_instructions(
        20,
        20,
        vec![InsnNode::if_cmp(
            IfOp::Eq,
            arg(1),
            InsnArg::lit(0, ArgType::INT),
            10,
        )],
    ));
    cfg.add_block(Block::with_instructions(
        30,
        30,
        vec![InsnNode::return_value(arg(1))],
    ));
    let sum = InsnNode::arith(ArithOp::Add, reg(8), arg(0), arg(7), ArgType::INT);
    cfg.add_block(Block::with_instructions(
        40,
        40,
        vec![InsnNode::return_value(InsnArg::wrap(sum))],
    ));
    for (from, to, edge) in [
        (0, 10, EdgeKind::Normal),
        (10, 20, EdgeKind::Exception),
        (10, 30, EdgeKind::Normal),
        (20, 10, EdgeKind::True),
        (20, 40, EdgeKind::False),
    ] {
        cfg.add_edge(BlockId::new(from), BlockId::new(to), edge);
    }

    let live = RegisterLiveness::analyze(&cfg);
    for (id, expected) in [
        (0, &[1, 2, 7][..]),
        (10, &[1, 2, 7]),
        (20, &[0, 1, 2, 7]),
        (30, &[1]),
        (40, &[0, 7]),
    ] {
        let actual: Vec<_> = (0..=8)
            .filter(|&r| live.is_live_in(BlockId::new(id), r))
            .collect();
        assert_eq!(actual, expected, "block {id}");
    }

    // An earlier write supplies the exceptional handler even if the same
    // register is also the result of the throwing instruction.
    cfg.block_mut(10)
        .unwrap()
        .insns
        .insert(1, InsnNode::const_value(reg(1), 11));
    let overwritten = RegisterLiveness::analyze(&cfg);
    assert!(!overwritten.is_live_in(BlockId::new(0), 1));
    assert!(!overwritten.is_live_in(BlockId::new(10), 1));
    assert!(overwritten.is_live_in(BlockId::new(20), 1));
    assert_eq!(
        overwritten.definition_blocks(1).collect::<Vec<_>>(),
        [BlockId::new(10)]
    );
}

#[test]
fn exceptional_and_normal_edges_preserve_distinct_low_and_sparse_registers() {
    let mut cfg = CFG::new("registerBounds");
    for (block, register) in [(10, 63), (20, 64), (30, u32::MAX)] {
        cfg.add_block(Block::with_instructions(
            block,
            block,
            vec![InsnNode::return_value(InsnArg::reg(register, ArgType::INT))],
        ));
    }
    cfg.add_block(Block::new(0));
    for (target, kind) in [
        (10, EdgeKind::Exception),
        (20, EdgeKind::True),
        (30, EdgeKind::False),
    ] {
        cfg.add_edge(BlockId::new(0), BlockId::new(target), kind);
    }
    let live = RegisterLiveness::analyze(&cfg);
    for (block, register) in [(10, 63), (20, 64), (30, u32::MAX)] {
        assert!(live.is_live_in(BlockId::new(0), register));
        for other in [62, 63, 64, 65, u32::MAX] {
            assert_eq!(
                live.is_live_in(BlockId::new(block), other),
                register == other
            );
        }
    }
}

#[test]
fn loop_backedges_propagate_word_and_sparse_registers_across_distinct_kill_sets() {
    // Normal and exceptional successors read disjoint registers. Only writes
    // before the throwing instruction reach both successors.
    let registers: Vec<u32> = (0..80).chain([u32::MAX - 3, u32::MAX - 2]).collect();
    let mut body = Vec::new();
    for &r in registers.iter().filter(|&&r| r % 3 == 0) {
        body.push(InsnNode::const_value(RegisterArg::new(r, ArgType::INT), 8));
    }
    body.push(InsnNode::array_length(
        RegisterArg::new(1000, ArgType::INT),
        InsnArg::reg(2000, ArgType::array(ArgType::INT)),
    ));
    for &r in registers.iter().filter(|&&r| r % 3 != 2) {
        body.push(InsnNode::const_value(RegisterArg::new(r, ArgType::INT), 12));
    }
    let mut cfg = CFG::new("mixedLiveness");
    cfg.add_block(Block::new(0));
    cfg.add_block(Block::with_instructions(1, 1, body));
    for (id, parity) in [(2, 0), (3, 1)] {
        let uses = registers
            .iter()
            .filter(|&&r| r % 2 == parity)
            .map(|&r| {
                InsnNode::arith(
                    ArithOp::Add,
                    RegisterArg::new(3000, ArgType::INT),
                    InsnArg::reg(r, ArgType::INT),
                    InsnArg::lit(1, ArgType::INT),
                    ArgType::INT,
                )
            })
            .collect();
        cfg.add_block(Block::with_instructions(id, id, uses));
    }
    for (from, to, edge) in [
        (0, 1, EdgeKind::Normal),
        (1, 2, EdgeKind::Normal),
        (1, 3, EdgeKind::Exception),
        (2, 1, EdgeKind::Normal),
        (3, 1, EdgeKind::Normal),
    ] {
        cfg.add_edge(BlockId(from), BlockId(to), edge);
    }
    let live = RegisterLiveness::analyze(&cfg);
    for r in registers {
        let at_head = r % 3 == 2 || (r % 3 == 1 && r % 2 == 1);
        assert_eq!(live.is_live_in(BlockId(0), r), at_head);
        assert_eq!(live.is_live_in(BlockId(1), r), at_head);
        assert_eq!(live.is_live_in(BlockId(2), r), at_head || r % 2 == 0);
        assert_eq!(live.is_live_in(BlockId(3), r), at_head || r % 2 == 1);
        assert_eq!(
            live.definition_blocks(r).collect::<Vec<_>>(),
            if r % 3 == 2 { vec![] } else { vec![BlockId(1)] }
        );
    }
}

#[test]
fn throwing_result_seeds_the_handler_phi_and_its_normal_join() {
    use diana_decomp::ir::passes::SSATransform;
    use diana_decomp::ir::{InsnType, Pass};

    let reg = |n| RegisterArg::new(n, ArgType::INT);
    let mut cfg = CFG::new("exceptionalPhiSeed");
    cfg.add_block(Block::with_instructions(
        0,
        0,
        vec![
            InsnNode::const_value(reg(65), 12),
            InsnNode::const_value(RegisterArg::new(2, ArgType::array(ArgType::INT)), 0),
        ],
    ));
    cfg.add_block(Block::with_instructions(
        1,
        1,
        vec![InsnNode::array_length(
            reg(65),
            InsnArg::reg(2, ArgType::array(ArgType::INT)),
        )],
    ));
    cfg.add_block(Block::new(2));
    cfg.add_block(Block::new(3));
    cfg.add_block(Block::with_instructions(
        4,
        4,
        vec![InsnNode::return_value(InsnArg::reg(65, ArgType::INT))],
    ));
    for (source, target, kind) in [
        (0, 1, EdgeKind::Normal),
        (1, 2, EdgeKind::Normal),
        (1, 3, EdgeKind::Exception),
        (2, 4, EdgeKind::Normal),
        (3, 4, EdgeKind::Normal),
    ] {
        cfg.add_edge(BlockId(source), BlockId(target), kind);
    }
    SSATransform.run(&mut cfg).unwrap();
    let version = |id| {
        cfg.block(BlockId(id)).unwrap().insns[0]
            .result
            .as_ref()
            .unwrap()
            .ssa_version
            .unwrap()
    };
    assert_ne!(version(0), version(1));
    let handler = &cfg.block(BlockId(3)).unwrap().insns[0];
    assert_eq!(handler.insn_type, InsnType::Phi);
    assert_eq!(
        handler.payload.phi_edges,
        [(BlockId(1), EdgeKind::Exception)]
    );
    assert_eq!(
        handler.args[0].as_register().unwrap().ssa_version,
        Some(version(0))
    );
    let join = &cfg.block(BlockId(4)).unwrap().insns[0];
    assert_eq!(join.insn_type, InsnType::Phi);
    let pairs: Vec<_> = join
        .payload
        .phi_edges
        .iter()
        .copied()
        .zip(
            join.args
                .iter()
                .map(|arg| arg.as_register().unwrap().ssa_version.unwrap()),
        )
        .collect();
    assert_eq!(
        pairs,
        [
            ((BlockId(2), EdgeKind::Normal), version(1)),
            ((BlockId(3), EdgeKind::Normal), version(3))
        ]
    );
}

#[test]
fn invalid_throwing_boundaries_preserve_each_instruction_position() {
    use diana_decomp::ir::passes::{SSATransform, SsaTransformError};
    use diana_decomp::ir::Pass;
    for sites in [vec![], vec![0], vec![0, 1, 3]] {
        let mut cfg = CFG::new("invalidExceptionPoint");
        let mut source = Block::new(0);
        for index in 0..4 {
            let reg = RegisterArg::new(0, ArgType::INT);
            source.push(if sites.contains(&index) {
                InsnNode::array_length(reg, InsnArg::reg(1, ArgType::array(ArgType::INT)))
            } else {
                InsnNode::const_value(reg, 0)
            });
        }
        cfg.add_block(source);
        cfg.add_block(Block::new(1));
        cfg.add_edge(BlockId(0), BlockId(1), EdgeKind::Exception);
        assert!(matches!(
            SSATransform.run(&mut cfg),
            Err(SsaTransformError::InvalidExceptionBoundary { block: BlockId(0), throwing }) if throwing == sites
        ));
    }
}

#[test]
fn duplicate_body_id_keeps_earlier_seed_uses_and_the_last_body_definitions() {
    let mut cfg = CFG::new("declaredBodyIds");
    cfg.add_block(Block::new(0));
    let make_use = |reg| InsnNode::return_value(InsnArg::reg(reg, ArgType::INT));
    cfg.blocks.insert(
        BlockId(1),
        Block::with_instructions(9, 0, vec![make_use(2)]),
    );
    cfg.blocks.insert(
        BlockId(2),
        Block::with_instructions(
            9,
            0,
            vec![
                InsnNode::const_value(RegisterArg::new(3, ArgType::INT), 7),
                make_use(65),
            ],
        ),
    );
    cfg.add_edge(BlockId(0), BlockId(9), EdgeKind::Normal);
    let live = RegisterLiveness::analyze(&cfg);
    assert!(live.is_live_in(BlockId(0), 2));
    assert!(live.is_live_in(BlockId(0), 65));
    assert!(!live.is_live_in(BlockId(9), 2));
    assert!(live.is_live_in(BlockId(9), 65));
    assert_eq!(live.definition_blocks(3).collect::<Vec<_>>(), [BlockId(9)]);
}
