//! Original updates remain body statements, including shared update targets.

use diana_decomp::p6::{RegionKind, SyncExitKind};
use diana_decomp::p7::{EmissionLayout, LoopPlacement};
use diana_decomp::p9::Origin;

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;
#[path = "support/loops.rs"]
mod loops;
use loops::updates_once;

#[test]
fn skip_value_reaches_the_original_update_before_retesting() {
    inspect(
        "exceptions/typed-catch-loop-exits",
        "Lfixture/LoopCatchFinally;",
        "scan",
        "(II)I",
        |facts, code, rendered| {
            assert_eq!(facts.loops().len(), 1);
            let updates = updates_once(facts, code, rendered);
            assert_eq!(updates.len(), 1);
            assert!(rendered.text().contains("while ("));
            let update = code.operations()[updates[0].0 as usize].node;
            assert_ne!(facts.loops()[0].continue_target(), update);
        },
    );
}

#[test]
fn both_nested_updates_remain_in_their_own_loop_bodies() {
    inspect(
        "loops/iteration-tail-cleanup",
        "Lfixture/IterationTail;",
        "nested",
        "(III)I",
        |facts, code, rendered| {
            assert_eq!(facts.loops().len(), 2);
            assert_eq!(
                code.controls()
                    .iter()
                    .filter(|c| matches!(
                        c.layout,
                        EmissionLayout::Loop {
                            placement: LoopPlacement::While { .. },
                            ..
                        }
                    ))
                    .count(),
                2
            );
            assert_eq!(updates_once(facts, code, rendered).len(), 2);
            assert!(!rendered.text().contains("graph<"));
        },
    );
}

#[test]
fn body_before_the_original_test_is_not_relocated() {
    inspect(
        "loops/loop-for-multi",
        "Lfixture/LoopForMulti;",
        "emptyCondition",
        "(I)I",
        |facts, code, rendered| {
            let [plan] = facts.loops() else {
                panic!("one source loop")
            };
            let mut kept = 0;
            for op in code.operations() {
                if plan.evaluation().contains(&op.node)
                    && matches!(op.original, diana_decomp::p2::OpKind::Binary { .. })
                {
                    assert!(rendered.spans().iter().any(|s| matches!(s.origin, Origin::Operation { operation, .. } if operation == op.source) && s.start.0 < s.end.0));
                    kept += 1;
                }
            }
            assert!(
                kept > 0,
                "the first addition still precedes the first comparison"
            );
            updates_once(facts, code, rendered);
        },
    );
}

#[test]
fn a_header_backedge_that_skips_the_update_keeps_its_destination() {
    inspect(
        "loops/iteration-tail-cleanup",
        "Lfixture/IterationTail;",
        "skipUpdate",
        "(II)I",
        |facts, code, rendered| {
            let [plan] = facts.loops() else {
                panic!("one source loop")
            };
            assert_eq!(plan.latches().len(), 2);
            assert_eq!(plan.continue_target(), plan.header());
            assert!(rendered.text().contains("while ("));
            updates_once(facts, code, rendered);
        },
    );
}

#[test]
fn specialized_release_preserves_its_original_body_update_target() {
    inspect(
        "loops/iteration-tail-cleanup",
        "Lfixture/IterationTail;",
        "locked",
        "(Ljava/lang/Object;I)I",
        |facts, code, rendered| {
            let updates = updates_once(facts, code, rendered);
            let update = code.operations()[updates[0].0 as usize].node;
            assert!(
                facts
                    .synchronized()
                    .iter()
                    .flat_map(|s| s.exits())
                    .any(|e| matches!(e.kind(), SyncExitKind::Follow(target) if target == update))
            );
            assert_eq!(
                facts
                    .regions()
                    .iter()
                    .filter(|r| matches!(r.kind(), RegionKind::Synchronized(_)))
                    .count(),
                1
            );
            assert!(rendered.text().contains("while ("));
        },
    );
}
