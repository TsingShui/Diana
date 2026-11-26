mod support;

use diana_decomp::{MethodSelector, Request};

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/branches/protected-branch-joins/classes.dex");

fn recovered(name: &str) -> String {
    recovered_in("Lfixture/ProtectedJoins;", name)
}

fn recovered_in(descriptor: &str, name: &str) -> String {
    let engine = support::engine(DEX);
    let class = support::class(DEX, &engine, descriptor);
    let method = MethodSelector {
        name: name.encode_utf16().collect(),
        prototype: "(II)I".encode_utf16().collect(),
    };
    let result = engine
        .class_source(Request {
            class,
            method: Some(method),
        })
        .expect("method source");
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    result.text
}

fn source_variable(line: &str) -> &str {
    line.split_once(" = ")
        .expect("assignment")
        .0
        .split_whitespace()
        .last()
        .expect("destination variable")
}

#[test]
fn protected_arm_failure_uses_the_value_before_the_join() {
    let source = recovered("route");
    let (prefix, protected) = source.split_once("try {").expect("protected region");
    let (body, suffix) = protected
        .split_once("catch (IllegalStateException")
        .expect("typed handler");
    let initial = prefix
        .lines()
        .find(|line| line.contains("ProtectedJoins.before("))
        .expect("outside initializer");
    let saved = source_variable(initial);
    let shared = body
        .lines()
        .find(|line| line.contains("ProtectedJoins.join("))
        .expect("common join");
    let result = source_variable(shared);
    assert_ne!(result, saved);
    assert!(body.contains(&format!(
        "== 0 ? ProtectedJoins.left({saved}) : ProtectedJoins.right({saved})"
    )));
    assert!(suffix.contains(&format!("return ProtectedJoins.recover({saved});")));
    assert!(suffix.contains(&format!("return ProtectedJoins.after({result});")));
}

#[test]
fn loop_bypass_and_exception_preserve_the_last_successful_write() {
    let source = recovered("loopRoute");
    let (prefix, protected) = source.split_once("try {").expect("protected loop");
    let (body, suffix) = protected
        .split_once("catch (IllegalStateException")
        .expect("typed handler");
    let initial = prefix
        .lines()
        .find(|line| line.contains("ProtectedJoins.before("))
        .expect("outside initializer");
    let carried = source_variable(initial);
    let arm_line = body
        .lines()
        .find(|line| line.contains("ProtectedJoins.left("))
        .expect("selected arm");
    let arm = source_variable(arm_line);
    assert!(arm_line.contains(&format!(
        "& 1) == 0 ? ProtectedJoins.left({carried}) : ProtectedJoins.right({carried})"
    )));
    let (zero, nonzero) = body
        .split_once(&format!("if ({arm} == 0)"))
        .expect("join guard")
        .1
        .split_once("else")
        .expect("nonzero arm");
    assert!(zero.contains(&format!("{carried} = {arm};")));
    assert!(!zero.contains("ProtectedJoins.join("));
    let (prior, later) = nonzero
        .split_once(&format!("{carried} = ProtectedJoins.join({arm});"))
        .expect("protected join");
    assert!(prior.contains(&format!("{carried} = {arm};")));
    assert!(later.contains("--;"));
    assert!(suffix.contains(&format!("return ProtectedJoins.recover({carried});")));
    assert!(suffix.contains(&format!("return ProtectedJoins.after({carried});")));
}

#[test]
fn split_protected_ranges_merge_only_within_their_own_stage() {
    let source = recovered("wideRoute");
    let mut stages = source.split("try {");
    let prefix = stages.next().expect("initializer before the first try");
    let mut incoming = source_variable(
        prefix
            .lines()
            .find(|line| line.contains("ProtectedJoins.before("))
            .expect("outside initializer"),
    );
    for stage in 1..=6 {
        let (body, handler) = stages
            .next()
            .expect("one try per stage")
            .split_once("catch (IllegalStateException")
            .expect("stage handler");
        let (handler, continuation) = handler.split_once('}').expect("normal continuation");
        let (before, after) = body
            .split_once(&format!(
                "{incoming} = ProtectedJoins.left({incoming}) + {stage};"
            ))
            .expect("protected left and nonthrowing addition");
        assert!(!before.contains("ProtectedJoins."));
        let right = after
            .lines()
            .find(|line| line.contains(&format!(" = ProtectedJoins.right({incoming});")))
            .expect("right consumes the last successful value");
        let result = source_variable(right);
        assert!(handler.contains(&format!(
            "return ProtectedJoins.recover({incoming} - {stage});"
        )));
        let completed = continuation
            .lines()
            .find(|line| line.contains(&format!("ProtectedJoins.after({result});")))
            .expect("unprotected continuation");
        if stage < 6 {
            incoming = source_variable(completed);
        } else {
            assert!(completed.trim_start().starts_with("return "));
        }
    }
    assert!(stages.next().is_none());
}

#[test]
fn cross_word_diamonds_call_one_arm_in_bit_order_with_the_carried_value() {
    let source = recovered_in("Lfixture/CrossWordBranches;", "run");
    let mut lines = source.lines().map(str::trim);
    let signature = lines
        .find(|line| line.contains("int run("))
        .expect("signature");
    let parameters = signature
        .split_once("run(")
        .unwrap()
        .1
        .split_once(')')
        .unwrap()
        .0;
    let (key, value) = parameters.split_once(", ").expect("two parameters");
    let key = key.strip_prefix("int ").unwrap();
    let mut value = value.strip_prefix("int ").unwrap();
    for bit in 0..24 {
        let line = lines.next().expect("bit test");
        let (shifted, _) = line.split_once(" & 1) == 0").expect("low bit test");
        let guarded = shifted.rsplit_once('(').expect("guard expression").1;
        let mut shifts = guarded.split(" >>> ");
        assert_eq!(shifts.next(), Some(key));
        let distance: usize = shifts
            .map(|shift| shift.parse::<usize>().expect("logical shift"))
            .sum();
        assert_eq!(distance, bit);
        if bit < 23 {
            assert!(line.ends_with(&format!(
                "? ProtectedJoins.left({value}) : ProtectedJoins.right({value});"
            )));
            value = source_variable(line);
        } else {
            assert!(line.starts_with("if ("));
            assert_eq!(
                lines.next(),
                Some(format!("return ProtectedJoins.left({value});").as_str())
            );
            assert_eq!(lines.next(), Some("}"));
            assert_eq!(
                lines.next(),
                Some(format!("return ProtectedJoins.right({value});").as_str())
            );
        }
    }
}
