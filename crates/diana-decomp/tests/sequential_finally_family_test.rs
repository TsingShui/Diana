mod support;

const SEQUENTIAL: &[u8] =
    include_bytes!("../../../quality/fixtures/exceptions/sequential-finally-family/classes.dex");
const TYPED: &[u8] = include_bytes!(
    "../../../quality/fixtures/exceptions/sequential-finally-family-typed/classes.dex"
);
const OUTWARD_STATE: &[u8] = include_bytes!(
    "../../../quality/fixtures/exceptions/sequential-finally-outward-state/classes.dex"
);

#[test]
fn split_cleanup_captures_have_one_finally() {
    for (fixture, descriptor, scalar_parameters) in [
        (SEQUENTIAL, "Lfixture/FinallyFamily;", "I"),
        (TYPED, "Lfixture/FinallyTyped;", "I"),
        (OUTWARD_STATE, "Lfixture/FinallyOutwardState;", "II"),
    ] {
        let engine = support::engine(fixture);
        let class = support::class(fixture, &engine, descriptor);
        let facts = engine.class_facts(class).expect("class facts");
        let prototype = format!("({descriptor}{scalar_parameters})V")
            .encode_utf16()
            .collect::<Vec<_>>();
        let method = facts
            .resolve_method(&"run".encode_utf16().collect::<Vec<_>>(), &prototype)
            .expect("original method");
        let analysis = engine.analyze_method(class, method).expect("method");
        assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
        let text = analysis.rendered.text();
        let (body, cleanup) = text.split_once("} finally {").expect(text);
        assert!(!cleanup.contains("} finally {"), "{text}");
        assert!(!body.contains(".cleanup("), "{text}");
        assert_eq!(cleanup.matches(".cleanup(").count(), 1, "{text}");
        let first = body.find(".stage(1);").expect(text);
        let second = body.find(".stage(2);").expect(text);
        let third = body.find(".stage(3);").expect(text);
        assert!(first < second && second < third, "{text}");
        if descriptor == "Lfixture/FinallyTyped;" {
            assert!(
                body[second..third].contains("catch (IllegalArgumentException "),
                "{text}"
            );
        }
        if descriptor == "Lfixture/FinallyOutwardState;" {
            assert!(cleanup.contains("catch (Throwable "), "{text}");
            assert!(cleanup.contains(".outward("), "{text}");
        }
    }
}
