use super::*;

#[test]
fn definition_lookup_preserves_absent_and_conflicting_facts_after_repeated_writes() {
    let object = TypeState::Exact(JavaType::source_class("java.lang.Object"));
    let mut states = SourceTypeStates::default();
    states.insert(1, TypeState::Conflict);
    states.insert(2, object.clone());
    let definitions = states.current().clone();
    assert!(states.definition(&2).is_none());
    states.seal_definitions();
    for key in [1, 2, 3, 1, 3] {
        states.insert(key, object.clone());
        assert_eq!(states.definition(&key), definitions.get(&key));
    }
    assert_eq!(
        states.definitions(),
        definitions.iter().map(|(&key, ty)| (key, ty)).collect()
    );
    let retained = states.retained(&BTreeSet::new());
    assert_eq!(retained.definition(&1), None);
    assert_eq!(
        retained.definition(&2),
        Some(&JavaType::source_class("java.lang.Object"))
    );
    assert_eq!(retained.definition(&3), None);
    assert_eq!(
        retained.get(&3),
        Some(&JavaType::source_class("java.lang.Object"))
    );
}
