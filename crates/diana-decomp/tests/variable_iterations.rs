//! Available local reads retain their original update and boundary phases.

use diana_decomp::p2::{BinaryOp, OpKind};
use diana_decomp::p6::RegionKind;
use diana_decomp::p7::{ControlCompletion, ControlId, MethodCode, TransferSpelling};
use diana_decomp::p9::Origin;

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

#[path = "support/loops.rs"]
mod loops;
use loops::updates_once;

fn boundary_completions(code: &MethodCode) -> Vec<ControlCompletion> {
    let mut out = Vec::new();
    for c in code.controls() {
        if let Some(completion) = c.continuation {
            out.push(match completion {
                ControlCompletion::BreakHere(target) => ControlCompletion::Break(target),
                ControlCompletion::ContinueHere(target) => ControlCompletion::Continue(target),
                other => other,
            });
        }
    }
    for transfer in code.emission().transfers() {
        match transfer.spelling {
            TransferSpelling::Break(target) | TransferSpelling::BreakHere(target) => {
                out.push(ControlCompletion::Break(target))
            }
            TransferSpelling::Continue(target) | TransferSpelling::ContinueHere(target) => {
                out.push(ControlCompletion::Continue(target))
            }
            TransferSpelling::Goto(target) => out.push(ControlCompletion::Goto(target)),
            _ => {}
        }
    }
    out
}

#[test]
fn an_available_parameter_is_read_at_the_original_shared_update() {
    inspect(
        "loops/iteration-tail-cleanup",
        "Lfixture/IterationTail;",
        "stride",
        "(II)I",
        |facts, code, rendered| {
            assert_eq!(facts.loops().len(), 1);
            updates_once(facts, code, rendered);
            assert!(rendered.text().contains("while ("));
            assert!(rendered.text().contains("+ p1"));
        },
    );
}

#[test]
fn a_body_mutated_operand_and_no_body_break_keep_distinct_schedules() {
    inspect(
        "loops/mutable-iteration-step",
        "Lfixture/MutableStep;",
        "walk",
        "(III)I",
        |facts, code, rendered| {
            assert_eq!(facts.loops().len(), 1);
            updates_once(facts, code, rendered);
            let boundaries = boundary_completions(code);
            assert_eq!(boundaries.len(), 2);
            assert!(
                boundaries
                    .iter()
                    .all(|c| matches!(c, ControlCompletion::Break(_)))
            );
            assert_eq!(rendered.text().matches("break;").count(), 2);
            assert!(rendered.text().contains("cleanup("));
            // Standalone method rendering uses qualified type spellings.
            assert!(rendered.text().contains("NumberFormatException"));
            assert!(rendered.text().contains("NullPointerException"));
        },
    );
}

#[test]
fn nested_false_arms_keep_the_original_outer_break_and_continue() {
    inspect(
        "loops/mutable-iteration-step",
        "Lfixture/MutableStep;",
        "nested",
        "(III)I",
        |facts, code, rendered| {
            assert_eq!(facts.loops().len(), 2);
            updates_once(facts, code, rendered);
            let outer = facts
                .loops()
                .iter()
                .max_by_key(|p| p.members().len())
                .unwrap();
            let scope = facts
                .regions()
                .iter()
                .find(|r| matches!(r.kind(), RegionKind::Loop(id) if *id == outer.id()))
                .unwrap();
            let transfers = boundary_completions(code);
            assert_eq!(transfers.len(), 2);
            assert!(transfers.iter().any(
                |t| matches!(t, ControlCompletion::Break(target) if *target == ControlId(scope.id().0))
            ));
            let inner = facts
                .loops()
                .iter()
                .min_by_key(|p| p.members().len())
                .unwrap();
            assert!(transfers.iter().any(|t| matches!(t, ControlCompletion::Goto(target) if {
                let node = diana_decomp::p6::NodeId(target.0);
                outer.members().contains(&node) && !inner.members().contains(&node) && node != outer.continue_target()
            })), "the original outer update is an explicit target, not a continue that skips it");
            assert!(rendered.text().contains("break R"));
            assert!(rendered.text().contains("goto L"));
        },
    );
}

#[test]
fn a_shared_throwing_update_stays_at_its_original_body_point() {
    inspect(
        "loops/mutable-iteration-step",
        "Lfixture/MutableStep;",
        "divide",
        "(II)I",
        |facts, code, rendered| {
            let [plan] = facts.loops() else {
                panic!("one original loop");
            };
            assert_eq!(plan.continue_target(), plan.header());
            let divide = code
                .operations()
                .iter()
                .find(|op| {
                    matches!(
                        op.original,
                        OpKind::Binary {
                            op: BinaryOp::DivInt,
                            ..
                        }
                    )
                })
                .expect("original division");
            assert!(rendered.spans().iter().any(|s| matches!(s.origin, Origin::Operation { operation, .. } if operation == divide.source) && s.start.0 < s.end.0));
            assert!(rendered.text().contains("while ("));
            assert!(!rendered.text().contains("for (;"));
        },
    );
}
