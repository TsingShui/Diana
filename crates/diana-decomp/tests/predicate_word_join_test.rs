mod support;

const FIXTURE: &[u8] =
    include_bytes!("../../../quality/fixtures/branches/predicate-word-join/classes.dex");

#[test]
fn an_integer_loop_keeps_its_width_when_the_initializer_is_a_predicate() {
    let engine = support::engine(FIXTURE);
    let class = support::class(FIXTURE, &engine, "Lfixture/PredicateWordJoin;");
    let facts = engine.class_facts(class).expect("class facts");
    let method = facts
        .resolve_method(
            &"run".encode_utf16().collect::<Vec<_>>(),
            &"(Ljava/lang/Object;[Z)I".encode_utf16().collect::<Vec<_>>(),
        )
        .expect("original method");
    let analysis = engine.analyze_method(class, method).expect("method");

    assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
    let text = analysis.rendered.text();
    assert!(
        text.lines()
            .any(|line| line.trim_start().starts_with("int ")
                && line.contains("instanceof String ? 1 : 0")),
        "{text}"
    );
    assert!(text.contains(" + 1"), "{text}");
}
