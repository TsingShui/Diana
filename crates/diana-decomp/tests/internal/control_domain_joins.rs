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
        assert_eq!(
            logic.conjunctions.len() + logic.disjunctions.len(),
            atoms.len() - 2
        );
        let pair = logic.node(prefixes[1]).unwrap();
        assert!(matches!(
            pair,
            DomainNode::AndPair(_) | DomainNode::OrPair(_)
        ));
        assert_eq!(pair.terms(conjunction), Some(atoms[..2].as_ref()));
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
