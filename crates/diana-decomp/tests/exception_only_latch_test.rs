mod support;

use diana_decomp::{MethodSelector, Request};

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/loops/exception-only-latch/classes.dex");

#[test]
fn only_the_caught_failure_retries_the_loop() {
    let engine = support::engine(DEX);
    let class = support::class(DEX, &engine, "Lfixture/LoopOwners;");
    let source = engine
        .class_source(Request {
            class,
            method: Some(MethodSelector {
                name: "dispatch".encode_utf16().collect(),
                prototype: "(II)I".encode_utf16().collect(),
            }),
        })
        .expect("method source");
    assert!(source.failures.is_empty(), "{:?}", source.failures);
    let mut lines = source.text.lines().map(str::trim);
    let declaration = lines.find(|line| line.contains("int dispatch(")).unwrap();
    let parameters = declaration
        .split_once('(')
        .unwrap()
        .1
        .split_once(')')
        .unwrap()
        .0;
    let (value, mode) = parameters.split_once(", ").unwrap();
    let value = value.strip_prefix("int ").unwrap();
    let mode = mode.strip_prefix("int ").unwrap();
    for expected in [
        format!("while ({value} > 0) {{"),
        "try {".to_owned(),
        format!("switch (LoopOwners.choose({value}, {mode})) {{"),
        "case 0:".to_owned(),
        "case 1:".to_owned(),
        format!("return LoopOwners.second(LoopOwners.first({value}));"),
        "case 2:".to_owned(),
        format!("return LoopOwners.alternate({value});"),
        "default:".to_owned(),
        format!("return LoopOwners.stop({value});"),
        "}".to_owned(),
    ] {
        assert_eq!(lines.next(), Some(expected.as_str()));
    }
    assert!(lines
        .next()
        .unwrap()
        .starts_with("} catch (IllegalStateException "));
    assert_eq!(
        lines.next(),
        Some(format!("LoopOwners.retry({value});").as_str())
    );
    assert_eq!(lines.next(), Some(format!("{value}--;").as_str()));
    assert_eq!(lines.next(), Some("}"));
    assert_eq!(lines.next(), Some("}"));
    assert_eq!(
        lines.next(),
        Some(format!("return LoopOwners.empty({value});").as_str())
    );
    assert_eq!(lines.next(), Some("}"));
}
