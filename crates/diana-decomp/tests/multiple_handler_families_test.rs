mod support;

const FIXTURE: &[u8] =
    include_bytes!("../../../quality/fixtures/exceptions/multiple-handler-families/classes.dex");

#[test]
fn shared_protection_recovers_after_distinct_handler_paths_and_on_loop_reentry() {
    let engine = support::engine(FIXTURE);
    let class = support::class(FIXTURE, &engine, "Lfixture/MultipleHandlerFamilies;");
    let facts = engine.class_facts(class).expect("class facts");
    for (name, descriptor) in [("run", "()V"), ("runRepeated", "(I)V")] {
        let method = facts
            .resolve_method(
                &name.encode_utf16().collect::<Vec<_>>(),
                &descriptor.encode_utf16().collect::<Vec<_>>(),
            )
            .expect("original method");
        let analysis = engine.analyze_method(class, method).expect("method");
        assert!(
            analysis.failures.is_empty(),
            "{name}: {:?}",
            analysis.failures
        );

        let text = analysis.rendered.text();
        let start = text.find(".start();").expect("first protected operation");
        let retry = text.find(".retry();").expect("second protected operation");
        let wait = text.find(".await();").expect("shared protected operation");
        assert!(start < retry && retry < wait, "{text}");
        assert_eq!(text.matches(".await();").count(), 1, "{text}");
        assert!(text.contains("catch (InterruptedException "), "{text}");
    }
}
