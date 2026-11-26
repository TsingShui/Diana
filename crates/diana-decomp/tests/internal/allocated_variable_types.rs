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
