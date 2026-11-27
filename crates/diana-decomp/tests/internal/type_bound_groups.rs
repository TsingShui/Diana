use super::*;

#[test]
fn type_evidence_preserves_priority_order_and_duplicates_when_merging_components() {
    let kinds = [
        BoundKind::Domain,
        BoundKind::Fallback,
        BoundKind::Upper,
        BoundKind::Lower,
        BoundKind::Exact,
    ];
    for sizes in [(1, 2), (2, 17), (16, 3), (21, 26)] {
        let mut combined = TypeBounds::default();
        let mut expected = BTreeSet::new();
        for (part, size) in [sizes.0, sizes.1].into_iter().enumerate() {
            let mut incoming = TypeBounds::default();
            let mut unique = BTreeSet::new();
            for i in (0..size).rev() {
                let kind = kinds[(i + part) % kinds.len()];
                for ty in [ArgType::VOID, ArgType::object(&format!("test/Bound{i}"))] {
                    let bound = TypeBound::new(kind, ty);
                    assert_eq!(incoming.insert(bound.clone()), unique.insert(bound.clone()));
                    assert!(!incoming.insert(bound.clone()));
                    expected.insert(bound);
                }
            }
            combined.merge(incoming);
        }
        for kind in kinds {
            let evidence: Vec<_> = expected
                .iter()
                .filter(|b| b.kind == kind)
                .map(|b| &b.ty)
                .collect();
            assert_eq!(combined.evidence(kind).collect::<Vec<_>>(), evidence);
        }
    }
}

#[test]
fn observations_keep_each_kind_independent_across_promotion_and_merging() {
    let known = ArgType::array(ArgType::object("test/Element"));
    let unknown = ArgType::unknown_object();
    let mut small = TypeBounds::default();
    small.observe(&known);
    small.observe(&unknown);
    assert!(!small.insert_ref(BoundKind::Domain, &known));
    assert!(!small.insert_ref(BoundKind::Fallback, &known));
    assert!(small.insert_ref(BoundKind::Exact, &known));
    assert!(small.evidence(BoundKind::Upper).next().is_none());
    assert_eq!(
        small.evidence(BoundKind::Fallback).collect::<Vec<_>>(),
        [&known]
    );

    let mut wide = TypeBounds::default();
    for i in (0..20).rev() {
        wide.insert(TypeBound::new(
            BoundKind::Lower,
            ArgType::object(&format!("test/T{i}")),
        ));
    }
    wide.observe(&known);
    wide.observe(&unknown);
    wide.merge(small);
    assert_eq!(
        wide.evidence(BoundKind::Exact).collect::<Vec<_>>(),
        [&known]
    );
    assert_eq!(
        wide.evidence(BoundKind::Fallback).collect::<Vec<_>>(),
        [&known]
    );
    assert_eq!(wide.evidence(BoundKind::Lower).count(), 20);
    assert_eq!(wide.evidence(BoundKind::Domain).count(), 2);
    assert!(wide.insert_ref(BoundKind::Upper, &unknown));
    assert!(!wide.insert_ref(BoundKind::Upper, &unknown));
}
