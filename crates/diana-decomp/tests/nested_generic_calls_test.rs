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

#[test]
fn concrete_formals_preserve_all_calls_and_the_open_sibling_argument() {
    let engine = support::engine(DEX);
    let class = support::class(DEX, &engine, "Lfixture/NestedCalls;");
    let result = engine
        .class_source(Request {
            class,
            method: None,
        })
        .unwrap();
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    let mut methods = result.text.lines().map(str::trim);
    let chain = methods
        .find(|line| line.contains("String closedChain("))
        .unwrap();
    let arguments = chain.split_once('(').unwrap().1.split_once(')').unwrap().0;
    let (text, witness) = arguments.split_once(", ").unwrap();
    let text = text.strip_prefix("String ").unwrap();
    let witness = witness.strip_prefix("Object ").unwrap();
    let nested = (0..11).fold(text.to_owned(), |value, _| {
        format!("NestedCalls.closedStep({value}, {witness})")
    });
    assert_eq!(methods.next().unwrap(), format!("return {nested};"));

    let routed = methods
        .find(|line| line.contains("List<String> routed("))
        .unwrap();
    let parameters = routed.split_once('(').unwrap().1.split_once(')').unwrap().0;
    let parameters: Vec<_> = parameters.split(", ").collect();
    let [flag, typed, raw] = parameters.as_slice() else {
        panic!("{routed}");
    };
    let flag = flag.strip_prefix("boolean ").unwrap();
    let typed = typed.strip_prefix("List<String> ").unwrap();
    let raw = raw.strip_prefix("List ").unwrap();
    let body = methods.next().unwrap();
    let open = format!(", NestedCalls.id({typed}));");
    let conditional = body
        .strip_prefix(&format!(
            "return NestedCalls.keepAfter({flag} ? NestedCalls.bounded({typed}) : "
        ))
        .and_then(|body| body.strip_suffix(&open))
        .unwrap_or_else(|| panic!("{body}"));
    assert_eq!(conditional.matches("NestedCalls.bounded(").count(), 1);
    assert!(
        conditional.contains("List<String>") && conditional.contains(raw),
        "{conditional}"
    );
}
