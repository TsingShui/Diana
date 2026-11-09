//! Selected loop context refines release roles without moving lock effects.

use diana_decomp::p2::OpKind;
use diana_decomp::p6::{RegionFacts, RegionKind, SyncExitKind, SyncLoopTransfer};
use diana_decomp::p7::{MethodCode, TransferKind, TransferSpelling};
use diana_decomp::p9::Rendered;

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;
#[path = "support/locks.rs"]
mod locks;
use locks::lock_anchors;

fn loop_update(facts: &RegionFacts, code: &MethodCode, rendered: &Rendered) {
    let [plan] = facts.loops() else {
        panic!("one source loop")
    };
    let update = plan.iteration().expect("delivered source iteration");
    assert_eq!(
        facts.links()[update.repeat().0 as usize].to(),
        plan.header()
    );
    assert!(
        code.emission()
            .loop_header_operation(update.operation())
            .is_some()
    );
    assert!(rendered.text().contains("for (;"));
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
            assert!(facts.loops().iter().all(|p| p.iteration().is_some()));
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
            assert!(
                !rendered.text().contains("goto "),
                "no goto to a header-owned update"
            );
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
            let update = plan.iteration().unwrap();
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
                    SyncExitKind::Loop {
                        loop_id,
                        kind: SyncLoopTransfer::Continue,
                        target,
                    } => {
                        assert_eq!(loop_id, plan.id());
                        assert_eq!(target, update.entry());
                        continues += 1;
                    }
                    SyncExitKind::Rethrow { .. } => rethrows += 1,
                    other => panic!("unexpected outlet {other:?}"),
                }
            }
            assert_eq!((continues, rethrows), (2, 1));
            assert!(!rendered.text().contains("goto "));
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
            assert!(plan.iteration().is_none());
            assert_eq!(plan.continue_target(), plan.header());
            assert!(!rendered.text().contains("for (;"));
            assert!(rendered.text().contains("while ("));
            lock_anchors(code, rendered);
        },
    );
}
