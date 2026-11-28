mod support;

const FIXTURE: &[u8] =
    include_bytes!("../../../quality/fixtures/branches/ordered-short-circuit/classes.dex");

#[test]
fn ordered_predicates_share_the_boolean_return_inside_the_loop() {
    let engine = support::engine(FIXTURE);
    let class = support::class(FIXTURE, &engine, "Lfixture/OrderedShortCircuit;");
    let facts = engine.class_facts(class).expect("class facts");
    let method = facts
        .resolve_method(
            &"run".encode_utf16().collect::<Vec<_>>(),
            &"(Ljava/lang/String;)Z".encode_utf16().collect::<Vec<_>>(),
        )
        .expect("original method");
    let analysis = engine.analyze_method(class, method).expect("method");
    assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
    let text = analysis.rendered.text();
    let guard = text
        .lines()
        .find(|line| line.contains("OrderedShortCircuit.acceptLeft("))
        .expect(text);
    let operator = guard.find(" || ").expect(guard);
    assert!(
        guard[operator..].contains("OrderedShortCircuit.acceptRight("),
        "{guard}"
    );
    let advance = text.find("OrderedShortCircuit.next(").expect(text);
    assert!(text.find(guard).expect(text) < advance, "{text}");
    assert_eq!(text.matches("return true;").count(), 1, "{text}");
}
