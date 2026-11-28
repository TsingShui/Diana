mod support;

const FIXTURE: &[u8] =
    include_bytes!("../../../quality/fixtures/classes/instance-field-owners/classes.dex");

fn recover(owner: &str, name: &str, descriptor: &str) -> String {
    let engine = support::engine(FIXTURE);
    let class = support::class(FIXTURE, &engine, owner);
    let facts = engine.class_facts(class).expect("class facts");
    let method = facts
        .resolve_method(
            &name.encode_utf16().collect::<Vec<_>>(),
            &descriptor.encode_utf16().collect::<Vec<_>>(),
        )
        .expect("original method identity");
    let analysis = engine
        .analyze_method(class, method)
        .expect("method analysis");
    assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
    analysis.rendered.text().to_owned()
}

#[test]
fn current_receiver_keeps_each_field_owner() {
    for (name, descriptor, access) in [
        ("readOwn", "()I", "return this.value;"),
        ("readParent", "()I", "return super.value;"),
        ("readRoot", "()I", "return ((RootFields) this).value;"),
        ("writeParent", "(I)V", "super.value = "),
        ("writeRoot", "(I)V", "((RootFields) this).value = "),
    ] {
        let text = recover("Lfixture/ShadowFields;", name, descriptor);
        assert!(text.contains(access), "{name}: {text}");
    }
}

#[test]
fn foreign_field_owner_qualifies_one_receiver_evaluation() {
    let text = recover(
        "Lfixture/ForeignFieldAccess;",
        "writeChosenRoot",
        "(Lfixture/ShadowFields;I)V",
    );
    let call = "ForeignFieldAccess.pick(";
    assert_eq!(text.matches(call).count(), 1, "{text}");
    let (_, write) = text
        .split_once("((RootFields) ForeignFieldAccess.pick(")
        .unwrap_or_else(|| panic!("{text}"));
    assert!(write.contains(")).value = "), "{text}");
}
