mod support;

const BASIC: &[u8] =
    include_bytes!("../../../quality/fixtures/branches/guarded-boolean-return/classes.dex");
const CHALLENGE: &[u8] =
    include_bytes!("../../../quality/fixtures/branches/nested-guarded-boolean-return/classes.dex");

fn recover(input: &[u8], descriptor: &str, name: &str) -> String {
    let engine = support::engine(input);
    let class = support::class(input, &engine, descriptor);
    let facts = engine.class_facts(class).expect("class facts");
    let method = facts
        .resolve_method(
            &name.encode_utf16().collect::<Vec<_>>(),
            &"()Z".encode_utf16().collect::<Vec<_>>(),
        )
        .expect("original boolean method");
    let analysis = engine.analyze_method(class, method).expect("method");
    assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
    analysis.rendered.text().to_owned()
}

#[test]
fn terminal_guards_become_one_short_circuit_return() {
    let text = recover(BASIC, "Lfixture/GuardedBoolean;", "allowed");
    assert!(
        text.contains("return BooleanEffects.first() || !BooleanEffects.second();"),
        "{text}"
    );
    assert_eq!(text.matches("return ").count(), 1, "{text}");
}

#[test]
fn nested_predicates_keep_their_grouping_and_order() {
    let text = recover(CHALLENGE, "Lfixture/MixedBoolean;", "mixed");
    assert!(
        text.contains(concat!(
            "return BooleanEffects.first() && (!BooleanEffects.second() || ",
            "BooleanEffects.third()) || !BooleanEffects.fourth();"
        )),
        "{text}"
    );
    assert_eq!(text.matches("return ").count(), 1, "{text}");
}

#[test]
fn effectful_arm_stays_a_statement_before_the_independent_suffix() {
    let text = recover(CHALLENGE, "Lfixture/MixedBoolean;", "effectful");
    let guard = text.find("if (BooleanEffects.first())").expect(&text);
    let marker = text.find("BooleanEffects.mark();").expect(&text);
    let early_return = text.find("return true;").expect(&text);
    let fallback = text.find("return !BooleanEffects.second();").expect(&text);
    assert!(
        guard < marker && marker < early_return && early_return < fallback,
        "{text}"
    );
}
