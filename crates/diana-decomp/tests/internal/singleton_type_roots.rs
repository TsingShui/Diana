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
        flows: BTreeSet::from([(source, middle), (middle, target)]),
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
        flows: BTreeSet::from([(root, other)]),
        upper_flows: BTreeSet::new(),
        arrays: BTreeSet::new(),
    };
    let types = TypeLattice::solve(constraints, &ClassHierarchyIndex::default());
    assert_eq!(types.types, [(alias, ArgType::INT), (other, ArgType::INT)]);
}
