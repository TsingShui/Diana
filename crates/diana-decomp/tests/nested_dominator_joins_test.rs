mod support;

use diana_decomp::{MethodSelector, Request};

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/branches/nested-dominator-joins/classes.dex");

#[test]
fn each_nested_choice_and_bypass_preserves_the_value_entering_the_next_join() {
    let engine = support::engine(DEX);
    let class = support::class(DEX, &engine, "Lfixture/NestedJoins;");
    let result = engine
        .class_source(Request {
            class,
            method: Some(MethodSelector {
                name: "run".encode_utf16().collect(),
                prototype: "(II)I".encode_utf16().collect(),
            }),
        })
        .expect("method source");
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    let mut lines = result.text.lines().map(str::trim);
    let signature = lines
        .find(|line| line.contains("int run("))
        .expect("method signature");
    let parameters = signature
        .split_once('(')
        .unwrap()
        .1
        .split_once(')')
        .unwrap()
        .0;
    let (key, carried) = parameters.split_once(", ").unwrap();
    let mut key = key.strip_prefix("int ").unwrap().to_owned();
    let carried = carried.strip_prefix("int ").unwrap();
    let join = format!("{carried} = NestedJoins.join({carried});");
    let finish = format!("return NestedJoins.finish({carried});");
    for stage in 0..5 {
        let arm = format!("if (({key} & 1) != 0) {{");
        let select = format!("if (({key} & 2) == 0) {{");
        assert_eq!(lines.next(), Some(arm.as_str()));
        assert_eq!(lines.next(), Some(select.as_str()));
        assert_eq!(
            lines.next().unwrap(),
            format!("{carried} = NestedJoins.left({carried});")
        );
        assert_eq!(lines.next(), Some("} else {"));
        assert_eq!(
            lines.next().unwrap(),
            format!("{carried} = NestedJoins.right({carried});")
        );
        assert_eq!(lines.next(), Some("}"));
        assert_eq!(lines.next(), Some("}"));
        if stage == 4 {
            assert_eq!(
                lines.next().unwrap(),
                format!("if (({key} & 1) != 0 || ({key} & 4) == 0) {{")
            );
            assert_eq!(lines.next(), Some(join.as_str()));
            assert_eq!(lines.next(), Some("}"));
            assert_eq!(lines.next(), Some(finish.as_str()));
        } else {
            assert_eq!(
                lines.next().unwrap(),
                format!("if (({key} & 1) == 0 && ({key} & 4) != 0) {{")
            );
            assert_eq!(lines.next(), Some(finish.as_str()));
            assert_eq!(lines.next(), Some("}"));
            assert_eq!(lines.next(), Some(join.as_str()));
            key.push_str(" >>> 3");
        }
    }
    assert_eq!(lines.next(), Some("}"));
}
