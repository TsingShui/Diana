use super::{LiveSet, SsaLiveness, SsaVar};
use crate::ir::{
    ArgType, Block, BlockId, EdgeKind, InsnArg, InsnNode, InvokeType, RegisterArg, CFG,
};
use std::collections::BTreeSet;

#[test]
fn sparse_words_report_only_new_bits_and_drop_their_last_member() {
    let mut set = LiveSet::default();
    assert_eq!(set.add(0, 0b110), 0b110);
    assert_eq!(set.add(0, 0b101), 1);
    assert_eq!(set.add(1000, 0b1011), 0b1011);
    assert_eq!(set.add(1000, 0b1001), 0);
    set.insert(64);
    set.insert(127);
    assert_eq!(
        set.ids().collect::<Vec<_>>(),
        [0, 1, 2, 64, 127, 64000, 64001, 64003]
    );
    for id in [64, 127] {
        set.remove(id);
    }
    assert_eq!(set.words().collect::<Vec<_>>(), [(0, 7), (1000, 11)]);
    let mut copy = set.clone();
    copy.remove(64000);
    assert!(set.contains(64000));
    assert!(!copy.contains(64000));
}

#[test]
fn cyclic_live_ins_cross_machine_words_with_edge_specific_phi_arguments() {
    let ids = [0, 1, 2, 3, 4].map(BlockId::new);
    let [entry, header, body, exit, handler] = ids;
    let var = |n| SsaVar::new(n, 0);
    let arg = |n| InsnArg::reg_ssa(n, 0, ArgType::INT);
    let mut cfg = CFG::new("wide_cyclic_liveness");
    for id in ids {
        cfg.add_block(Block::new(id));
    }
    let mut phi = InsnNode::phi(
        RegisterArg::new_ssa(191, 0, ArgType::INT),
        vec![(entry.raw(), arg(63)), (body.raw(), arg(64))],
    );
    phi.payload.phi_edges[1].1 = EdgeKind::True;
    cfg.block_mut(header).unwrap().push(phi);
    cfg.block_mut(body).unwrap().push(InsnNode::const_value(
        RegisterArg::new_ssa(64, 0, ArgType::INT),
        5,
    ));
    let reads: Vec<_> = (0..192).filter(|n| n % 3 == 1 && *n != 64).collect();
    cfg.block_mut(exit).unwrap().push(InsnNode::invoke(
        InvokeType::Static,
        0,
        reads.iter().copied().map(arg).collect(),
    ));
    cfg.block_mut(handler)
        .unwrap()
        .push(InsnNode::return_value(arg(129)));
    for (from, to, kind) in [
        (entry, header, EdgeKind::Normal),
        (header, body, EdgeKind::Normal),
        (body, header, EdgeKind::True),
        (body, exit, EdgeKind::False),
        (body, handler, EdgeKind::Exception),
    ] {
        cfg.add_edge(from, to, kind);
    }
    let live = SsaLiveness::analyze(&cfg, &(0..192).map(var).collect()).unwrap();
    let edge = |from, to, kind| {
        live.edge_live(from, to, kind)
            .unwrap()
            .copied()
            .collect::<BTreeSet<_>>()
    };
    let common: BTreeSet<_> = reads.into_iter().map(var).collect();
    assert_eq!(edge(body, exit, EdgeKind::False), common);
    assert_eq!(edge(body, handler, EdgeKind::Exception), [var(129)].into());
    let mut loop_inputs = common;
    loop_inputs.insert(var(129));
    let mut entry_inputs = loop_inputs.clone();
    entry_inputs.insert(var(63));
    assert_eq!(edge(entry, header, EdgeKind::Normal), entry_inputs);
    loop_inputs.insert(var(64));
    assert_eq!(edge(body, header, EdgeKind::True), loop_inputs);
}

#[test]
fn transfers_keep_retained_operand_groups_and_skip_empty_instructions_across_blocks() {
    let [entry, exit] = [BlockId(0), BlockId(90)];
    let mut cfg = CFG::new("transfer_groups");
    let reg = |id| RegisterArg::new_ssa(id, 0, ArgType::INT);
    let arg = |id| InsnArg::reg_ssa(id, 0, ArgType::INT);
    let mut first = Block::new(entry);
    first.push(InsnNode::nop());
    first.push(InsnNode::invoke(
        InvokeType::Static,
        0,
        vec![arg(11), arg(9), arg(11), arg(18)],
    ));
    first.push(InsnNode::move_insn(reg(18), arg(11)));
    cfg.add_block(first);
    let mut second = Block::new(exit);
    second.push(InsnNode::const_value(reg(9), 1));
    second.push(InsnNode::nop());
    second.push(InsnNode::return_value(arg(18)));
    cfg.add_block(second);
    cfg.add_edge(entry, exit, EdgeKind::Normal);

    let retained = [11, 18].map(|id| SsaVar::new(id, 0));
    let live = SsaLiveness::analyze(&cfg, &retained.into()).unwrap();
    let transfers = |block| {
        live.block(block)
            .unwrap()
            .unwrap()
            .transfers
            .clone()
            .map(|i| {
                let transfer = &live.transfers[i];
                let start = i
                    .checked_sub(1)
                    .map_or(0, |prev| live.transfers[prev].uses_end);
                let uses: Vec<_> = live.operands[start..transfer.uses_end]
                    .iter()
                    .map(|&n| live.names[n].reg_num)
                    .collect();
                (
                    transfer.position,
                    transfer.definition.map(|n| live.names[n].reg_num),
                    uses,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        transfers(entry),
        [(1, None, vec![11, 18]), (2, Some(18), vec![11])]
    );
    assert_eq!(transfers(exit), [(2, None, vec![18])]);
}

#[test]
fn sparse_block_keys_keep_empty_edges_and_missing_target_precedence() {
    use super::SourceVariableError;
    let [low, high, missing_low, missing_high] = [11, 2_000, 39, 8_000].map(BlockId::new);
    let mut cfg = CFG::new("sparse_liveness_blocks");
    for id in [low, high] {
        cfg.add_block(Block::new(id));
    }
    // Physical liveness is keyed by the CFG map, even for mismatched body IDs.
    cfg.blocks.get_mut(&high).unwrap().id = low;
    cfg.add_edge(low, high, EdgeKind::Exception);
    let names = BTreeSet::from([SsaVar::new(1, 0)]);
    for retained in [&BTreeSet::new(), &names] {
        let live = SsaLiveness::analyze(&cfg, retained).unwrap();
        assert_eq!(
            live.edge_live(low, high, EdgeKind::Exception)
                .unwrap()
                .count(),
            0
        );
        assert!(matches!(live.edge_live(low, missing_low, EdgeKind::Normal),
            Err(SourceVariableError::MissingBlock(id)) if id == missing_low));
    }
    cfg.add_edge(low, missing_low, EdgeKind::Normal);
    cfg.add_edge(high, missing_high, EdgeKind::Exception);
    for retained in [&BTreeSet::new(), &names] {
        assert!(matches!(SsaLiveness::analyze(&cfg, retained),
            Err(SourceVariableError::MissingBlock(id)) if id == missing_high));
    }
}
