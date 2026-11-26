use super::*;
use crate::ir::{InsnNode, InsnType, InstructionId, SemanticOperation, SemanticPredicate};

#[test]
fn terminal_only_collection_preserves_lazy_unknown_symbol_errors() {
    let mut logic = DomainLogic::new(&BTreeSet::new());
    let t = logic.truth();
    let f = logic.falsity();
    let mut op = InsnNode::new(InsnType::If, 0);
    op.id = InstructionId::new(77);
    let atom = SemanticPredicate::Test(SemanticOperation::from_instruction(op).unwrap());
    let absorbed = SemanticPredicate::And(vec![atom.clone(), SemanticPredicate::False]);
    assert_eq!(logic.predicate(&absorbed).unwrap(), f);
    assert_eq!(logic.and(t, f).unwrap(), f);
    assert!(logic.disjoint(t, f).unwrap());
    assert!(!logic.disjoint(t, t).unwrap());
    assert!(logic.disjoint_without_bdd_work(t));
    assert!(logic.bdd.get().is_none());
    assert!(matches!(
        logic.predicate(&SemanticPredicate::Not(Box::new(atom))),
        Err(crate::analysis::value_recovery::ValueRecoveryError::ControlDomain(DomainError {
            kind: DomainFailure::Boolean(BddError::UnknownVariable(BoolVariable::Instruction(id))),
        })) if id == InstructionId::new(77)
    ));
}

#[test]
fn growing_and_replayed_joins_keep_one_derivation_each_across_saturation() {
    let variables = (0..48).map(BoolVariable::Atom).collect();
    for conjunction in [true, false] {
        let mut logic = DomainLogic::new(&variables);
        let atoms = (0..48)
            .map(|i| logic.symbol(BoolVariable::Atom(i)).unwrap())
            .collect::<Vec<_>>();
        let mut prefixes = vec![atoms[0]];
        for &atom in &atoms[1..] {
            prefixes.push(
                logic
                    .junction(*prefixes.last().unwrap(), atom, conjunction)
                    .unwrap(),
            );
        }
        assert_eq!(logic.joins.len(), atoms.len() - 1);
        assert_eq!(logic.junction_index.len(), atoms.len() - 2);
        assert_eq!(logic.term_arena.len(), atoms.len());
        for (i, &prefix) in prefixes.iter().enumerate().skip(2) {
            assert_eq!(
                logic.junction_terms(&prefix, conjunction).unwrap(),
                &atoms[..=i]
            );
        }
        let pair = logic.node(prefixes[1]).unwrap();
        assert!(matches!(
            pair,
            DomainNode::AndPair(_) | DomainNode::OrPair(_)
        ));
        assert_eq!(
            pair.terms(conjunction, &logic.term_arena),
            Some(atoms[..2].as_ref())
        );
        let alternative = logic.junction(atoms[1], atoms[2], conjunction).unwrap();
        let count = logic.joins.len();
        let opposite = logic.not(atoms[0]).unwrap();
        for saturated in [false, true] {
            logic.saturated.set(saturated);
            for i in 1..atoms.len() {
                assert_eq!(
                    logic
                        .junction(atoms[i], prefixes[i - 1], conjunction)
                        .unwrap(),
                    prefixes[i]
                );
            }
            assert_eq!(
                logic.junction(atoms[0], alternative, conjunction).unwrap(),
                prefixes[2]
            );
            let absorbing = if conjunction {
                logic.falsity()
            } else {
                logic.truth()
            };
            assert_eq!(
                logic.junction(prefixes[47], opposite, conjunction).unwrap(),
                absorbing
            );
            assert_eq!(logic.joins.len(), count);
            assert_eq!(logic.proof_budget_exhausted(), saturated);
        }
    }
}

#[test]
fn collected_proofs_require_the_same_inputs_and_keep_planner_mutations_private() {
    let cache = DomainProofCache::default();
    let names = (0..20).map(BoolVariable::Atom).collect::<Vec<_>>();
    let mut previous = None;
    for (reverse, conjunction, reuse) in [
        (false, false, false),
        (false, false, true),
        (false, true, false),
        (true, true, false),
        (true, true, true),
    ] {
        let mut order = names.clone();
        if reverse {
            order.reverse();
        }
        let mut logic = DomainLogic::ordered(order.into_iter());
        logic.begin_collection(Some(&cache));
        let a = logic.symbol(BoolVariable::Atom(2)).unwrap();
        let b = logic.symbol(BoolVariable::Atom(7)).unwrap();
        let c = logic.symbol(BoolVariable::Atom(11)).unwrap();
        let ab = logic.and(a, b).unwrap();
        let not_a = logic.not(a).unwrap();
        let nc = logic.and(not_a, c).unwrap();
        let value = logic.junction(ab, nc, conjunction).unwrap();
        logic.finish_collection(Some(&cache)).unwrap();
        let checkpoint = cache.0.lock().unwrap().as_ref().unwrap().bdd.truth();
        assert_eq!(previous == Some(checkpoint), reuse);
        previous = Some(checkpoint);
        assert!(matches!(
            logic.bdd().and(logic.bdd().truth(), checkpoint),
            Err(BddError::ForeignHandle)
        ));
        // The formula is either (a && b) || (!a && c), or false.
        for mask in 0..8 {
            let mut valuation = logic.truth();
            for (bit, atom) in [a, b, c].into_iter().enumerate() {
                let literal = if mask & (1 << bit) == 0 {
                    logic.not(atom).unwrap()
                } else {
                    atom
                };
                valuation = logic.and(valuation, literal).unwrap();
            }
            let expected = !conjunction
                && if mask & 1 == 1 {
                    mask & 2 != 0
                } else {
                    mask & 4 != 0
                };
            assert_eq!(logic.disjoint(valuation, value).unwrap(), !expected);
        }
    }
}

