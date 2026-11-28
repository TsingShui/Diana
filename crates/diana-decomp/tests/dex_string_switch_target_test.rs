mod support;

use diana_decomp::Request;

#[test]
fn original_dex_string_invocations_still_recover_compact_switches() {
    let dex = include_bytes!("../../../quality/fixtures/branches/branch-switch-string/classes.dex");
    let engine = support::engine(dex);
    let class = support::class(dex, &engine, "Lfixture/BranchSwitchString;");
    let result = engine
        .analyze(Request {
            class,
            method: None,
        })
        .expect("DEX string switch recovery");
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    let text = result.rendered.text();
    assert_eq!(text.matches("switch (").count(), 2, "{text}");
    for label in ["a", "b", "c", "add", "sub", "mul"] {
        assert!(text.contains(&format!("case \"{label}\":")), "{text}");
    }
}
