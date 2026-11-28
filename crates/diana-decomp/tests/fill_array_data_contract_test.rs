mod support;

const FIXTURE: &[u8] =
    include_bytes!("../../../quality/fixtures/atoms/fill-array-data-contract/classes.dex");

fn recover(name: &str, descriptor: &str) -> String {
    let engine = support::engine(FIXTURE);
    let class = support::class(FIXTURE, &engine, "Lfixture/FillData;");
    let facts = engine.class_facts(class).expect("class facts");
    let method = facts
        .resolve_method(
            &name.encode_utf16().collect::<Vec<_>>(),
            &descriptor.encode_utf16().collect::<Vec<_>>(),
        )
        .expect("original fill method");
    let analysis = engine
        .analyze_method(class, method)
        .expect("method analysis");
    assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
    analysis.rendered.text().to_owned()
}

#[test]
fn protected_fill_captures_once_and_checks_extent_before_stores() {
    let text = recover("protectedFill", "([ILjava/lang/ArithmeticException;)I");
    let receiver = "FillData.receiverMaybe(";
    assert_eq!(text.matches(receiver).count(), 1, "{text}");
    let mut remaining = text.as_str();
    for marker in [
        "try {",
        receiver,
        ".length < 2",
        "throw new ArrayIndexOutOfBoundsException(",
        "[0] = 11;",
        "[1] = 22;",
        "catch (ArrayIndexOutOfBoundsException ",
        "catch (ArithmeticException ",
    ] {
        remaining = remaining
            .split_once(marker)
            .unwrap_or_else(|| panic!("missing {marker}: {text}"))
            .1;
    }
}

#[test]
fn empty_and_typed_payloads_preserve_their_observable_values() {
    let empty = recover("fillEmpty", "([I)V");
    assert!(empty.contains(".length;"), "{empty}");

    let floats = recover("fillFloatBits", "([F)V");
    assert!(floats.contains("[0] = 0.0f;"), "{floats}");
    assert!(floats.contains("[1] = -0.0f;"), "{floats}");
    assert!(
        floats.contains("Float.intBitsToFloat(2143363909)")
            || floats.contains("Float.intBitsToFloat(0x7fc12345)"),
        "{floats}"
    );
}
