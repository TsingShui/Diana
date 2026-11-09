//! While/do-while retain original shared updates and exact nonlocal exits.

use diana_decomp::p6::{LoopForm, RegionKind};
use diana_decomp::p7::{EmissionLayout, LoopPlacement};
use diana_decomp::p9::Origin;

#[path = "support/inspection.rs"]
mod inspection;
#[path = "support/loops.rs"]
mod loops;
use inspection::inspect;
use loops::updates_once;

#[test]
fn three_nested_loops_keep_the_private_outer_continue_tail() {
    inspect(
        "loops/while-tail-continuations",
        "Lfixture/WhileTails;",
        "nested",
        "(III)I",
        |facts, code, rendered| {
            assert_eq!(facts.loops().len(), 3);
            assert_eq!(updates_once(facts, code, rendered).len(), 4);
            let inner = facts
                .loops()
                .iter()
                .min_by_key(|p| p.members().len())
                .unwrap();
            let outer = facts
                .loops()
                .iter()
                .max_by_key(|p| p.members().len())
                .unwrap();
            let [exit] = inner.private_exits() else {
                panic!("one proved private entrance")
            };
            let link = facts.links()[exit.0 as usize];
            assert!(facts.regions().iter().any(|r| {
                matches!(r.kind(), RegionKind::If { .. })
                    && r.nodes().contains(&link.from())
                    && r.nodes().contains(&link.to())
                    && r.follow() == Some(inner.continue_target())
                    && outer
                        .latches()
                        .iter()
                        .any(|l| r.nodes().contains(&facts.links()[l.0 as usize].from()))
            }));
            assert!(rendered.text().contains("continue R"));
            assert!(!rendered.text().contains("goto "));
        },
    );
}

#[test]
fn two_original_tail_entrances_do_not_become_private() {
    inspect(
        "loops/while-tail-continuations",
        "Lfixture/WhileTails;",
        "shared",
        "(III)I",
        |facts, code, rendered| {
            assert_eq!(facts.loops().len(), 3);
            assert!(facts.loops().iter().all(|p| p.private_exits().is_empty()));
            assert_eq!(updates_once(facts, code, rendered).len(), 4);
            assert!(facts.nodes().iter().any(|n| {
                facts.links().iter().filter(|l| l.to() == n.id()).count() == 2
                    && !facts.loops().iter().any(|p| p.header() == n.id())
            }));
            assert_eq!(
                rendered.text().matches("WhileTails.touch(").count(),
                2,
                "one private-path call plus the distinct normal outer tail, never cloned per entrance"
            );
            assert!(rendered.text().matches("goto L").count() >= 2);
        },
    );
}

#[test]
fn cleanup_then_break_or_shared_update_have_distinct_body_positions() {
    inspect(
        "loops/while-tail-continuations",
        "Lfixture/WhileTails;",
        "walk",
        "(III)I",
        |facts, code, rendered| {
            assert_eq!(
                facts
                    .tries()
                    .iter()
                    .filter(|p| p.finally().is_some())
                    .count(),
                1
            );
            let updates = updates_once(facts, code, rendered);
            assert_eq!(updates.len(), 1);
            let update = rendered.spans().iter().find(|s| matches!(s.origin, Origin::Operation { operation, .. } if operation == updates[0])).unwrap();
            assert!(rendered.text().find("WhileTails.cleanup(").unwrap() < update.start.0);
            assert!(rendered.text().find("break;").unwrap() < update.start.0);
            assert!(rendered.text()[update.start.0..update.end.0].contains("+ p1"));
            assert_ne!(
                facts.loops()[0].continue_target(),
                code.operations()[updates[0].0 as usize].node
            );
        },
    );
}

#[test]
fn a_tail_test_keeps_the_first_body_for_nonpositive_limits() {
    inspect(
        "loops/while-tail-continuations",
        "Lfixture/WhileTails;",
        "tail",
        "(II)I",
        |facts, code, rendered| {
            let [plan] = facts.loops() else {
                panic!("one loop")
            };
            assert!(matches!(plan.form(), LoopForm::DoWhile(_)));
            assert_eq!(updates_once(facts, code, rendered).len(), 1);
            assert!(code.emission().controls().iter().any(|c| matches!(
                c.layout,
                EmissionLayout::Loop {
                    placement: LoopPlacement::DoWhile { .. },
                    ..
                }
            )));
            assert!(
                rendered.text().find("do {").unwrap()
                    < rendered.text().find("WhileTails.touch(").unwrap()
            );
            assert!(
                rendered.text().find("WhileTails.touch(").unwrap()
                    < rendered.text().rfind("while (").unwrap()
            );
        },
    );
}
