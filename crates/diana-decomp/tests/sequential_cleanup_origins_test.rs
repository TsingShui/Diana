mod support;

const SEQUENTIAL: &[u8] =
    include_bytes!("../../../quality/fixtures/exceptions/sequential-cleanup-origins/classes.dex");
const ORDERED: &[u8] =
    include_bytes!("../../../quality/fixtures/exceptions/ordered-cleanup-arguments/classes.dex");

fn rendered(bytes: &[u8], owner: &str, name: &str) -> String {
    let engine = support::engine(bytes);
    let class = support::class(bytes, &engine, owner);
    let facts = engine.class_facts(class).expect("fixture class");
    let method = facts
        .resolve_method(
            &name.encode_utf16().collect::<Vec<_>>(),
            &"(II)I".encode_utf16().collect::<Vec<_>>(),
        )
        .expect("original method");
    let analysis = engine
        .analyze_method(class, method)
        .expect("recover method");
    assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
    analysis.rendered.text().to_owned()
}

fn parameters_and_body<'a>(text: &'a str, name: &str) -> (Vec<&'a str>, &'a str) {
    let (_, declaration) = text
        .split_once(&format!("int {name}("))
        .expect("method declaration");
    let (parameters, body) = declaration.split_once(')').expect("two parameters");
    let params: Vec<_> = parameters
        .split(", ")
        .map(|p| p.strip_prefix("int ").unwrap())
        .collect();
    assert_eq!(params.len(), 2);
    assert_ne!(params[0], params[1]);
    (params, body)
}

#[test]
fn sequential_cleanup_keeps_each_regions_parameter() {
    let text = rendered(
        SEQUENTIAL,
        "Lfixture/SequentialCleanupOrigins;",
        "closeBoth",
    );
    let (params, body) = parameters_and_body(&text, "closeBoth");
    let regions: Vec<_> = body.split("try {").skip(1).collect();
    assert_eq!(regions.len(), 2, "{text}");
    for (region, parameter) in regions.into_iter().zip(params) {
        let (protected, cleanup) = region.split_once("} finally {").expect("finally");
        assert!(protected.contains(&format!(".read({parameter})")), "{text}");
        assert_eq!(protected.matches(".read(").count(), 1, "{text}");
        assert!(cleanup.contains(&format!(".close({parameter})")), "{text}");
        assert_eq!(cleanup.matches(".close(").count(), 1, "{text}");
    }
}

// These fixtures pass only parameter aliases to close. Follow simple move
// assignments so the check survives register coalescing and local renaming.
fn arguments_at_close<'a>(prefix: &'a str, location: &'a str) -> Vec<&'a str> {
    let (_, args) = location.split_once(".close(").expect("cleanup call");
    let (args, _) = args.split_once(')').expect("cleanup arguments");
    args.split(", ")
        .map(|mut value| {
            for line in prefix.lines().rev() {
                if let Some((_, rhs)) = line
                    .trim()
                    .trim_end_matches(';')
                    .split_once(" = ")
                    .filter(|(lhs, _)| lhs.split_whitespace().last() == Some(value))
                {
                    value = rhs;
                }
            }
            value
        })
        .collect()
}

fn check_distinct_cleanups(bytes: &[u8], owner: &str, method: &str, pair: bool) {
    let text = rendered(bytes, owner, method);
    let (params, body) = parameters_and_body(&text, method);
    let (before_second, second) = body.rsplit_once("try {").expect("second protected read");
    let (protected, handler) = second.split_once("} catch (").expect("distinct handler");
    let (handler, normal) = handler.split_once('}').expect("end of catch");
    assert!(protected.contains(".read("));
    assert!(!protected.contains(".close("));
    assert!(!protected.contains(".mark("));
    for (path, first) in [(normal, false), (handler, true)] {
        let mut lines = path.lines().filter(|line| line.contains(".close("));
        let close = lines.next().expect("one cleanup");
        assert!(lines.next().is_none());
        let before_close = path.split_once(close).unwrap().0;
        assert_eq!(before_close.matches(".mark(").count(), usize::from(pair));
        assert_eq!(path.matches(".mark(").count(), usize::from(pair));
        let mut want = vec![params[usize::from(!first)]];
        if pair {
            want.push(params[usize::from(first)]);
        }
        assert_eq!(arguments_at_close(before_second, close), want, "{text}");
    }
    assert!(handler.contains("throw "));
    assert!(normal.contains("return "));
}

#[test]
fn different_second_operand_survives_on_each_exit() {
    check_distinct_cleanups(
        SEQUENTIAL,
        "Lfixture/SequentialCleanupOrigins;",
        "readBoth",
        false,
    );
}

#[test]
fn swapped_arguments_after_shared_mark_remain_path_specific() {
    check_distinct_cleanups(ORDERED, "Lfixture/OrderedCleanupArguments;", "run", true);
}
