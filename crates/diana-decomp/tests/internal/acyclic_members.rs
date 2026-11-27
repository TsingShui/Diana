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
        roots: smallvec![0],
        terminals: closed,
        lexical_rank: 0,
        structure: block_fact(0),
    };
    assert!(matches!(analysis.finish(vec![item], &assumption),
        Err(AcyclicStructureError::MissingCondition(block)) if block == BlockId::new(4)));
}

#[test]
fn suspended_branch_queries_preserve_eligibility_order_and_bdd_capacity() {
    let mut cfg = CFG::new("nested_then_serial_diamonds");
    cfg.entry = BlockId::new(0);
    let branches = [0, 1, 4, 7, 10, 13, 16, 19];
    for id in 0..=22 {
        let mut block = Block::new(id);
        if branches.contains(&id) {
            block.push(InsnNode::new(InsnType::If, 0));
        }
        cfg.add_block(block);
    }
    let id = BlockId::new;
    for (source, when_true, when_false) in [(0, 1, 4), (1, 2, 3), (4, 5, 6)] {
        cfg.add_edge(id(source), id(when_true), EdgeKind::True);
        cfg.add_edge(id(source), id(when_false), EdgeKind::False);
    }
    for source in [2, 3, 5, 6] {
        cfg.add_edge(id(source), id(7), EdgeKind::Normal);
    }
    for source in (7..=19).step_by(3) {
        cfg.add_edge(id(source), id(source + 1), EdgeKind::True);
        cfg.add_edge(id(source), id(source + 2), EdgeKind::False);
        cfg.add_edge(id(source + 1), id(source + 3), EdgeKind::Normal);
        cfg.add_edge(id(source + 2), id(source + 3), EdgeKind::Normal);
    }

    fn exercise(cfg: &CFG, independent: bool) -> usize {
        let facts = ReachingConditions::analyze(cfg).unwrap();
        let first = facts.compile(&BoolExpr::block(BlockId::new(0))).unwrap();
        let assumptions = [facts.truth(), first, facts.not(first).unwrap()];
        let mut queries = BranchQueries::default();
        for (scope, rank, wanted) in [
            (0, 0, 0),
            (1, 0, 1),
            (2, 0, 4),
            (1, 1, 7),
            (0, 1, 7),
            (2, 1, 7),
            (1, 0, 1),
            (1, 2, 10),
            (0, 2, 10),
            (2, 0, 4),
        ] {
            let assumption = assumptions[scope];
            let actual = if independent {
                let mut eligible = facts.branch_variables().iter().copied().filter(|&block| {
                    let owner = facts.condition(block).unwrap().domain();
                    if !facts.implies(assumption, owner).unwrap() {
                        return false;
                    }
                    let positive = facts.compile(&BoolExpr::block(block)).unwrap();
                    let negative = facts.not(positive).unwrap();
                    !facts.implies(assumption, positive).unwrap()
                        && !facts.implies(assumption, negative).unwrap()
                });
                eligible.nth(rank)
            } else {
                let variables = if scope == 0 {
                    &mut queries.root
                } else {
                    queries.assumed.entry(assumption).or_default()
                };
                variables
                    .candidate(rank, assumption, &facts, &mut queries.literals)
                    .unwrap()
                    .map(|(owner, _, _)| owner)
            };
            assert_eq!(actual, Some(BlockId::new(wanted)));
        }

        let mut domains = vec![facts.truth()];
        for &block in facts.branch_variables().iter().take(7) {
            let term = facts.compile(&BoolExpr::block(block)).unwrap();
            for i in 0..domains.len() {
                domains.push(facts.and(domains[i], term).unwrap());
            }
        }
        for (i, &premise) in domains.iter().enumerate() {
            for (j, &consequence) in domains.iter().enumerate() {
                if let Err(error) = facts.implies(premise, consequence) {
                    assert!(
                        matches!(
                            error,
                            ReachingConditionError::Bdd(crate::ir::bdd::BddError::ResourceLimit {
                                resource: crate::ir::bdd::BddResource::CacheEntries,
                                ..
                            })
                        ),
                        "{error:?}"
                    );
                    return i * domains.len() + j;
                }
            }
        }
        panic!("expected to exhaust the bounded implication cache");
    }
    assert_eq!(exercise(&cfg, false), exercise(&cfg, true));
}

#[test]
fn sibling_root_groups_preserve_paths_between_their_full_dominator_subtrees() {
    for seed in 0..12usize {
        let count = 73 + seed * 6;
        let blocks = (0..count)
            .map(|index| BlockId::new((((index * 37 + seed * 13) % 251) * 17) as u32))
            .collect::<Vec<_>>();
        let mut cfg = CFG::new("sparse_sibling_groups");
        cfg.entry = blocks[0];
        for (index, &id) in blocks.iter().enumerate() {
            let mut block = Block::new(id);
            let limit = count.min((index / 8 + 1) * 8 + 1);
            if index + 2 < limit && !(index + seed).is_multiple_of(4) {
                block.push(InsnNode::new(InsnType::If, 0));
                let target = index + 2 + (index * 11 + seed * 7) % (limit - index - 2);
                cfg.add_edge(id, blocks[target], EdgeKind::True);
                cfg.add_edge(id, blocks[index + 1], EdgeKind::False);
            } else if index + 1 < count {
                cfg.add_edge(id, blocks[index + 1], EdgeKind::Normal);
            }
            cfg.add_block(block);
        }
        let facts = ReachingConditions::analyze(&cfg).unwrap();
        let dom = DominatorTree::for_reaching_conditions(&cfg, &facts).unwrap();
        for &parent in &blocks {
            let children = dom.children(parent).collect::<Vec<_>>();
            if children.len() < 2 {
                continue;
            }
            for offset in 0..3 {
                let mut left = Vec::new();
                let mut right = Vec::new();
                for (index, &child) in children.iter().enumerate() {
                    match (index + offset) % 3 {
                        0 => left.push(child),
                        1 => right.push(child),
                        _ => {}
                    }
                }
                let mut queue = blocks
                    .iter()
                    .copied()
                    .filter(|&block| left.iter().any(|&child| dom.dominates(child, block)))
                    .collect::<Vec<_>>();
                let mut seen = BTreeSet::new();
                let mut reaches = false;
                while let Some(source) = queue.pop() {
                    if !seen.insert(source) {
                        continue;
                    }
                    for target in cfg.normal_successors(source) {
                        reaches |= right.iter().any(|&child| dom.dominates(child, target));
                        queue.push(target);
                    }
                }
                let ranks = |group: &[BlockId]| {
                    group
                        .iter()
                        .map(|&block| facts.rank(block).unwrap())
                        .collect::<Vec<_>>()
                };
                assert_eq!(
                    facts.any_rank_reaches(&ranks(&left), &ranks(&right)),
                    reaches
                );
            }
        }
    }
}
