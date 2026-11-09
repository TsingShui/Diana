//! Single-entry loop construction with private versus foreign handler arrivals.

use diana_decomp::p2::OpKind;
use diana_decomp::p6::RegionKind;
use diana_decomp::p9::Origin;

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

#[test]
fn private_handler_rejoins_are_inside_the_single_loop() {
    inspect(
        "exceptions/typed-catch-loop-exits",
        "Lfixture/LoopCatchFinally;",
        "scan",
        "(II)I",
        |facts, _, rendered| {
            let [plan] = facts.loops() else {
                panic!("one source loop");
            };
            assert!(
                facts
                    .regions()
                    .iter()
                    .any(|r| matches!(r.kind(), RegionKind::Loop(id) if *id == plan.id()))
            );
            let catches = facts
                .tries()
                .iter()
                .find(|p| p.handler_clauses().iter().any(|r| r.end - r.start == 2))
                .unwrap();
            assert!(
                catches
                    .handlers()
                    .iter()
                    .all(|n| plan.lexical_members().contains(n))
            );
            assert!(!rendered.text().contains("graph<"), "{}", rendered.text());
        },
    );
}

#[test]
fn nested_private_rejoins_keep_two_loops_and_monitor_order() {
    inspect(
        "exceptions/nested-catch-rejoins",
        "Lfixture/NestedCatchLoops;",
        "run",
        "(Ljava/lang/Object;II)I",
        |facts, code, rendered| {
            assert_eq!(facts.loops().len(), 2);
            assert_eq!(
                facts
                    .regions()
                    .iter()
                    .filter(|r| matches!(r.kind(), RegionKind::Loop(_)))
                    .count(),
                2
            );
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
            assert!(
                inner
                    .members()
                    .iter()
                    .all(|n| outer.lexical_members().contains(n))
            );
            assert_eq!(
                facts
                    .regions()
                    .iter()
                    .filter(|r| matches!(r.kind(), RegionKind::Synchronized(_)))
                    .count(),
                1
            );
            let mut monitors = 0;
            for operation in code.operations() {
                if matches!(operation.original, OpKind::Monitor { .. }) {
                    monitors += 1;
                    assert_eq!(rendered.spans().iter().filter(|s| matches!(s.origin, Origin::Operation { operation: source, pc, .. } if source == operation.source && pc == operation.pc)).count(), 1);
                }
            }
            assert_eq!(monitors, 3);
            assert!(!rendered.text().contains("graph<"), "{}", rendered.text());
        },
    );
}

#[test]
fn a_preloop_exception_is_not_a_private_rejoin() {
    inspect(
        "exceptions/nested-catch-rejoins",
        "Lfixture/NestedCatchLoops;",
        "foreign",
        "(II)I",
        |facts, _, rendered| {
            assert!(
                facts.loops().is_empty(),
                "foreign pre-loop arrival must remain visible"
            );
            assert!(rendered.text().contains("graph<"));
            assert!(rendered.text().contains("on_exception_entry"));
        },
    );
}
