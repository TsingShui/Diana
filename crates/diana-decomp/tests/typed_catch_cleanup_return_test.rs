mod support;

const FIXTURE: &[u8] =
    include_bytes!("../../../quality/fixtures/exceptions/typed-catch-cleanup-return/classes.dex");
const CHALLENGE: &[u8] = include_bytes!(
    "../../../quality/fixtures/exceptions/typed-catch-cleanup-return-late-effect/classes.dex"
);

#[test]
fn typed_catch_retains_its_return_across_cleanup_and_monitor_release() {
    for (fixture, fault_arguments) in [(FIXTURE, 3), (CHALLENGE, 4)] {
        let engine = support::engine(fixture);
        let class = support::class(fixture, &engine, "Lfixture/ReturnLanes;");
        let facts = engine.class_facts(class).expect("class facts");
        let descriptor = format!(
            "(Ljava/lang/Object;{})Ljava/lang/Object;",
            "Ljava/lang/RuntimeException;".repeat(fault_arguments),
        );
        let method = facts
            .resolve_method(
                &"lookup".encode_utf16().collect::<Vec<_>>(),
                &descriptor.encode_utf16().collect::<Vec<_>>(),
            )
            .expect("original method");
        let analysis = engine.analyze_method(class, method).expect("method");
        assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
        let text = analysis.rendered.text();
        let (body, handler) = text.split_once("catch (Exception ").expect(text);
        let (handler, cleanup) = handler.split_once("} finally {").expect(text);
        assert!(body.contains("synchronized (ReturnLanes.class)"), "{text}");
        assert!(handler.contains("return null;"), "{text}");
        assert!(cleanup.contains("ReturnLanes.markThenThrow(5, "), "{text}");
        assert_eq!(text.matches("ReturnLanes.markThenThrow(5, ").count(), 1);
    }
}
