//! Omit a second display action, not the original transfer or opaque completion.

use diana_decomp::p6::{RegionKind, SyncExitKind};
use diana_decomp::p7::{
    BlockEnd, ControlCompletion, ControlKind, EmissionLayout, TerminalKind, TransferSpelling,
};
use diana_decomp::p9::Origin;

#[path = "support/inspection.rs"]
mod inspection;
#[path = "support/rethrows.rs"]
mod rethrows;
use inspection::inspect;

#[test]
fn case_breaks_keep_their_original_transfer_but_no_second_action() {
    inspect(
        "branches/abrupt-continuation-tails",
        "Lfixture/TailCompletion;",
        "decorated",
        "(III)I",
        |_, code, rendered| {
            let mut omitted = 0;
            for control in code.controls() {
                // A precise shared-tail producer can already omit the source
                // continuation. The invariant still applies to every printed
                // unconditional exit, not only an old Goto-shaped plan.
                let mut tail = control;
                while matches!(
                    code.controls()[tail.id.0 as usize].layout,
                    EmissionLayout::Sequence
                ) {
                    let Some(child) = code.controls()[tail.id.0 as usize].children().last() else {
                        break;
                    };
                    tail = &code.controls()[child.0 as usize];
                }
                let ControlKind::Block { node } = tail.kind else {
                    continue;
                };
                let transfer = match &code.emission().blocks()[node.0 as usize].end {
                    BlockEnd::Terminal(block) => match code.terminals()[block.0 as usize].kind {
                        TerminalKind::Goto(t) => Some(t),
                        _ => None,
                    },
                    BlockEnd::Transfers(t) => t.last().copied(),
                    BlockEnd::None => None,
                };
                let Some(t) = transfer else { continue };
                if matches!(
                    code.emission().transfers()[t.0 as usize].spelling,
                    TransferSpelling::Break(_)
                        | TransferSpelling::BreakHere(_)
                        | TransferSpelling::Goto(_)
                ) {
                    assert!(
                        code.controls()[control.id.0 as usize]
                            .continuation
                            .is_none()
                    );
                    assert_eq!(
                        rendered
                            .spans()
                            .iter()
                            .filter(|s| matches!(s.origin,
                        Origin::Transfer { transfer, .. } if transfer == t))
                            .count(),
                        1
                    );
                    omitted += 1;
                }
            }
            assert!(
                omitted >= 2,
                "original case exits keep single-completion coverage active"
            );
        },
    );
}

#[test]
fn a_case_break_does_not_erase_work_after_its_own_switch() {
    inspect(
        "branches/abrupt-continuation-tails",
        "Lfixture/TailCompletion;",
        "breakScope",
        "(I)I",
        |facts, code, rendered| {
            assert!(
                facts
                    .regions()
                    .iter()
                    .any(|r| matches!(r.kind(), RegionKind::Switch { .. }))
            );
            assert!(code.emission().transfers().iter().any(|t| matches!(
                t.spelling,
                TransferSpelling::Break(_) | TransferSpelling::BreakHere(_)
            )));
            assert!(rendered.text().contains("TailCompletion.mark(62)"));
            assert!(rendered.text().contains("return 7"));
        },
    );
}

#[test]
fn protected_completion_keeps_cleanup_and_later_effects() {
    inspect(
        "branches/abrupt-continuation-tails",
        "Lfixture/TailCompletion;",
        "select",
        "(III)I",
        |facts, _, rendered| {
            assert!(
                facts
                    .regions()
                    .iter()
                    .any(|r| matches!(r.kind(), RegionKind::Try(_)))
            );
            assert!(rendered.text().contains("TailCompletion.mark(30)"));
            assert!(rendered.text().contains("TailCompletion.mark(40)"));
            rethrows::assert_pending_rethrow(facts, rendered);
        },
    );
    inspect(
        "branches/abrupt-continuation-tails",
        "Lfixture/TailCompletion;",
        "caught",
        "(I)I",
        |_, _, rendered| {
            assert!(rendered.text().contains("TailCompletion.mark(51)"));
            assert!(rendered.text().contains("TailCompletion.mark(52)"));
        },
    );
}

#[test]
fn a_release_without_an_emitted_jump_keeps_its_planned_continuation() {
    inspect(
        "exceptions/sync-finally-return",
        "Lfixture/SyncFinallyReturn;",
        "syncFinally",
        "(Ljava/lang/Object;I)I",
        |facts, code, _| {
            let mut retained = 0;
            for exit in facts.synchronized().iter().flat_map(|s| s.exits()) {
                let SyncExitKind::Follow(target) = exit.kind() else {
                    continue;
                };
                let node = code.operations()[exit.release().0 as usize].node;
                let control = code
                    .controls()
                    .iter()
                    .find(|c| {
                        matches!(c.kind,
                    ControlKind::Block { node: n } if n == node)
                    })
                    .expect("original release block");
                assert!(
                    matches!(code.controls()[control.id.0 as usize].continuation,
                    Some(ControlCompletion::Goto(label)) if label.0 == target.0)
                );
                retained += 1;
            }
            assert_eq!(retained, 1);
        },
    );
}
