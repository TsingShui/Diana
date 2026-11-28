mod support;

const FIXTURE: &[u8] =
    include_bytes!("../../../quality/fixtures/branches/floating-condition-negation/classes.dex");

#[test]
fn joined_floating_branches_keep_the_unordered_guard() {
    let engine = support::engine(FIXTURE);
    let class = support::class(FIXTURE, &engine, "Lfixture/BranchFloat;");
    let facts = engine.class_facts(class).expect("class facts");
    for (name, descriptor, relation) in [
        ("fJoinFalls", "(FF)V", " < "),
        ("fJoinBoolean", "(FFZ)V", " < "),
        ("dJoinFalls", "(DD)V", " > "),
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
        let (_, guard) = text
            .split_once("if (!(")
            .unwrap_or_else(|| panic!("{name} loses the floating negation: {text}"));
        let (condition, arm) = guard
            .split_once(")) {")
            .unwrap_or_else(|| panic!("{name} has no unordered guard: {text}"));
        assert!(condition.contains(relation), "{name}: {text}");
        assert!(
            arm.trim_start().starts_with("BranchFloat.mark(9);"),
            "{name}: {text}"
        );
    }
}
