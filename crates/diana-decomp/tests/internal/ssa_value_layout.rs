use super::*;

#[test]
fn lookup_and_iteration_preserve_gapped_versions_and_sparse_register_numbers() {
    let mut keys = (0..14).map(|v| SsaVar::new(3, 2 * v)).collect::<Vec<_>>();
    keys.extend([SsaVar::new(0, 7), SsaVar::new(4, u32::MAX)]);
    for sparse in [false, true] {
        if sparse {
            keys.push(SsaVar::new(u32::MAX, u32::MAX));
        }
        let mut builder = SsaValueBuilder::default();
        for &key in keys.iter().rev() {
            builder.value_mut(key).observe_type(&ArgType::INT);
        }
        let graph = builder.finish();
        let mut ordered = keys.clone();
        ordered.sort_unstable();
        assert_eq!(
            graph.values().map(|row| row.variable).collect::<Vec<_>>(),
            ordered
        );
        for &key in &keys {
            let row = graph.value(key).expect("original identity");
            assert_eq!(row.variable, key);
            assert!(row.observed_types.contains(&ArgType::INT));
        }
        for key in [
            SsaVar::new(0, 6),
            SsaVar::new(2, 0),
            SsaVar::new(3, 5),
            SsaVar::new(3, 100),
            SsaVar::new(4, 2),
            SsaVar::new(u32::MAX, 9),
        ] {
            assert!(graph.value(key).is_none());
        }
    }
}

#[test]
fn sparse_coalescing_numbers_declared_and_body_only_values_together() {
    let value = |index: u32| SsaVar::new(index / 3, index % 3);
    let mut builder = SsaValueBuilder::default();
    for key in [9, 8, 4, 2] {
        builder.value_mut(value(key)).observe_type(&ArgType::INT);
    }
    let graph = builder.finish();
    let inputs = [0, 4, 7, 12].map(value);
    let mut touched = SsaClasses::default();
    touched.union(value(7), value(13));
    touched.union(value(2), value(7));
    touched.union(value(8), value(3));
    touched.root(value(11));
    touched.root(value(9));

    let variables = CodeVariables::from_classes(&graph, &inputs, touched);
    let expected = [
        (0, 0),
        (2, 1),
        (3, 2),
        (4, 3),
        (7, 1),
        (8, 2),
        (9, 4),
        (11, 5),
        (12, 6),
        (13, 1),
    ]
    .map(|(index, id)| (value(index), id));
    assert_eq!(variables.assignments(), expected);
    assert_eq!(variables.next_id(), 7);
    for (value, id) in expected {
        assert_eq!(variables.variable_of(value), Some(id));
    }
    assert_eq!(variables.variable_of(value(10)), None);
}
