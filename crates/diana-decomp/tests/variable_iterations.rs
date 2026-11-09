//! Available local reads retain their original update and boundary phases.

use diana_decomp::p2::{BinaryOp, OpKind};
use diana_decomp::p6::{RegionFacts, RegionKind};
use diana_decomp::p7::{ConditionElse, ControlId, EmissionLayout, MethodCode, TransferKind};
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn updates_once(facts: &RegionFacts, code: &MethodCode, rendered: &Rendered) {
    for plan in facts.loops() {
        let update = plan.iteration().expect("original shared update");
        assert_eq!(plan.continue_target(), update.entry());
        assert!(
            code.emission()
                .loop_header_operation(update.operation())
                .is_some()
        );
        let spans: Vec<_> = rendered.spans().iter().filter(|span| matches!(
            span.origin, Origin::Operation { operation, .. } if operation == update.operation()
        )).collect();
        assert_eq!(spans.len(), 1);
        assert!(spans[0].start.0 < spans[0].end.0);
    }
}

fn boundary_transfers(code: &MethodCode, rendered: &Rendered) -> Vec<ControlId> {
    let mut targets = Vec::new();
    for control in code.emission().controls() {
        let EmissionLayout::Condition {
            no: ConditionElse::Transfer,
            fall,
            ..
        } = control.layout
        else {
            continue;
        };
        let (target, keyword) = match code.transfers()[fall.0 as usize].kind {
            TransferKind::Break(target) => (target, "break"),
            TransferKind::Continue(target) => (target, "continue"),
            _ => panic!("this fixture supplies classified nonlocal loop completions"),
        };
        let spans: Vec<_> = rendered
            .spans()
            .iter()
            .filter(|span| {
                matches!(
                    span.origin, Origin::Transfer { transfer, .. } if transfer == fall
                )
            })
            .collect();
        assert_eq!(
            spans.len(),
            1,
            "final other edge executes once, not in the tests too"
        );
        let span = spans[0];
        assert!(rendered.text()[span.start.0..span.end.0].contains(keyword));
        targets.push(target);
    }
    targets
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
            assert!(rendered.text().contains("for (;"));
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
            assert_eq!(
                facts
                    .conditions()
                    .iter()
                    .filter(|p| p.explicit().is_some() && p.other_exit())
                    .count(),
                1
            );
            assert_eq!(boundary_transfers(code, rendered).len(), 1);
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
            assert_eq!(
                boundary_transfers(code, rendered),
                [ControlId(scope.id().0); 2]
            );
            assert_eq!(
                facts
                    .conditions()
                    .iter()
                    .filter(|p| p.explicit().is_some() && p.other_exit())
                    .count(),
                2
            );
        },
    );
}

#[test]
fn a_shared_throwing_update_is_not_admitted_as_a_pure_iteration() {
    inspect(
        "loops/mutable-iteration-step",
        "Lfixture/MutableStep;",
        "divide",
        "(II)I",
        |facts, code, rendered| {
            let [plan] = facts.loops() else {
                panic!("one original loop");
            };
            assert!(plan.iteration().is_none());
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
            assert!(
                code.emission()
                    .loop_header_operation(divide.source)
                    .is_none()
            );
            assert!(rendered.text().contains("while ("));
            assert!(!rendered.text().contains("for (;"));
        },
    );
}
