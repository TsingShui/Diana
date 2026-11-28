use super::*;
use crate::ir::analysis::ClassHierarchyIndex;

#[test]
fn unobserved_singleton_endpoint_transmits_evidence_without_becoming_an_output() {
    let source = SsaVar::new(0, 0);
    let middle = SsaVar::new(1, 1);
    let target = SsaVar::new(2, 1);
    let constraints = NormalizedTypeConstraints {
        members: None,
        bounds: Vec::from([
            (
                source,
                TypeBounds::from([TypeBound::new(BoundKind::Exact, ArgType::LONG)]),
            ),
            (target, TypeBounds::default()),
        ]),
        flows: BTreeSet::from([
            (source, middle, FlowKind::Phi),
            (middle, target, FlowKind::Phi),
        ]),
        upper_flows: BTreeSet::new(),
        arrays: BTreeSet::new(),
    };
    let types = TypeLattice::solve(constraints, &ClassHierarchyIndex::default());
    assert_eq!(
        types.types,
        [(source, ArgType::LONG), (target, ArgType::LONG)]
    );
}

#[test]
fn a_copy_membership_root_can_be_absent_from_the_observations() {
    let root = SsaVar::new(2, 0);
    let alias = SsaVar::new(5, 7);
    let other = SsaVar::new(11, 2);
    let constraints = NormalizedTypeConstraints {
        members: Some(vec![(alias, root), (other, other)]),
        bounds: vec![
            (
                root,
                TypeBounds::from([TypeBound::new(BoundKind::Exact, ArgType::INT)]),
            ),
            (other, TypeBounds::default()),
        ],
        flows: BTreeSet::from([(root, other, FlowKind::Phi)]),
        upper_flows: BTreeSet::new(),
        arrays: BTreeSet::new(),
    };
    let types = TypeLattice::solve(constraints, &ClassHierarchyIndex::default());
    assert_eq!(types.types, [(alias, ArgType::INT), (other, ArgType::INT)]);
}

#[test]
fn a_certified_predicate_word_can_feed_integer_storage_in_a_phi_cycle() {
    let predicate = SsaVar::new(0, 0);
    let head = SsaVar::new(0, 1);
    let tail = SsaVar::new(0, 2);
    let incremented = SsaVar::new(0, 3);
    let predicate_only = SsaVar::new(1, 0);
    let constraints = NormalizedTypeConstraints {
        members: None,
        bounds: vec![
            (
                predicate,
                TypeBounds::from([TypeBound::new(BoundKind::PredicateWord, ArgType::BOOLEAN)]),
            ),
            (head, TypeBounds::default()),
            (
                tail,
                TypeBounds::from([TypeBound::new(BoundKind::Upper, ArgType::INT)]),
            ),
            (
                incremented,
                TypeBounds::from([TypeBound::new(BoundKind::Lower, ArgType::INT)]),
            ),
            (predicate_only, TypeBounds::default()),
        ],
        flows: BTreeSet::from([
            (predicate, head, FlowKind::Phi),
            (predicate, predicate_only, FlowKind::Phi),
            (head, tail, FlowKind::Phi),
            (incremented, tail, FlowKind::Phi),
            (tail, head, FlowKind::Phi),
        ]),
        upper_flows: BTreeSet::new(),
        arrays: BTreeSet::new(),
    };

    let types = TypeLattice::solve(constraints, &ClassHierarchyIndex::default());

    assert_eq!(types.value_type(predicate), Some(&ArgType::BOOLEAN));
    assert_eq!(types.value_type(predicate_only), Some(&ArgType::BOOLEAN));
    assert_eq!(types.value_type(head), Some(&ArgType::INT));
    assert_eq!(types.value_type(tail), Some(&ArgType::INT));
}
