mod support;

const FIXTURE: &[u8] =
    include_bytes!("../../../quality/fixtures/branches/guarded-effect-join/classes.dex");

#[test]
fn guarded_effect_materializes_before_the_shared_publication() {
    let engine = support::engine(FIXTURE);
    let class = support::class(FIXTURE, &engine, "Lfixture/GuardedEffectJoin;");
    let facts = engine.class_facts(class).expect("class facts");
    let prototype = "(ZZ)Ljava/lang/Object;".encode_utf16().collect::<Vec<_>>();
    for name in ["run", "runWithCleanup"] {
        let method = facts
            .resolve_method(&name.encode_utf16().collect::<Vec<_>>(), &prototype)
            .expect("guarded method identity");
        let analysis = engine.analyze_method(class, method).expect("method");
        assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
        let text = analysis.rendered.text();
        let prepare = text.find("GuardedEffectJoin.prepare()").expect(text);
        let publish = text.find("GuardedEffectJoin.publish(").expect(text);
        let consume = text.find("GuardedEffectJoin.consume(").expect(text);
        assert!(prepare < publish && publish < consume, "{text}");
        assert_eq!(
            text.matches("GuardedEffectJoin.prepare()").count(),
            1,
            "{text}"
        );
    }
}
