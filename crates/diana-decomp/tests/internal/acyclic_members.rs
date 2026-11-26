use super::*;
use crate::ir::{Block, InsnNode};

fn block_fact(id: u32) -> AcyclicStructureFact {
    AcyclicStructureFact::Block {
        block: BlockId::new(id),
        prefix_only: false,
    }
}

fn block_id(fact: &AcyclicStructureFact) -> u32 {
    let AcyclicStructureFact::Block { block, .. } = fact else {
        panic!("expected a block, got {fact:?}")
    };
    block.raw()
}

fn branch_ids(branch: &AcyclicStructureFact) -> Vec<u32> {
    let AcyclicStructureFact::Sequence(items) = branch else {
        panic!("expected a sequence, got {branch:?}")
    };
    items.iter().map(block_id).collect()
}

#[test]
fn long_dominating_chain_preserves_disjoint_arms_and_their_shared_continuation() {
    let mut cfg = CFG::new("long_diamond");
    let condition = BlockId::new(48);
    let last = BlockId::new(260);
    cfg.entry = BlockId::new(0);
    for id in (0..=48).chain([103, 102, 81, 80]).chain(260..=305) {
        let mut block = Block::new(id);
        if block.id == condition {
            block.push(InsnNode::new(InsnType::If, 0));
        } else if block.id == last {
            block.push(InsnNode::new(InsnType::Return, 0));
        }
        cfg.add_block(block);
    }
    for id in 0..48 {
        cfg.add_edge(BlockId::new(id), BlockId::new(id + 1), EdgeKind::Normal);
    }
    cfg.add_edge(condition, BlockId::new(103), EdgeKind::True);
    cfg.add_edge(condition, BlockId::new(81), EdgeKind::False);
    for (from, to) in [(103, 102), (102, 305), (81, 80), (80, 305)] {
        cfg.add_edge(BlockId::new(from), BlockId::new(to), EdgeKind::Normal);
    }
    for id in (261..=305).rev() {
        cfg.add_edge(BlockId::new(id), BlockId::new(id - 1), EdgeKind::Normal);
    }
    let facts = ReachingConditions::analyze(&cfg).unwrap();
    let structure = AcyclicStructureAnalysis::new(&cfg, &facts, [last].into())
        .analyze()
        .unwrap()
        .unwrap();
    let AcyclicStructureFact::Sequence(items) = structure else {
        panic!("root sequence")
    };
    assert_eq!(
        items.iter().take(49).map(block_id).collect::<Vec<_>>(),
        (0..=48).collect::<Vec<_>>()
    );
    let AcyclicStructureFact::If {
        condition: guard,
        when_true,
        when_false: Some(when_false),
    } = &items[49]
    else {
        panic!("factored diamond")
    };
    assert_eq!(guard, &BoolExpr::block(condition));
    assert_eq!(branch_ids(when_true), [103, 102]);
    assert_eq!(branch_ids(when_false), [81, 80]);
    assert_eq!(
        items.iter().skip(50).map(block_id).collect::<Vec<_>>(),
        (260..=305).rev().collect::<Vec<_>>()
    );
}

#[test]
fn concatenation_keeps_the_larger_owned_buffer_and_orders_both_sides() {
    let mut middle = VecDeque::with_capacity(16);
    middle.extend((2..8).map(block_fact));
    let original_leaf: *const _ = &middle[0];
    let sequence = AcyclicStructureFact::sequence(vec![
        AcyclicStructureFact::sequence(vec![block_fact(0), block_fact(1)]),
        AcyclicStructureFact::Sequence(middle),
        block_fact(8),
    ]);
    assert_eq!(branch_ids(&sequence), (0..9).collect::<Vec<_>>());
    let AcyclicStructureFact::Sequence(stored) = &sequence else {
        unreachable!()
    };
    assert!(std::ptr::eq(original_leaf, &stored[2]));
}

#[test]
fn terminal_condition_errors_keep_their_block_order_during_membership_transfer() {
    let mut cfg = CFG::new("terminal_members");
    cfg.entry = BlockId::new(0);
    cfg.add_block(Block::new(cfg.entry));
    let facts = ReachingConditions::analyze(&cfg).unwrap();
    let closed = [BlockId::new(19), BlockId::new(4)]
        .into_iter()
        .collect::<BTreeSet<_>>();
    let analysis = AcyclicStructureAnalysis::new(&cfg, &facts, closed.clone());
    let assumption = ConditionDomain {
        domain: facts.truth(),
    };
    let item = StructureItem {
        condition: assumption.clone(),
        blocks: closed,
        lexical_rank: 0,
        structure: block_fact(0),
    };
    let mut members = BTreeSet::new();
    assert!(
        matches!(analysis.finish(vec![item], &assumption, Some(&mut members)),
        Err(AcyclicStructureError::MissingCondition(block)) if block == BlockId::new(4))
    );
    assert!(members.is_empty());
}
