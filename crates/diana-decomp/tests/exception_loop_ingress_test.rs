mod support;

const FIXTURE: &[u8] =
    include_bytes!("../../../quality/fixtures/exceptions/exception-loop-ingress/classes.dex");

#[test]
fn distinct_catches_copy_their_values_before_the_shared_loop_continuation() {
    let engine = support::engine(FIXTURE);
    let class = support::class(FIXTURE, &engine, "Lfixture/ExceptionLoopIngress;");
    let facts = engine.class_facts(class).expect("class facts");
    let method = facts
        .resolve_method(
            &"run".encode_utf16().collect::<Vec<_>>(),
            &"()Ljava/lang/String;".encode_utf16().collect::<Vec<_>>(),
        )
        .expect("original method");
    let analysis = engine.analyze_method(class, method).expect("method");

    assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
    let text = analysis.rendered.text();
    assert!(text.contains("catch (IllegalArgumentException | IndexOutOfBoundsException "));
    let bounds_body = text
        .split_once("catch (IndexOutOfBoundsException ")
        .and_then(|(_, handler)| handler.split_once('{'))
        .and_then(|(_, handler)| handler.split_once('}'))
        .map(|(handler, _)| handler)
        .unwrap_or_else(|| panic!("missing bounds catch in {text}"));
    assert!(bounds_body.contains(" = "), "{text}");
    assert!(!bounds_body.contains("null"), "{text}");
}
