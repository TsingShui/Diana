//! Source iteration tails keep original updates, phases and continuation targets.

use diana_decomp::p6::{RegionFacts, RegionKind, SyncExitKind, SyncLoopTransfer};
use diana_decomp::p7::{EmissionLayout, LoopPlacement, MethodCode};
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn update_anchors(facts: &RegionFacts, code: &MethodCode, rendered: &Rendered) {
    for plan in facts.loops() {
        let Some(update) = plan.iteration() else {
            continue;
        };
        assert_eq!(plan.continue_target(), update.entry());
        assert_ne!(plan.header(), update.entry());
        assert_eq!(
            facts.links()[update.repeat().0 as usize].to(),
            plan.header()
        );
        assert!(
            code.emission()
                .loop_header_operation(update.operation())
                .is_some()
        );
        let spans: Vec<_> = rendered.spans().iter().filter(|s| matches!(s.origin, Origin::Operation { operation, .. } if operation==update.operation())).collect();
        assert_eq!(
            spans.len(),
            1,
            "update has exactly one original operation anchor"
        );
        assert!(
            spans[0].start.0 < spans[0].end.0,
            "update is printed, not origin-only"
        );
    }
}

#[test]
fn skip_value_runs_the_original_update_before_retesting() {
    inspect(
        "exceptions/typed-catch-loop-exits",
        "Lfixture/LoopCatchFinally;",
        "scan",
        "(II)I",
        |facts, code, rendered| {
            let [plan] = facts.loops() else {
                panic!("one source loop");
            };
            assert!(plan.iteration().is_some());
            update_anchors(facts, code, rendered);
            assert!(rendered.text().contains("for (;"));
            assert!(!rendered.text().contains("goto "));
        },
    );
}

#[test]
fn both_nested_updates_remain_in_their_own_loop_headers() {
    inspect(
        "loops/iteration-tail-cleanup",
        "Lfixture/IterationTail;",
        "nested",
        "(III)I",
        |facts, code, rendered| {
            assert_eq!(facts.loops().len(), 2);
            assert!(facts.loops().iter().all(|p| p.iteration().is_some()));
            assert_eq!(
                code.emission()
                    .controls()
                    .iter()
                    .filter(|c| matches!(
                        c.layout,
                        EmissionLayout::Loop {
                            placement: LoopPlacement::For { .. },
                            ..
                        }
                    ))
                    .count(),
                2
            );
            update_anchors(facts, code, rendered);
            assert!(!rendered.text().contains("graph<"));
        },
    );
}

#[test]
fn body_before_the_original_test_is_not_moved_to_the_iteration() {
    inspect(
        "loops/loop-for-multi",
        "Lfixture/LoopForMulti;",
        "emptyCondition",
        "(I)I",
        |facts, code, rendered| {
            let [plan] = facts.loops() else {
                panic!("one source loop");
            };
            assert!(plan.iteration().is_some());
            let mut kept = 0;
            for op in code.operations() {
                if plan.evaluation().contains(&op.node)
                    && matches!(op.original, diana_decomp::p2::OpKind::Binary { .. })
                {
                    assert!(code.emission().loop_header_operation(op.source).is_none());
                    assert!(rendered.spans().iter().any(|s| matches!(s.origin, Origin::Operation { operation, .. } if operation == op.source) && s.start.0 < s.end.0));
                    kept += 1;
                }
            }
            assert!(kept > 0, "original pre-test assignment remains evaluated");
        },
    );
}

#[test]
fn a_header_backedge_that_skips_the_update_stays_a_while() {
    inspect(
        "loops/iteration-tail-cleanup",
        "Lfixture/IterationTail;",
        "skipUpdate",
        "(II)I",
        |facts, _, rendered| {
            let [plan] = facts.loops() else {
                panic!("one source loop");
            };
            assert_eq!(plan.latches().len(), 2);
            assert!(plan.iteration().is_none());
            assert_eq!(plan.continue_target(), plan.header());
            assert!(rendered.text().contains("while ("));
            assert!(!rendered.text().contains("for (;"));
        },
    );
}

#[test]
fn specialized_release_uses_the_supplied_iteration_continuation() {
    inspect(
        "loops/iteration-tail-cleanup",
        "Lfixture/IterationTail;",
        "locked",
        "(Ljava/lang/Object;I)I",
        |facts, code, rendered| {
            let [plan] = facts.loops() else {
                panic!("one source loop");
            };
            let update = plan.iteration().expect("delivered source update");
            update_anchors(facts, code, rendered);
            assert!(facts.synchronized().iter().flat_map(|s| s.exits()).any(|e|
                matches!(e.kind(), SyncExitKind::Loop { loop_id, kind: SyncLoopTransfer::Continue, target }
                    if loop_id == plan.id() && target == update.entry())));
            assert_eq!(
                facts
                    .regions()
                    .iter()
                    .filter(|r| matches!(r.kind(), RegionKind::Synchronized(_)))
                    .count(),
                1
            );
            assert!(rendered.text().contains("for (;"));
            assert!(!rendered.text().contains("goto "));
        },
    );
}
