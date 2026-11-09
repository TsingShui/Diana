//! Completed pure body work and cleanup keep distinct original execution phases.

use diana_decomp::p2::OpKind;
use diana_decomp::p6::{RegionFacts, RegionKind, TryPlan};
use diana_decomp::p7::{ConditionElse, ControlId, EmissionLayout, MethodCode, TransferKind};
use diana_decomp::p9::Origin;

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn cleanup(facts: &RegionFacts) -> &TryPlan {
    facts
        .tries()
        .iter()
        .find(|p| p.finally().is_some())
        .expect("delivered matched cleanup")
}

fn operation_portions(facts: &RegionFacts, code: &MethodCode) {
    for operation in code.operations() {
        let node = &facts.nodes()[operation.node.0 as usize];
        assert_eq!(
            node.operations(),
            operation.source.0..operation.source.0 + 1
        );
        assert_eq!(node.block(), operation.original_block);
    }
}

fn post_cleanup_condition(code: &MethodCode) {
    assert!(
        code.emission()
            .controls()
            .iter()
            .any(|c| matches!(&c.layout,
        EmissionLayout::Condition { tests, .. } if tests.len() == 2))
    );
}

#[test]
fn a_completed_body_keeps_the_original_body_update_and_both_condition_exits() {
    inspect(
        "loops/mutable-iteration-step",
        "Lfixture/MutableStep;",
        "walk",
        "(III)I",
        |facts, code, rendered| {
            operation_portions(facts, code);
            cleanup(facts);
            assert!(
                facts
                    .loops()
                    .iter()
                    .all(|p| p.continue_target() == p.header())
            );
            assert!(!rendered.text().contains("for ("));
            assert_eq!(
                code.emission()
                    .controls()
                    .iter()
                    .filter(|c| matches!(c.layout, EmissionLayout::Condition { .. }))
                    .count(),
                1
            );
            assert_eq!(rendered.text().matches("break;").count(), 2);
            assert!(code.emission().controls().iter().any(|c| matches!(
                c.layout,
                EmissionLayout::Condition {
                    no: ConditionElse::None,
                    ..
                }
            )));
        },
    );
}

#[test]
fn both_diamonds_and_an_actual_parameter_phi_assignment_finish_before_cleanup() {
    inspect(
        "exceptions/pure-tail-finally",
        "Lfixture/TailPreparation;",
        "finish",
        "(IIIII)I",
        |facts, code, rendered| {
            operation_portions(facts, code);
            let plan = cleanup(facts);
            let matched = plan.finally().unwrap();
            post_cleanup_condition(code);
            let canonical = matched
                .operations()
                .iter()
                .find_map(|(_, op)| {
                    code.operations().iter().find(|source| {
                        source.source == *op && matches!(source.original, OpKind::Invoke { .. })
                    })
                })
                .expect("actual cleanup call");
            let cleanup_span = rendered
                .spans()
                .iter()
                .find(|s| {
                    matches!(s.origin,
                Origin::Operation { operation, .. } if operation == canonical.source)
                })
                .unwrap();
            let mut actual = 0;
            for transfer in code.transfers() {
                if !plan.body().contains(&transfer.from) {
                    continue;
                }
                for &copy in &code.emission().transfers()[transfer.id.0 as usize].copies {
                    let bundle = &code.copies()[copy.0 as usize];
                    if bundle.steps.is_empty() {
                        continue;
                    }
                    assert!(bundle.assignments.iter().all(|a| a.issue().is_none()));
                    let spans: Vec<_> = rendered
                        .spans()
                        .iter()
                        .filter(|s| {
                            matches!(s.origin,
                        Origin::Copy { copy: original, .. } if original == copy)
                        })
                        .collect();
                    assert_eq!(spans.len(), 1);
                    assert!(spans[0].start.0 < spans[0].end.0);
                    assert!(spans[0].end.0 <= cleanup_span.start.0);
                    actual += 1;
                }
            }
            assert!(
                actual > 0,
                "guard needs an actual pre-cleanup Assign, not only requirements"
            );
        },
    );
}

#[test]
fn literal_argument_preparation_and_both_calls_stay_in_the_cleanup_product() {
    inspect(
        "exceptions/pure-tail-finally",
        "Lfixture/TailPreparation;",
        "literal",
        "(IIIII)I",
        |facts, code, rendered| {
            operation_portions(facts, code);
            let plan = cleanup(facts);
            let matched = plan.finally().unwrap();
            assert!(matched.normal().iter().all(|n| !plan.body().contains(n)));
            let first = &facts.nodes()[matched.normal()[0].0 as usize];
            assert!(
                code.operations()
                    .iter()
                    .any(|op| op.node == first.id() && matches!(op.original, OpKind::Const { .. }))
            );
            let calls: Vec<_> = matched
                .operations()
                .iter()
                .filter_map(|(_, op)| {
                    code.operations().iter().find(|source| {
                        source.source == *op && matches!(source.original, OpKind::Invoke { .. })
                    })
                })
                .collect();
            assert_eq!(calls.len(), 2);
            let starts: Vec<_> = calls
                .iter()
                .map(|op| {
                    rendered
                        .spans()
                        .iter()
                        .find(|s| {
                            matches!(s.origin,
                Origin::Operation { operation, .. } if operation == op.source)
                        })
                        .unwrap()
                        .start
                        .0
                })
                .collect();
            assert!(starts[0] < starts[1]);
            post_cleanup_condition(code);
        },
    );
}

#[test]
fn a_switch_bypass_exits_the_try_but_a_foreign_tail_does_not_become_private() {
    inspect(
        "exceptions/pure-tail-finally",
        "Lfixture/TailPreparation;",
        "bypass",
        "(IIIII)I",
        |facts, code, rendered| {
            let plan = cleanup(facts);
            let scope = facts
                .regions()
                .iter()
                .find(|r| matches!(r.kind(), RegionKind::Try(id) if *id == plan.id()))
                .unwrap();
            let entry = plan.finally().unwrap().normal()[0];
            let exit = code.transfers().iter().find(|t| t.destination.0 == entry.0
                && matches!(t.kind, TransferKind::Break(id) if id == ControlId(scope.id().0))).expect("original cleanup target is a scoped Try completion");
            let spans: Vec<_> = rendered
                .spans()
                .iter()
                .filter(|s| {
                    matches!(s.origin,
                Origin::Transfer { transfer, .. } if transfer == exit.id)
                })
                .collect();
            assert_eq!(spans.len(), 1);
            assert!(rendered.text()[spans[0].start.0..spans[0].end.0].contains("break"));
            post_cleanup_condition(code);
        },
    );
    inspect(
        "exceptions/pure-tail-finally",
        "Lfixture/TailPreparation;",
        "foreign",
        "(III)I",
        |facts, code, rendered| {
            assert!(facts.tries().iter().all(|p| p.finally().is_none()));
            let tail = code
                .operations()
                .iter()
                .rfind(|op| matches!(op.original, OpKind::Binary { .. }))
                .unwrap();
            assert!(facts.tries().iter().all(|p| !p.body().contains(&tail.node)));
            assert!(rendered.text().contains("throw "));
        },
    );
}
