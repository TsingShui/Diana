use super::*;
use crate::ir::analysis::ClassHierarchyIndex;

#[test]
fn intersection_preserves_left_order_and_multiplicity_for_borrowed_and_owned_candidates() {
    use PrimitiveType::{Array, Boolean, Byte, Double, Int, Object};
    let ordered = [Object, Int, Array, Boolean, Int, Byte, Double];
    let classes = [Array, Boolean, Byte, Double, Int, Object];
    let hierarchy = ClassHierarchyIndex::default();
    for left in [
        ArgType::equality_operand(),
        ArgType::Unknown(ordered.to_vec().into()),
    ] {
        let ArgType::Unknown(original) = &left else {
            unreachable!()
        };
        for mask in 0..64 {
            let selected: Vec<_> = classes
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(position, value)| (mask & (1 << position) != 0).then_some(value))
                .collect();
            let common: Vec<_> = original
                .iter()
                .filter(|ty| selected.contains(ty))
                .copied()
                .collect();
            let expected = if common.is_empty() {
                ArgType::unknown()
            } else {
                ArgType::Unknown(common.into())
            };
            assert_eq!(
                TypeLattice::join(left.clone(), ArgType::Unknown(selected.into()), &hierarchy),
                expected
            );
        }
    }
}

#[test]
fn type_reduction_keeps_sorted_evidence_order_and_coincident_domain_hints() {
    use PrimitiveType::{Boolean, Float, Int};
    let hierarchy = ClassHierarchyIndex::default();
    let first = ArgType::Unknown([Boolean, Int, Int].into());
    let second = ArgType::Unknown([Int, Boolean].into());
    for order in [[first.clone(), second.clone()], [second, first.clone()]] {
        let mut evidence = TypeBounds::default();
        evidence.insert(TypeBound::new(BoundKind::Lower, ArgType::DOUBLE));
        for ty in order {
            evidence.insert(TypeBound::new(BoundKind::Exact, ty));
        }
        assert_eq!(evidence.produced_type(&hierarchy), Some(first.clone()));
    }

    let mut domain = TypeBounds::from([
        TypeBound::new(BoundKind::Domain, ArgType::INT),
        TypeBound::new(BoundKind::Domain, ArgType::Unknown([Int].into())),
    ]);
    assert_eq!(
        TypeLattice::resolve_state(&domain, &hierarchy),
        Some(ArgType::INT)
    );
    domain.insert(TypeBound::new(
        BoundKind::Domain,
        ArgType::Unknown([Float].into()),
    ));
    assert_eq!(TypeLattice::resolve_state(&domain, &hierarchy), None);
}
