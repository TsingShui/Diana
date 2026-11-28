mod support;

const FIXTURE: &[u8] =
    include_bytes!("../../../quality/fixtures/atoms/float-comparison-effects/classes.dex");

#[test]
fn floating_comparisons_keep_one_ordered_call_per_operand() {
    let engine = support::engine(FIXTURE);
    let class = support::class(FIXTURE, &engine, "Lfixture/FloatCompare;");
    let facts = engine.class_facts(class).expect("class facts");
    let fault_predicate = "(Ljava/lang/RuntimeException;)Z";
    for (name, descriptor, operands) in [
        ("one", "()Z", ["left", "right"]),
        ("doubleFinite", "()Z", ["doubleLeft", "doubleRightMaybe"]),
        ("nanSkips", fault_predicate, ["leftNaN", "rightMaybe"]),
        (
            "doubleLow",
            fault_predicate,
            ["doubleLeft", "doubleRightMaybe"],
        ),
    ] {
        let method = facts
            .resolve_method(
                &name.encode_utf16().collect::<Vec<_>>(),
                &descriptor.encode_utf16().collect::<Vec<_>>(),
            )
            .expect("original comparison method");
        let analysis = engine
            .analyze_method(class, method)
            .expect("method analysis");
        assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
        let text = analysis.rendered.text();
        let [left, right] = operands.map(|operand| {
            let call = format!("FloatCompare.{operand}(");
            let mut positions = text.match_indices(&call);
            let (first, _) = positions.next().unwrap_or_else(|| panic!("{name}: {text}"));
            assert!(
                positions.next().is_none(),
                "{name} repeats {operand}: {text}"
            );
            first
        });
        assert!(left < right, "{name} reorders the producers: {text}");
    }
}
