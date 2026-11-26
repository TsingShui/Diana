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
