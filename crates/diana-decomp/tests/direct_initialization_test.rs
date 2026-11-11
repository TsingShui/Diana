mod support;

use diana_decomp::{MethodFailure, SingleMethodAnalysis};

const DIRECT: &[u8] =
    include_bytes!("../../../quality/fixtures/classes/direct-initialization/classes.dex");
const EFFECTS: &[u8] = include_bytes!(
    "../../../quality/fixtures/exceptions/direct-initialization-effects/classes.dex"
);

fn method(bytes: &[u8], owner: &str, name: &str, prototype: &str) -> SingleMethodAnalysis {
    let engine = support::engine(bytes);
    let class = support::class(bytes, &engine, owner);
    let facts = engine.class_facts(class).expect("class facts");
    let method = facts
        .resolve_method(
            &name.encode_utf16().collect::<Vec<_>>(),
            &prototype.encode_utf16().collect::<Vec<_>>(),
        )
        .expect("original method identity");
    engine
        .analyze_method(class, method)
        .expect("method analysis")
}

fn allocation_name(text: &str) -> &str {
    let line = text
        .lines()
        .find(|line| line.contains(" = allocate-instance Child;"))
        .expect("original Child allocation");
    line.split(" = ")
        .next()
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap()
}

fn no_failures(failures: &[MethodFailure]) {
    assert!(
        failures.is_empty(),
        "unexpected recovery failures: {failures:?}"
    );
}

#[test]
fn preserves_different_allocation_and_constructor_owners() {
    let analysis = method(
        DIRECT,
        "Lfixture/DirectInitialization;",
        "direct",
        "(I)Lfixture/Child;",
    );
    no_failures(&analysis.failures);
    let text = analysis.rendered.text();
    let name = allocation_name(text);
    assert!(
        text.contains(&format!(
            "init-direct {name}, Lfixture/Parent;-><init>(I)V, ["
        )),
        "{text}"
    );
    assert!(text.contains(&format!("return {name};")), "{text}");
    assert!(
        !text.contains("new Child("),
        "the Child constructor would overwrite value with 99: {text}"
    );
}

#[test]
fn ordinary_construction_keeps_natural_syntax() {
    let analysis = method(
        DIRECT,
        "Lfixture/DirectInitialization;",
        "ordinary",
        "(I)Lfixture/Child;",
    );
    no_failures(&analysis.failures);
    let text = analysis.rendered.text();
    assert!(text.contains("new Child("), "{text}");
    assert!(!text.contains("init-direct"), "{text}");
}

#[test]
fn retains_alias_effect_order_and_handler_boundaries() {
    let analysis = method(
        EFFECTS,
        "Lfixture/DirectInitializationEffects;",
        "build",
        "(I)Lfixture/Child;",
    );
    no_failures(&analysis.failures);
    let text = analysis.rendered.text();
    let name = allocation_name(text);
    let operations = [
        "try {".to_owned(),
        "allocate-instance Child".to_owned(),
        "DirectInitializationEffects.event(1)".to_owned(),
        "if (i < 0)".to_owned(),
        "DirectInitializationEffects.event(3)".to_owned(),
        format!("init-direct {name}, Lfixture/Parent;-><init>(I)V"),
        "DirectInitializationEffects.event(4)".to_owned(),
        format!("DirectInitializationEffects.last = {name}"),
        format!("return {name}"),
        "catch (RuntimeException".to_owned(),
        "DirectInitializationEffects.event(5)".to_owned(),
        "throw exception".to_owned(),
    ];
    let mut at = 0;
    for operation in operations {
        at += text[at..]
            .find(&operation)
            .unwrap_or_else(|| panic!("missing ordered {operation}: {text}"))
            + operation.len();
    }
    assert_eq!(text.matches("allocate-instance Child").count(), 1, "{text}");
}

#[test]
fn missing_constructor_remains_a_method_local_failure() {
    let analysis = method(
        DIRECT,
        "Lfixture/DirectInitialization;",
        "broken",
        "()Lfixture/Child;",
    );
    assert_eq!(analysis.failures.len(), 1);
    assert_eq!(analysis.failures[0].name, "broken");
    assert!(analysis.failures[0].reason.contains("has no constructor"));
    let text = analysis.rendered.text();
    assert!(text.contains("// Diana: WARNING: Failed to decompile this method."));
    assert!(text.contains("https://github.com/TsingShui/Diana/issues"));
    assert!(!text.contains("throw "), "invented exception: {text}");
}
