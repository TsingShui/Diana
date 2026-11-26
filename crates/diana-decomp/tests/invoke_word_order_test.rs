mod support;

use diana_decomp::{ClassSource, Request};

const DEX: &[u8] = include_bytes!("../../../quality/fixtures/atoms/invoke-word-order/classes.dex");

fn retrieve(bytes: &[u8]) -> ClassSource {
    let engine = support::engine(bytes);
    let class = support::class(bytes, &engine, "Lfixture/WordOrder;");
    engine
        .class_source(Request {
            class,
            method: None,
        })
        .unwrap()
}

#[test]
fn list_range_and_signature_polymorphic_uses_keep_the_original_words_and_call_order() {
    let result = retrieve(DEX);
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    let mut lines = result.text.lines().map(str::trim);
    let signature = lines.find(|line| line.contains("int run(")).unwrap();
    let (_, parameters) = signature.split_once("run(").unwrap();
    let (parameters, _) = parameters.split_once(')').unwrap();
    let parameters: Vec<_> = parameters
        .split(", ")
        .map(|p| p.rsplit_once(' ').unwrap().1)
        .collect();
    let [double, long, integer, handle] = parameters.as_slice() else {
        panic!("{signature}")
    };
    let mut assigned_call = |expected: String| {
        let line = lines.next().unwrap();
        let (declaration, expression) = line.split_once(" = ").unwrap();
        assert_eq!(expression, expected);
        declaration.strip_prefix("int ").unwrap()
    };
    let left = assigned_call(format!("WordOrder.take({long}, {integer});"));
    let middle = assigned_call(format!("WordOrder.take({double}, {long}, {integer});"));
    let right = assigned_call(format!("{handle}.invokeExact({long});"));
    assert_eq!(
        lines.next(),
        Some(format!("return WordOrder.finish({left}, {right}, new int[] {{ {left}, {middle}, {right} }});").as_str())
    );
}

#[test]
fn damaged_uses_preserve_word_count_first_reference_slot_and_original_pc() {
    // List-form invoke followed by move-result and a five-word range invoke.
    let prefix = [0x71, 0x30, 3, 0, 0x87, 9, 0x0a, 0, 0x77, 5];
    let mut starts = DEX
        .windows(prefix.len())
        .enumerate()
        .filter(|(_, bytes)| *bytes == prefix);
    let start = starts.next().unwrap().0;
    assert!(starts.next().is_none());
    for (patches, stage, reason) in [
        (
            &[(9, &[4][..])][..],
            "semantic recovery",
            "invoke at 4 has 4 arguments, expected 3",
        ),
        (
            &[(2, &[255, 255][..]), (26, &[255, 255][..])],
            "DEX decoding",
            "reference Method(MethodId(65535)) at CodeUnitOffset(0)",
        ),
        (
            &[(22, &[254, 255][..]), (26, &[255, 255][..])],
            "DEX decoding",
            "reference Method(MethodId(65534)) at CodeUnitOffset(10)",
        ),
        (
            &[(26, &[255, 255][..])],
            "DEX decoding",
            "reference Proto(ProtoId(65535)) at CodeUnitOffset(10)",
        ),
    ] {
        let mut bytes = DEX.to_vec();
        for &(offset, value) in patches {
            bytes[start + offset..start + offset + value.len()].copy_from_slice(value);
        }
        let result = retrieve(&bytes);
        let [failure] = result.failures.as_slice() else {
            panic!("{:?}", result.failures)
        };
        assert_eq!(failure.name, "run");
        assert_eq!(failure.stage, stage);
        assert!(failure.reason.contains(reason), "{}", failure.reason);
        assert_eq!(result.text.matches("native int ").count(), 3);
    }
}
