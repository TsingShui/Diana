use super::*;

#[test]
fn ordered_values_keep_their_first_row_and_all_event_ordinals() {
    let mut rows = OccurrenceRows::default();
    let mut first = std::collections::BTreeMap::new();
    let keys = (0..24)
        .rev()
        .map(|n| SsaVar::new(n % 7, n))
        .chain([SsaVar::new(u32::MAX, u32::MAX)])
        .collect::<Vec<_>>();
    for (i, &key) in keys.iter().enumerate() {
        rows.push(key, i);
        first.insert(key, i);
        if matches!(i, 5 | 11) {
            assert_eq!(
                rows.keys().copied().collect::<Vec<_>>(),
                first.keys().copied().collect::<Vec<_>>()
            );
        }
    }
    for (i, &key) in keys.iter().enumerate() {
        rows.push(key, i + keys.len());
    }
    assert_eq!(rows.iter().count(), keys.len());
    assert!(rows.keys().is_sorted());
    for (&key, values) in rows.iter() {
        let i = first[&key];
        assert_eq!(rows.row(&key), Some(i));
        assert_eq!(values, [i, i + keys.len()]);
    }
    assert_eq!(rows.row(&SsaVar::new(u32::MAX, 0)), None);
}
