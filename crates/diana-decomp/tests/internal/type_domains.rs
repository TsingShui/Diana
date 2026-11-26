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
