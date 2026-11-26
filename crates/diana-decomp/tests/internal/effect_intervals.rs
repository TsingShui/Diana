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
    assert!(versions.domain_runs.get().unwrap().is_some());
    assert!(versions.order.get().is_some());
    assert_eq!(versions.uncharged_facts.get(), Some(&false));
    let mut oracle = ValueFlowGraph::build_source(&root, &BTreeSet::new()).unwrap();
    let oracle_nonterminal = oracle.logic.predicate(&condition).unwrap();
    let domain = graph.logic.truth();
    let flow = graph.semantic_flow().unwrap();
    for (domain, oracle_domain) in [
        (domain, oracle.logic.truth()),
        (nonterminal, oracle_nonterminal),
    ] {
        for s in 0..18 {
            for t in 0..18 {
                let (from, to) = (after(s), before(t));
                let expected = if !flow.must_reach(from, to) {
                    EffectStability::DoesNotReach
                } else {
                    let mut changed = false;
                    for fact in &oracle.effects {
                        if fact.point != from
                            && fact.point != to
                            && !oracle.logic.disjoint(oracle_domain, fact.domain).unwrap()
                        {
                            changed |=
                                flow.reaches(from, fact.point) && flow.reaches(fact.point, to);
                        }
                    }
                    if changed {
                        EffectStability::Changed
                    } else {
                        EffectStability::Stable
                    }
                };
                assert_eq!(
                    versions.stability(&effect, domain, from, to),
                    expected,
                    "{s} to {t}"
                );
            }
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

#[test]
fn grouped_checks_keep_first_charges_and_recheck_separated_domains_after_saturation() {
    use super::{EffectDomainRuns, EffectFact};
    use crate::analysis::value_recovery::domain::DomainLogic;
    use crate::ir::{BoolExpr, BoolVariable};

    let variables = (0..4).map(BoolVariable::Atom).collect();
    let mut logic = DomainLogic::new(&variables);
    let domains = (0..4)
        .map(|id| {
            logic
                .compile(&BoolExpr::Symbol(BoolVariable::Atom(id)))
                .unwrap()
        })
        .collect::<Vec<_>>();
    let mut effects = Vec::new();
    for (domain, count) in [(0, 40), (1, 9), (2, 15), (3, 8), (0, 47)] {
        for _ in 0..count {
            effects.push(EffectFact {
                point: after(effects.len() as u64),
                domain: domains[domain],
                effect: if domain == 1 {
                    EffectSummary::pure()
                } else {
                    EffectSummary::synchronization()
                },
            });
        }
    }
    let from = after(0);
    let to = after(118);
    let conflicts = |effect: &EffectFact| {
        effect.point != from && effect.point != to && !effect.effect.is_pure()
    };
    // Before the proof limit, domain zero is disjoint. The first query on
    // domain two crosses the limit, making subsequent checks conservative.
    let mut saturated = false;
    let expected: Vec<_> = effects
        .iter()
        .map(|fact| {
            if !conflicts(fact) {
                return false;
            }
            saturated |= fact.domain == domains[2];
            saturated || fact.domain != domains[0]
        })
        .collect();
    let runs = EffectDomainRuns::grouped(&effects).unwrap();
    let saturated = std::cell::Cell::new(false);
    let mut checked = Vec::new();
    assert!(runs.prepare(
        &effects,
        domains[0],
        conflicts,
        |domain| {
            checked.push(domain);
            saturated.set(saturated.get() || domain == domains[2]);
            saturated.get() || domain != domains[0]
        },
        || saturated.get(),
    ));
    assert_eq!(checked, [domains[0], domains[2], domains[3], domains[0]]);
    for (i, fact) in effects.iter().enumerate() {
        assert_eq!(
            runs.eligible(i) && conflicts(fact),
            expected[i],
            "effect {i}"
        );
    }
    assert!(EffectDomainRuns::grouped(&[]).is_none());
}

#[test]
fn only_complete_unsaturated_domain_rows_are_reused_for_new_conflicts() {
    use super::{EffectDomainRuns, EffectFact};
    use crate::analysis::value_recovery::domain::DomainLogic;
    use crate::ir::{BoolExpr, BoolVariable};
    use std::cell::Cell;

    let variables = (0..5).map(BoolVariable::Atom).collect();
    let mut logic = DomainLogic::new(&variables);
    let domains: Vec<_> = (0..5)
        .map(|id| {
            logic
                .compile(&BoolExpr::Symbol(BoolVariable::Atom(id)))
                .unwrap()
        })
        .collect();
    let effects: Vec<_> = (0..10)
        .map(|i| EffectFact {
            point: after(i),
            domain: domains[i as usize / 2],
            effect: EffectSummary::synchronization(),
        })
        .collect();
    let runs = EffectDomainRuns::grouped(&effects).unwrap();

    // The first query establishes that runs 0, 2 and 4 are disjoint. The
    // current endpoints and effect leave runs 1 and 3 unqueried.
    let mut checked = Vec::new();
    assert!(!runs.prepare(
        &effects,
        domains[0],
        |fact| [domains[0], domains[2], domains[4]].contains(&fact.domain),
        |domain| {
            checked.push(domain);
            false
        },
        || false,
    ));
    assert_eq!(checked, [domains[0], domains[2], domains[4]]);
    checked.clear();

    // An exact negative for the newly conflicting run 1 leaves the arena
    // intact. Run 3 then saturates it, so the old negative for run 4 changes.
    let saturated = Cell::new(false);
    assert!(runs.prepare(
        &effects,
        domains[0],
        |_| true,
        |domain| {
            checked.push(domain);
            saturated.set(saturated.get() || domain == domains[3]);
            saturated.get()
        },
        || saturated.get(),
    ));
    assert_eq!(checked, domains);
    for i in 0..10 {
        assert_eq!(runs.eligible(i), i >= 6);
    }

    // A different movement starts a fresh ordered scan, including domains
    // proved disjoint before saturation in the previous movement.
    checked.clear();
    assert!(runs.prepare(
        &effects,
        domains[1],
        |_| true,
        |domain| {
            checked.push(domain);
            true
        },
        || false,
    ));
    assert_eq!(checked, domains);
    checked.clear();
    assert!(runs.prepare(
        &effects,
        domains[1],
        |fact| fact.domain == domains[0],
        |domain| {
            checked.push(domain);
            false
        },
        || false,
    ));
    assert!(checked.is_empty());
    // A proof elsewhere in the same arena may cross the limit between moves.
    assert!(runs.prepare(
        &effects,
        domains[1],
        |_| true,
        |domain| {
            checked.push(domain);
            true
        },
        || true,
    ));
    assert_eq!(checked, domains);
}
