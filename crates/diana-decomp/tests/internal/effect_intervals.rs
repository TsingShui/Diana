use super::{EffectStability, EffectSummary, EffectVersions, ValueFlowGraph};
use crate::ir::analysis::SemanticFlowPoint;
use crate::ir::{
    ArgType, BlockId, InsnArg, InsnNode, InsnType, InstructionId, RegionId, SemanticBlock,
    SemanticLoopControl, SemanticLoopKind, SemanticLoopTest, SemanticNode, SemanticOperand,
    SemanticOperation, SemanticPredicate, SemanticSiteId, SemanticStatement,
};
use std::collections::BTreeSet;

fn block(id: u32, sites: std::ops::Range<u64>) -> SemanticNode {
    let statements = sites
        .map(|site| {
            let mut store = InsnNode::new(InsnType::Sput, 1);
            store.id = InstructionId::new(site as usize);
            store.args.push(InsnArg::lit(8, ArgType::INT));
            let mut statement = SemanticStatement::instruction(store).unwrap();
            statement.site = Some(SemanticSiteId(site));
            statement
        })
        .collect();
    SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(id),
        statements,
    })
}

fn predicate(site: u64) -> SemanticOperand<SemanticPredicate> {
    let mut test = InsnNode::new(InsnType::If, 0);
    test.id = InstructionId::new(site as usize);
    let mut operand = SemanticOperand::new(SemanticPredicate::Test(
        SemanticOperation::from_instruction(test).unwrap(),
    ));
    operand.site = Some(SemanticSiteId(site));
    operand
}

fn before(site: u64) -> SemanticFlowPoint {
    SemanticFlowPoint::before(SemanticSiteId(site))
}
fn after(site: u64) -> SemanticFlowPoint {
    SemanticFlowPoint::after(SemanticSiteId(site))
}

#[test]
fn adjacent_effect_queries_examine_one_candidate_per_interval() {
    const COUNT: u64 = 256;
    let root = block(0, 0..COUNT);
    let graph = ValueFlowGraph::build_source(&root, &BTreeSet::new()).unwrap();
    let versions = EffectVersions::new(&graph);
    let effect = EffectSummary::synchronization();
    let domain = graph.logic.truth();
    for i in 0..COUNT - 1 {
        assert_eq!(
            versions.stability(&effect, domain, after(i), before(i + 1)),
            EffectStability::Stable
        );
    }
    let order = versions.order.get().unwrap();
    let flow = graph.semantic_flow().unwrap().indexed_reachability();
    let inspected = (0..COUNT - 1)
        .map(|i| order.between(flow, after(i), before(i + 1)).len())
        .sum::<usize>();
    assert_eq!(inspected, (COUNT - 1) as usize);
    assert_eq!(
        versions.stability(&effect, domain, after(0), before(COUNT - 1)),
        EffectStability::Changed
    );
    assert_eq!(
        versions.stability(&effect, domain, after(COUNT - 1), before(0)),
        EffectStability::DoesNotReach
    );
}

#[test]
fn shared_cycle_ranks_keep_all_effects_and_conditional_queries_preserve_proof_order() {
    let mut loop_test = SemanticLoopTest::pure(SemanticPredicate::True);
    loop_test.condition = predicate(10);
    let branch_test = predicate(5);
    let condition = branch_test.value.clone();
    let root = SemanticNode::Sequence(vec![
        block(0, 0..5),
        SemanticNode::If {
            condition: branch_test,
            then_node: Box::new(block(1, 6..8)),
            else_node: Some(Box::new(block(2, 8..10))),
        },
        SemanticNode::Loop {
            control: SemanticLoopControl::Region(RegionId::new(1)),
            header: None,
            kind: SemanticLoopKind::PreTested,
            test: loop_test,
            body: Box::new(block(3, 11..15)),
        },
        block(4, 15..18),
    ]);
    let mut graph = ValueFlowGraph::build_source(&root, &BTreeSet::new()).unwrap();
    let nonterminal = graph.logic.predicate(&condition).unwrap();
    let versions = EffectVersions::new(&graph);
    let effect = EffectSummary::synchronization();
    for _ in 0..2 {
        assert_eq!(
            versions.stability(&effect, nonterminal, after(0), before(1)),
            EffectStability::Stable
        );
    }
    assert!(versions.order.get().is_none());
    assert_eq!(versions.uncharged_facts.get(), Some(&false));
    let domain = graph.logic.truth();
    let flow = graph.semantic_flow().unwrap();
    for s in 0..18 {
        for t in 0..18 {
            let (from, to) = (after(s), before(t));
            let expected = if !flow.must_reach(from, to) {
                EffectStability::DoesNotReach
            } else if graph.effects.iter().any(|fact| {
                fact.point != from
                    && fact.point != to
                    && !graph.logic.disjoint(domain, fact.domain).unwrap()
                    && flow.reaches(from, fact.point)
                    && flow.reaches(fact.point, to)
            }) {
                EffectStability::Changed
            } else {
                EffectStability::Stable
            };
            assert_eq!(
                versions.stability(&effect, domain, from, to),
                expected,
                "{s} to {t}"
            );
        }
    }
    let order = versions.order.get().unwrap();
    let reachability = flow.indexed_reachability();
    assert_eq!(
        reachability.topological_component(after(11)),
        reachability.topological_component(before(12))
    );
    assert!(order.between(reachability, after(11), before(12)).len() >= 4);
    assert_eq!(
        versions.stability(&effect, domain, after(11), before(12)),
        EffectStability::Changed
    );
}
