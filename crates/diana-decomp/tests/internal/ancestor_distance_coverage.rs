use super::*;

#[test]
fn subtype_misses_preserve_unavailable_parents_and_closed_cycles() {
    let mut index = ClassHierarchyIndex::default();
    index.add("Child", ["Cycle".into(), "External".into()]);
    index.add("Cycle", ["Loop".into()]);
    index.add("Loop", ["Cycle".into()]);
    for frozen in [false, true] {
        if frozen {
            index.freeze_distances();
        }
        assert_eq!(
            index.subtype_relation("Child", "Loop"),
            SubtypeRelation::Yes
        );
        assert_eq!(
            index.subtype_relation("Child", "Other"),
            SubtypeRelation::Unknown
        );
        assert_eq!(
            index.subtype_relation("Cycle", "Other"),
            SubtypeRelation::No
        );
    }
}

#[test]
fn layered_miss_uses_the_effective_parents_of_an_inherited_start() {
    let mut base = ClassHierarchyIndex::default();
    base.add("Inherited", ["Parent".into()]);
    base.add("Parent", Vec::new());
    base.freeze_distances();
    let mut overlay = ClassHierarchyIndex::layered(Arc::new(base));
    overlay.add("Parent", ["Introduced".into()]);
    overlay.freeze_distances();
    assert_eq!(
        overlay.subtype_relation("Inherited", "Introduced"),
        SubtypeRelation::Yes
    );
    assert_eq!(
        overlay.subtype_relation("Inherited", "Other"),
        SubtypeRelation::Unknown
    );
}
