mod support;

const FIXTURE: &[u8] =
    include_bytes!("../../../quality/fixtures/exceptions/exception-join-types/classes.dex");

#[test]
fn exception_producers_type_the_loop_join_at_distinct_and_shared_entries() {
    let engine = support::engine(FIXTURE);
    let class = support::class(FIXTURE, &engine, "Lfixture/ExceptionJoinTypes;");
    let facts = engine.class_facts(class).expect("class facts");
    for (name, catches) in [
        (
            "run",
            &["catch (IOException ", "catch (RuntimeException "][..],
        ),
        ("runShared", &["catch (IOException | RuntimeException "][..]),
    ] {
        let method = facts
            .resolve_method(
                &name.encode_utf16().collect::<Vec<_>>(),
                &"()V".encode_utf16().collect::<Vec<_>>(),
            )
            .expect("original method");
        let analysis = engine.analyze_method(class, method).expect("method");
        assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
        let text = analysis.rendered.text();
        for clause in catches {
            assert!(text.contains(clause), "{text}");
        }
        assert!(text.contains("throw (IOException) "), "{text}");
        assert!(text.contains("throw (RuntimeException) "), "{text}");
    }
}
