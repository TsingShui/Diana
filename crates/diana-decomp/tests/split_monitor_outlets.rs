//! Closed release outlets keep their own original unlocked continuations.

use diana_decomp::p2::OpKind;
use diana_decomp::p6::{LockSeparation, MonitorMode, RegionKind, SyncExitKind, SyncLoopTransfer};
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;
#[path = "support/locks.rs"]
mod locks;
use locks::lock_anchors;

fn copy_anchors(facts: &diana_decomp::p6::RegionFacts, rendered: &Rendered) {
    for copy in facts
        .synchronized()
        .iter()
        .flat_map(|s| s.exits())
        .flat_map(|e| e.after_release())
    {
        assert_eq!(
            rendered
                .spans()
                .iter()
                .filter(|s| matches!(s.origin,
            Origin::Copy { copy: id, .. } if id.0 == copy.0))
                .count(),
            1,
            "original outlet copy has one phase anchor"
        );
    }
}

#[test]
fn three_inner_tails_are_unlocked_but_still_inside_the_outer_capture() {
    inspect(
        "sync/monitor-split-tails",
        "Lfixture/MonitorSplit;",
        "split",
        "(Ljava/lang/Object;Ljava/lang/Object;II)I",
        |facts, code, rendered| {
            assert_eq!(
                facts
                    .regions()
                    .iter()
                    .filter(|r| matches!(r.kind(), RegionKind::Synchronized(_)))
                    .count(),
                2
            );
            let [outer, inner] = facts.synchronized() else {
                panic!("two original captures")
            };
            assert!(
                outer
                    .held_nodes()
                    .contains(&code.operations()[inner.enter().0 as usize].node)
            );
            assert!(
                inner.follow().is_none(),
                "three original targets have no implicit common follow"
            );
            let mut targets = Vec::new();
            let mut tail_calls = 0;
            for exit in inner.exits() {
                assert_eq!(
                    facts.links()[exit.release_link().0 as usize].to(),
                    exit.target()
                );
                if let SyncExitKind::Follow(target) = exit.kind() {
                    assert!(!inner.members().contains(&target));
                    assert!(outer.held_nodes().contains(&target));
                    targets.push(target);
                    let block = facts.nodes()[target.0 as usize].block();
                    for op in code.operations() {
                        if facts.nodes()[op.node.0 as usize].block() == block
                            && matches!(op.original, OpKind::Invoke { .. })
                        {
                            assert!(!inner.held_nodes().contains(&op.node));
                            assert!(outer.held_nodes().contains(&op.node));
                            tail_calls += 1;
                        }
                    }
                }
            }
            targets.sort();
            targets.dedup();
            assert_eq!(targets.len(), 3);
            assert_eq!(tail_calls, 3, "one original call in each separate tail");
            lock_anchors(code, rendered);
            copy_anchors(facts, rendered);
            assert!(!rendered.text().contains("monitor_enter"));
        },
    );
}

#[test]
fn executable_post_release_phi_copies_keep_the_scope_explicit() {
    inspect(
        "sync/monitor-split-tails",
        "Lfixture/MonitorSplit;",
        "transfer",
        "(Ljava/lang/Object;II)I",
        |facts, code, rendered| {
            assert!(facts.monitor_plans().iter().any(|m| matches!(
                m.mode(),
                MonitorMode::Explicit(LockSeparation::PostReleaseCopy(_))
            )));
            assert!(
                !facts
                    .regions()
                    .iter()
                    .any(|r| matches!(r.kind(), RegionKind::Synchronized(_)))
            );
            assert!(
                facts
                    .synchronized()
                    .iter()
                    .flat_map(|s| s.exits())
                    .flat_map(|e| e.after_release())
                    .any(|p| !code.copies()[p.0 as usize].steps.is_empty())
            );
            lock_anchors(code, rendered);
            assert!(rendered.text().contains("monitor_enter"));
            copy_anchors(facts, rendered);
        },
    );
}

#[test]
fn mixed_loop_release_intents_need_no_fabricated_shared_follow() {
    inspect(
        "sync/monitor-iteration-exits",
        "Lfixture/MonitorIteration;",
        "walk",
        "(Ljava/lang/Object;II)I",
        |facts, code, rendered| {
            let [scope] = facts.synchronized() else {
                panic!("one closed scope")
            };
            assert!(scope.follow().is_none());
            assert!(scope.separation().is_none());
            let plan = &facts.loops()[0];
            let update = plan.iteration().expect("original common update");
            let mut kinds = [0; 4];
            for exit in scope.exits() {
                match exit.kind() {
                    SyncExitKind::Loop {
                        loop_id,
                        kind: SyncLoopTransfer::Continue,
                        target,
                    } => {
                        assert_eq!(loop_id, plan.id());
                        assert_eq!(target, update.entry());
                        kinds[0] += 1;
                    }
                    SyncExitKind::Loop {
                        kind: SyncLoopTransfer::Break,
                        target,
                        ..
                    } => {
                        assert_eq!(Some(target), plan.follow());
                        kinds[1] += 1;
                    }
                    SyncExitKind::Follow(_) => kinds[2] += 1,
                    SyncExitKind::Rethrow { .. } => kinds[3] += 1,
                    other => panic!("unexpected {other:?}"),
                }
            }
            assert_eq!(kinds, [2, 1, 1, 1]);
            lock_anchors(code, rendered);
            copy_anchors(facts, rendered);
        },
    );
}

#[test]
fn a_released_direct_header_continuation_does_not_acquire_an_update() {
    inspect(
        "sync/monitor-iteration-exits",
        "Lfixture/MonitorIteration;",
        "bypass",
        "(Ljava/lang/Object;II)I",
        |facts, code, rendered| {
            let plan = &facts.loops()[0];
            assert!(plan.iteration().is_none());
            assert_eq!(plan.latches().len(), 2);
            assert!(facts.synchronized().iter().flat_map(|s| s.exits()).any(|e| matches!(e.kind(),
                SyncExitKind::Loop { kind: SyncLoopTransfer::Continue, target, .. } if target == plan.header())));
            assert!(
                facts
                    .regions()
                    .iter()
                    .any(|r| matches!(r.kind(), RegionKind::Synchronized(_)))
            );
            lock_anchors(code, rendered);
            assert!(!rendered.text().contains("for (;"));
            copy_anchors(facts, rendered);
        },
    );
}
