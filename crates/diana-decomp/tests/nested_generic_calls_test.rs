mod support;

use diana_decomp::Request;

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/generics/nested-call-inference/classes.dex");

#[test]
fn nested_identity_calls_and_both_bounded_branches_keep_the_selected_arguments() {
    let engine = support::engine(DEX);
    let class = support::class(DEX, &engine, "Lfixture/NestedCalls;");
    let result = engine
        .class_source(Request {
            class,
            method: None,
        })
        .unwrap();
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    let mut lines = result.text.lines().map(str::trim);
    let chain = lines
        .find(|line| line.contains("String chain14(String "))
        .unwrap();
    let (_, parameter) = chain.split_once("chain14(String ").unwrap();
    let (parameter, _) = parameter.split_once(')').unwrap();
    let expected = format!(
        "return {}{parameter}{};",
        "NestedCalls.id(".repeat(14),
        ")".repeat(14)
    );
    assert_eq!(lines.next(), Some(expected.as_str()));

    let choose = lines.find(|line| line.contains("List<?> choose(")).unwrap();
    let (_, parameters) = choose.split_once("choose(").unwrap();
    let (parameters, _) = parameters.split_once(')').unwrap();
    let names: Vec<_> = parameters
        .split(", ")
        .map(|parameter| parameter.rsplit_once(' ').unwrap().1)
        .collect();
    let [flag, typed, raw] = names.as_slice() else {
        panic!("{choose}");
    };
    let body = lines.next().unwrap();
    assert!(
        body.starts_with(&format!(
            "return NestedCalls.id({flag} ? NestedCalls.bounded({typed}) : "
        )),
        "{body}"
    );
    let (_, false_branch) = body.split_once(" : ").unwrap();
    assert_eq!(
        false_branch.matches("NestedCalls.bounded(").count(),
        1,
        "{body}"
    );
    assert!(
        false_branch.contains("List<String>") && false_branch.contains(raw),
        "{body}"
    );
}