#[test]
fn collection_checks_an_unknown_symbol_after_the_bounded_prefix() {
    let names = (0..28).map(BoolVariable::Atom).collect::<BTreeSet<_>>();
    let cache = DomainProofCache::default();
    for deferred in [true, false] {
        let mut unsaturated = DomainLogic::new(&names);
        if deferred {
            unsaturated.begin_collection(Some(&cache));
        }
        let first = unsaturated.symbol(BoolVariable::Atom(0)).unwrap();
        let second = unsaturated.symbol(BoolVariable::Atom(14)).unwrap();
        unsaturated.or(first, second).unwrap();
        assert!(matches!(
            unsaturated.symbol(BoolVariable::Atom(99)),
            Err(DomainError {
                kind: DomainFailure::Boolean(BddError::UnknownVariable(BoolVariable::Atom(99)))
            })
        ));

        // All left-hand variables precede all right-hand variables. The BDD
        // needs to distinguish the possible left-hand assignments before
        // testing the matching right-hand bits and reaches the node budget.
        for check_after_finish in [false, true, true] {
            let mut logic = DomainLogic::new(&names);
            if deferred {
                logic.begin_collection(Some(&cache));
            }
            let first = logic.symbol(BoolVariable::Atom(0)).unwrap();
            let mut equal = logic.truth();
            for i in 0..14 {
                let x = logic.symbol(BoolVariable::Atom(i)).unwrap();
                let y = logic.symbol(BoolVariable::Atom(14 + i)).unwrap();
                let xy = logic.and(x, y).unwrap();
                let nx = logic.not(x).unwrap();
                let ny = logic.not(y).unwrap();
                let both_clear = logic.and(nx, ny).unwrap();
                let same = logic.or(xy, both_clear).unwrap();
                equal = logic.and(equal, same).unwrap();
            }
            if check_after_finish {
                logic.finish_collection(deferred.then_some(&cache)).unwrap();
                assert!(logic.proof_budget_exhausted());
            }
            let unknown = logic.symbol(BoolVariable::Atom(99)).unwrap();
            assert!(logic.proof_budget_exhausted());
            assert!(logic.exact(unknown).is_none());
            assert!(logic.exact(equal).is_none());
            assert!(logic.covered_by(first, [first]).unwrap());
        }
        if deferred {
            // The new collection ends before the old equality diagram grew
            // large. It must retain exactly the budget after these three nodes.
            let mut shorter = DomainLogic::new(&names);
            shorter.begin_collection(Some(&cache));
            let x = shorter.symbol(BoolVariable::Atom(0)).unwrap();
            let y = shorter.symbol(BoolVariable::Atom(14)).unwrap();
            let xy = shorter.and(x, y).unwrap();
            shorter.finish_collection(Some(&cache)).unwrap();
            assert!(!shorter.proof_budget_exhausted());
            assert!(shorter.implies(xy, x).unwrap());
            assert!(shorter.exact(xy).is_some());
            assert!(matches!(
                shorter.symbol(BoolVariable::Atom(99)),
                Err(DomainError {
                    kind: DomainFailure::Boolean(BddError::UnknownVariable(_))
                })
            ));
        }
    }
}

#[test]
fn branching_term_prefixes_preserve_each_sibling() {
    let names = (0..8).map(BoolVariable::Atom).collect::<BTreeSet<_>>();
    for conjunction in [false, true] {
        let mut logic = DomainLogic::new(&names);
        let atoms: Vec<_> = names
            .iter()
            .map(|name| logic.symbol(name.clone()).unwrap())
            .collect();
        let pair = logic.junction(atoms[1], atoms[2], conjunction).unwrap();
        let parent = logic.junction(pair, atoms[3], conjunction).unwrap();
        for extra in [5, 6, 0, 4, 7] {
            let sibling = logic.junction(parent, atoms[extra], conjunction).unwrap();
            let mut expected = vec![atoms[1], atoms[2], atoms[3], atoms[extra]];
            expected.sort_unstable();
            assert_eq!(
                logic.junction_terms(&sibling, conjunction).unwrap(),
                expected
            );
            assert_eq!(
                logic.junction_terms(&parent, conjunction).unwrap(),
                &atoms[1..4]
            );
        }
    }
}
