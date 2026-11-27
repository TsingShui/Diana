use super::*;

#[test]
fn sparse_additions_and_allocated_holes_keep_type_order_and_conflicts() {
    let mut types = SourceTypeEnvironment {
        allocated: vec![None, Some(ArgType::INT), None].into_boxed_slice(),
        ..Default::default()
    };
    types.bind_variable(u32::MAX, ArgType::INT).unwrap();
    types.bind_variable(2, ArgType::DOUBLE).unwrap();
    types.bind_variable(13, ArgType::FLOAT).unwrap();
    types.bind_variable(0, ArgType::LONG).unwrap();
    types.refine_booleans([1, 99, u32::MAX]);
    assert!(matches!(
        types.bind_variable(2, ArgType::LONG),
        Err(TypeConstraintError::ConflictingSourceVariable { variable: 2, .. })
    ));
    assert!(matches!(
        types.bind_variable(u32::MAX, ArgType::FLOAT),
        Err(TypeConstraintError::ConflictingSourceVariable {
            variable: u32::MAX,
            ..
        })
    ));
    assert_eq!(
        types.known_types().cloned().collect::<Vec<_>>(),
        [
            ArgType::LONG,
            ArgType::BOOLEAN,
            ArgType::DOUBLE,
            ArgType::FLOAT,
            ArgType::BOOLEAN
        ],
    );
    assert_eq!(types.allocated.len(), 3);
}

#[test]
fn sorted_source_binding_preserves_sparse_rows_and_the_first_conflict() {
    use crate::ir::analysis::{CodeVariables, SsaClasses};

    let first = SsaVar::new(3, 0);
    let missing = SsaVar::new(4, 9);
    let skipped = SsaVar::new(7, 3);
    let second = SsaVar::new(8, 4);
    let later = SsaVar::new(15, 0);
    let mut classes = SsaClasses::default();
    for alias in [missing, second, later] {
        classes.union(first, alias);
    }
    let values = SsaValueGraph::default();
    let variables = CodeVariables::from_classes(&values, &[], classes);
    let ssa = SsaTypeEnvironment::from_sorted(vec![
        (first, ArgType::INT),
        (skipped, ArgType::LONG),
        (second, ArgType::FLOAT),
        (later, ArgType::object("example/Later")),
    ]);
    let error = SourceTypeEnvironment::from_ssa(
        ssa,
        &variables,
        &values,
        &BTreeMap::new(),
        &ClassHierarchyIndex::default(),
    )
    .unwrap_err();
    assert!(
        matches!(error, TypeConstraintError::ConflictingSourceValues {
        variable:0, left_value, left:ArgType::INT, right_value, right:ArgType::FLOAT,
    } if left_value == first && right_value == second)
    );
}
