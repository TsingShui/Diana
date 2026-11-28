mod support;

fn body(dex: &[u8], descriptor: &str, name: &str, prototype: &str) -> String {
    let engine = support::engine(dex);
    let class = support::class(dex, &engine, descriptor);
    let facts = engine.class_facts(class).expect("original class facts");
    let method = facts
        .resolve_method(
            &name.encode_utf16().collect::<Vec<_>>(),
            &prototype.encode_utf16().collect::<Vec<_>>(),
        )
        .expect("original method identity");
    let result = engine
        .analyze_method(class, method)
        .expect("selected loop recovery");
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    result.rendered.text().to_owned()
}

#[test]
fn guarded_continue_exposes_the_counted_update() {
    let text = body(
        include_bytes!("../../../quality/fixtures/loops/loop-for/classes.dex"),
        "Lfixture/LoopFor;",
        "stopAt",
        "([II)I",
    );
    assert!(text.contains("for (int ") && text.contains("++)"), "{text}");
    assert!(
        text.contains("LoopFor.state = ") && text.contains("break;"),
        "{text}"
    );
    assert!(!text.contains("continue;"), "{text}");
}

#[test]
fn counted_update_requires_a_complete_and_effect_free_backedge() {
    const DEX: &[u8] =
        include_bytes!("../../../quality/fixtures/loops/counted-continue-update/classes.dex");
    const OWNER: &str = "Lfixture/CountedContinueUpdate;";
    let recovered = body(
        DEX,
        OWNER,
        "advanceThenStop",
        "([IILjava/lang/ArithmeticException;)I",
    );
    assert!(
        recovered.contains("for (int ") && recovered.contains("++)"),
        "{recovered}"
    );
    assert!(
        recovered.contains("CountedContinueUpdate.record("),
        "{recovered}"
    );
    for (method, prototype) in [
        ("holdOnZero", "([I)I"),
        (
            "observeAfterAdvance",
            "([ILjava/lang/ArithmeticException;)I",
        ),
    ] {
        let text = body(DEX, OWNER, method, prototype);
        assert!(
            text.contains("while (") && text.contains("continue;"),
            "{text}"
        );
        assert!(!text.contains("for ("), "{text}");
    }
}
