use super::*;
use crate::ir::{BlockId, BoolExpr, BoolVariable, InstructionId};

#[test]
fn block_domains_keep_missing_and_false_distinct_when_labels_become_sparse() {
    let mut logic = DomainLogic::new(&Default::default());
    let mut domains = BlockDomains::default();
    let no = logic.falsity();
    let yes = logic.truth();
    domains.record(BlockId::new(4), no, &mut logic).unwrap();
    assert!(domains.get(BlockId::new(4)).is_some());
    assert!(domains.get(BlockId::new(3)).is_none());
    for (id, domain) in [(u32::MAX, no), (7, yes), (4, yes)] {
        domains
            .record(BlockId::new(id), domain, &mut logic)
            .unwrap();
    }
    assert!(domains.get(BlockId::new(u32::MAX)).is_some());
    assert!(domains.get(BlockId::new(u32::MAX - 1)).is_none());
    assert_eq!(domains.get(BlockId::new(4)), Some(yes));
    assert_eq!(domains.get(BlockId::new(7)), Some(yes));
    assert_eq!(domains.get(BlockId::new(u32::MAX)), Some(no));
}

#[test]
fn replicated_blocks_charge_unions_in_the_original_occurrence_order() {
    let vars = (0..20)
        .map(|i| BoolVariable::Instruction(InstructionId::new(i)))
        .collect();
    let mut logic = DomainLogic::new(&vars);
    let mut reference = DomainLogic::new(&vars);
    let mut actual = BlockDomains::default();
    let mut expected = std::collections::BTreeMap::<BlockId, ControlDomain>::new();
    for i in 0..280 {
        let id = BlockId::new(if i == 145 {
            u32::MAX
        } else {
            (i * 19 % 37) as u32
        });
        let symbol = |n| BoolExpr::Symbol(BoolVariable::Instruction(InstructionId::new(n)));
        let expression = BoolExpr::and(vec![
            symbol(i % 20),
            BoolExpr::not(symbol((i * 7 + 1) % 20)),
        ]);
        let current = logic.compile(&expression).unwrap();
        let term = reference.compile(&expression).unwrap();
        actual.record(id, current, &mut logic).unwrap();
        let combined = match expected.get(&id) {
            Some(&previous) => reference.or(previous, term).unwrap(),
            None => term,
        };
        expected.insert(id, combined);
        assert_eq!(actual.get(id), Some(combined));
        assert_eq!(
            logic.proof_budget_exhausted(),
            reference.proof_budget_exhausted()
        );
    }
}
