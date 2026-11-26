mod support;

use diana_decomp::{MethodSelector, Request};

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/branches/bound-value-folding/classes.dex");

fn recovered(name: &str, prototype: &str) -> String {
    let engine = support::engine(DEX);
    let class = support::class(DEX, &engine, "Lfixture/BoundValues;");
    let result = engine
        .class_source(Request {
            class,
            method: Some(MethodSelector {
                name: name.encode_utf16().collect(),
                prototype: prototype.encode_utf16().collect(),
            }),
        })
        .expect("fixture method");
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    result.text
}

fn parameters<'a>(source: &'a str, method: &str) -> Vec<&'a str> {
    source
        .split_once(&format!(" {method}("))
        .unwrap()
        .1
        .split_once(')')
        .unwrap()
        .0
        .split(", ")
        .map(|declaration| declaration.split_whitespace().last().unwrap())
        .collect()
}

// Recovery may reuse a local when the converted value crosses a branch.
fn float_value(source: &str, argument: &str) -> String {
    let cast = format!("(float) {argument}");
    let assignment = format!(" = {cast};");
    source
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("float ")?.strip_suffix(&assignment))
        .map(str::to_owned)
        .unwrap_or(cast)
}

#[test]
fn conditional_default_preserves_the_dynamic_cast() {
    let source = recovered("guarded", "(IZ)I");
    let inputs = parameters(&source, "guarded");
    let (x, reset) = (inputs[0], inputs[1]);
    assert!(source.contains(&format!("if ({reset})")));
    assert!(source.contains(&format!("{x} = 0;")));
    let (guard, calls) = source.split_once("return -1;").unwrap();
    assert!(guard.contains(&format!("if ({x} == 0)")));
    let f = float_value(calls, x);
    let (before, after) = calls
        .split_once(&format!("BoundValues.take({f}, {f})"))
        .unwrap();
    assert!(before.contains("if ("));
    assert!(after.contains(&format!("BoundValues.done({f}, {f})")));
    assert!(after.contains(&format!("return {x};")));
}

#[test]
fn independent_defaults_preserve_both_arguments_in_order() {
    let source = recovered("pair", "(IIZZ)I");
    let inputs = parameters(&source, "pair");
    let (x, y) = (inputs[0], inputs[1]);
    let (guard, calls) = source.split_once("return -1;").unwrap();
    assert!(guard.contains(&format!("if ({x} == 0 && {y} == 0)")));
    for (param, reset) in [(x, inputs[2]), (y, inputs[3])] {
        assert!(guard.contains(&format!("if ({reset})")));
        assert!(guard.contains(&format!("{param} = 0;")));
    }
    let fx = float_value(calls, x);
    let fy = float_value(calls, y);
    let (_, calls) = calls
        .split_once(&format!("BoundValues.take({fx}, {fy})"))
        .unwrap();
    let (_, calls) = calls
        .split_once(&format!("BoundValues.done({fx}, {fy})"))
        .unwrap();
    let (_, remaining) = calls
        .split_once(&format!("BoundValues.keep({x}, {y})"))
        .unwrap();
    assert!(remaining.contains(&format!("return {x} + {y};")));
}

#[test]
fn conditional_clear_preserves_catch_binding_and_runtime_type() {
    let source = recovered("caughtDefault", "(ZZ)I");
    let inputs = parameters(&source, "caughtDefault");
    let (normal, handler) = source.split_once("catch (RuntimeException ").unwrap();
    assert!(normal.contains("BoundValues.mayThrow();"));
    assert!(normal.contains("return -1;"));
    let (exception, body) = handler.split_once(") {").unwrap();
    let (clear, after) = body.split_once(&format!("{exception} = null;")).unwrap();
    assert!(clear.contains(&format!("if ({})", inputs[0])));
    let (repeat, derived) = after
        .split_once(&format!("BoundValues.replacement({exception});"))
        .unwrap();
    assert!(repeat.contains(&format!("if ({})", inputs[1])));
    let expr = format!(" = {exception} instanceof IllegalStateException ? 1 : 0;");
    let result = derived
        .lines()
        .find_map(|line| line.trim().strip_prefix("int ")?.strip_suffix(&expr))
        .unwrap();
    let use_result = format!("BoundValues.keep({result}, 5);");
    let (repeat, after_first) = derived.split_once(&use_result).unwrap();
    assert!(repeat.contains(&format!("if ({})", inputs[1])));
    let (_, tail) = after_first.split_once(&use_result).unwrap();
    assert!(tail.contains(&format!("BoundValues.replacement({exception});")));
    assert!(tail.contains(&format!("return {result};")));
}

#[test]
fn closed_local_still_folds_to_its_float_literal() {
    let source = recovered("localConstant", "(Z)V");
    assert_eq!(source.matches("BoundValues.done(37.0f, 37.0f);").count(), 2);
}
