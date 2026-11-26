mod support;

use diana_decomp::{MethodSelector, Request};

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/exceptions/sibling-exit-cleanup/classes.dex");

#[test]
fn a_completed_switch_and_its_loop_leave_the_next_protected_latch_in_scope() {
    let engine = support::engine(DEX);
    let class = support::class(DEX, &engine, "Lfixture/SiblingExit;");
    let source = engine
        .class_source(Request {
            class,
            method: Some(MethodSelector {
                name: "sweep".encode_utf16().collect(),
                prototype: "(III)I".encode_utf16().collect(),
            }),
        })
        .expect("sweep");
    assert!(source.failures.is_empty(), "{:?}", source.failures);
    let mut lines = source.text.lines().map(str::trim);
    lines.find(|line| line.starts_with("switch (")).unwrap();
    assert_eq!(lines.next(), Some("case 0:"));
    assert_eq!(lines.next(), Some("case 1:"));
    assert!(lines.next().unwrap().contains("ScopeHooks.start("));
    assert!(lines.next().unwrap().starts_with("ScopeHooks.step("));
    lines.find(|line| *line == "case 2:").unwrap();
    assert!(lines.next().unwrap().starts_with("ScopeHooks.skip("));
    assert_eq!(lines.next(), Some("break;"));
    assert_eq!(lines.next(), Some("default:"));
    assert_eq!(lines.next(), Some("break;"));
    assert_eq!(lines.next(), Some("}"));
    assert_eq!(lines.next(), Some("try {"));
    assert_eq!(lines.next(), Some("while (true) {"));
    lines
        .find(|line| line.contains("ScopeHooks.read("))
        .unwrap();
    let catch = lines.next().unwrap();
    assert!(catch.starts_with("} catch ("));
    assert!(catch.contains("NullPointerException") && catch.contains("NumberFormatException"));
    assert!(lines.next().unwrap().contains("ScopeHooks.recover("));
    assert_eq!(lines.next(), Some("}"));
    assert!(lines.next().unwrap().ends_with("++;"));
    assert!(lines.next().unwrap().contains(" += "));
    // The protected latch is visited after the sibling preface has ended.
    // Both build profiles must retain the ordinary region-owned transfer.
    assert_eq!(lines.next(), Some("continue;"));
    assert_eq!(lines.next(), Some("}"));
    assert_eq!(lines.next(), Some("} finally {"));
    assert!(lines.next().unwrap().starts_with("if ("));
    assert!(lines.next().unwrap().starts_with("ScopeHooks.skip("));
    assert_eq!(lines.next(), Some("} else {"));
    assert!(lines.next().unwrap().starts_with("ScopeHooks.step("));
    assert!(lines.next().unwrap().starts_with("ScopeHooks.close("));
}
