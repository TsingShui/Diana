mod support;

use diana_decomp::Request;

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/exceptions/recursive-throws/classes.dex");

#[test]
fn propagates_checked_effects_across_recursive_calls_and_distinct_handler_sites() {
    let engine = support::engine(DEX);
    let class = support::class(DEX, &engine, "Lfixture/RecursiveThrows;");
    let result = engine
        .class_source(Request {
            class,
            method: None,
        })
        .expect("class source");
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    let declaration = |name| {
        result
            .text
            .lines()
            .find(|line| line.contains(&format!("void {name}(")))
            .expect("original method")
    };
    for method in ["a", "b", "c", "d", "zRead"] {
        assert!(
            declaration(method).contains("throws Throwable"),
            "{}",
            declaration(method)
        );
    }
    assert!(!declaration("e").contains("throws"), "{}", declaration("e"));
}
