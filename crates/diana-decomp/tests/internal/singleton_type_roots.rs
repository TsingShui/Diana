use super::*;
use crate::ir::analysis::ClassHierarchyIndex;

#[test]
fn unobserved_singleton_endpoint_transmits_evidence_without_becoming_an_output() {
    let source = SsaVar::new(0, 0);
    let middle = SsaVar::new(1, 1);
    let target = SsaVar::new(2, 1);
    let constraints = NormalizedTypeConstraints {
        members: None,
        bounds: BTreeMap::from([
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
        types,
        BTreeMap::from([(source, ArgType::LONG), (target, ArgType::LONG)])
    );
}
