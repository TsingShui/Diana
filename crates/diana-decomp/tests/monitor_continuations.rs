//! Selected loop context refines release roles without moving lock effects.

use diana_decomp::p2::OpKind;
use diana_decomp::p6::{RegionFacts, RegionKind, SyncExitKind};
use diana_decomp::p7::{MethodCode, TransferKind, TransferSpelling};
use diana_decomp::p9::Rendered;

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;
#[path = "support/locks.rs"]
mod locks;
use locks::lock_anchors;

#[path = "support/loops.rs"]
mod loops;
use loops::updates_once;

fn loop_update(facts: &RegionFacts, code: &MethodCode, rendered: &Rendered) {
    assert_eq!(facts.loops().len(), 1);
    assert_eq!(updates_once(facts, code, rendered).len(), 1);
    assert!(rendered.text().contains("while ("));
    lock_anchors(code, rendered);
}

#[test]
fn a_direct_release_outlet_uses_its_control_continuation_once() {
    inspect(
        "exceptions/nested-catch-rejoins",
        "Lfixture/NestedCatchLoops;",
        "run",
        "(Ljava/lang/Object;II)I",
        |facts, code, rendered| {
            assert_eq!(facts.loops().len(), 2);
            assert_eq!(updates_once(facts, code, rendered).len(), 2);
            lock_anchors(code, rendered);
            let mut outlets = 0;
            for transfer in code.transfers() {
                if matches!(transfer.kind, TransferKind::ReleasedSync { .. }) {
                    assert!(matches!(
                        code.emission().transfers()[transfer.id.0 as usize].spelling,
                        TransferSpelling::Fallthrough
                    ));
                    outlets += 1;
                }
            }
            assert_eq!(outlets, 2);
            // A shared update may have a visible goto; it is never hidden in
            // a header. Original operations and release completion stay once.
        },
    );
}

#[test]
fn held_cleanup_releases_then_enters_the_original_update() {
    inspect(
        "sync/monitor-iteration-exits",
        "Lfixture/MonitorIteration;",
        "heldCleanup",
        "(Ljava/lang/Object;II)I",
        |facts, code, rendered| {
            loop_update(facts, code, rendered);
            let plan = &facts.loops()[0];
            let updates = updates_once(facts, code, rendered);
            let update = code.operations()[updates[0].0 as usize].node;
            assert_eq!(
                facts
                    .regions()
                    .iter()
                    .filter(|r| matches!(r.kind(), RegionKind::Synchronized(_)))
                    .count(),
                1
            );
            let [scope] = facts.synchronized() else {
                panic!("one held scope")
            };
            let mut continues = 0;
            let mut rethrows = 0;
            for exit in scope.exits() {
                assert_eq!(
                    facts.links()[exit.release_link().0 as usize].to(),
                    exit.target()
                );
                match exit.kind() {
                    SyncExitKind::Follow(target) => {
                        assert_ne!(target, plan.continue_target());
                        assert_eq!(target, update);
                        continues += 1;
                    }
                    SyncExitKind::Rethrow { .. } => rethrows += 1,
                    other => panic!("unexpected outlet {other:?}"),
                }
            }
            assert_eq!((continues, rethrows), (2, 1));
        },
    );
}

#[test]
fn multiple_release_exits_keep_all_original_lock_operations() {
    inspect(
        "sync/monitor-iteration-exits",
        "Lfixture/MonitorIteration;",
        "walk",
        "(Ljava/lang/Object;II)I",
        |facts, code, rendered| {
            loop_update(facts, code, rendered);
            assert_eq!(
                code.operations()
                    .iter()
                    .filter(|op| matches!(op.original, OpKind::Monitor { .. }))
                    .count(),
                6
            );
            assert!(rendered.text().contains("return "));
            assert!(rendered.text().contains("throw "));
        },
    );
}

#[test]
fn a_released_header_backedge_still_skips_the_step() {
    inspect(
        "sync/monitor-iteration-exits",
        "Lfixture/MonitorIteration;",
        "bypass",
        "(Ljava/lang/Object;II)I",
        |facts, code, rendered| {
            let [plan] = facts.loops() else {
                panic!("one source loop")
            };
            assert_eq!(plan.latches().len(), 2);
            assert_eq!(plan.continue_target(), plan.header());
            assert!(!rendered.text().contains("for (;"));
            assert!(rendered.text().contains("while ("));
            lock_anchors(code, rendered);
        },
    );
}
