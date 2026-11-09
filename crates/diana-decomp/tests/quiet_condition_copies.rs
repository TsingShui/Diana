//! Origin-only edge requirements are not executable else statements.

use diana_decomp::p7::{ConditionElse, CopyPlanId, EmissionLayout, MethodCode, TransferId};
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn anchors(code: &MethodCode, rendered: &Rendered, edge: TransferId, executable: bool) {
    let transfers: Vec<_> = rendered
        .spans()
        .iter()
        .filter(|s| {
            matches!(s.origin,
        Origin::Transfer { transfer, .. } if transfer == edge)
        })
        .collect();
    assert_eq!(transfers.len(), 1, "one original final-other occurrence");
    assert_eq!(transfers[0].start.0 < transfers[0].end.0, executable);
    let copies = &code.emission().transfers()[edge.0 as usize].copies;
    assert!(
        !copies.is_empty(),
        "actual original obligations, not an empty list"
    );
    for &copy in copies {
        let bundle = &code.copies()[copy.0 as usize];
        assert!(!bundle.assignments.is_empty());
        assert!(bundle.assignments.iter().all(|a| a.issue().is_none()));
        assert_eq!(!bundle.steps.is_empty(), executable);
        let spans: Vec<_> = rendered
            .spans()
            .iter()
            .filter(|s| {
                matches!(s.origin,
            Origin::Copy { copy: CopyPlanId(id), .. } if id == copy.0)
            })
            .collect();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].start.0 < spans[0].end.0, executable);
    }
}

#[test]
fn a_quiet_loop_continuation_needs_origins_but_no_empty_else() {
    inspect(
        "loops/mutable-iteration-step",
        "Lfixture/MutableStep;",
        "walk",
        "(III)I",
        |_, code, rendered| {
            let quiet: Vec<_> = code
                .emission()
                .controls()
                .iter()
                .filter_map(|c| match c.layout {
                    EmissionLayout::Condition {
                        no: ConditionElse::None,
                        fall,
                        ..
                    } if !code.emission().transfers()[fall.0 as usize]
                        .copies
                        .is_empty() =>
                    {
                        Some(fall)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(quiet.len(), 1);
            anchors(code, rendered, quiet[0], false);
            assert!(code.emission().controls().iter().any(|c| matches!(
                c.layout,
                EmissionLayout::Condition {
                    no: ConditionElse::Transfer,
                    ..
                }
            )));
        },
    );
}

#[test]
fn consecutive_conditions_distinguish_identity_requirements_from_real_phi_assignments() {
    inspect(
        "branches/quiet-condition-copies",
        "Lfixture/QuietCopies;",
        "quiet",
        "(IIII)I",
        |_, code, rendered| {
            let arms: Vec<_> = code
                .emission()
                .controls()
                .iter()
                .filter_map(|c| match c.layout {
                    EmissionLayout::Condition { no, fall, .. } => Some((no, fall)),
                    _ => None,
                })
                .collect();
            assert_eq!(arms.len(), 2);
            let quiet = arms
                .iter()
                .find(|(no, _)| matches!(no, ConditionElse::None))
                .unwrap();
            let actual = arms
                .iter()
                .find(|(no, _)| matches!(no, ConditionElse::Copies))
                .unwrap();
            anchors(code, rendered, quiet.1, false);
            anchors(code, rendered, actual.1, true);
        },
    );
}

#[test]
fn call_result_join_and_parameter_join_keep_both_executable_false_arms() {
    inspect(
        "branches/quiet-condition-copies",
        "Lfixture/QuietCopies;",
        "pair",
        "(IIII)I",
        |_, code, rendered| {
            let arms: Vec<_> = code
                .emission()
                .controls()
                .iter()
                .filter_map(|c| match c.layout {
                    EmissionLayout::Condition {
                        no: ConditionElse::Copies,
                        fall,
                        ..
                    } => Some(fall),
                    _ => None,
                })
                .collect();
            assert_eq!(arms.len(), 2);
            for edge in arms {
                anchors(code, rendered, edge, true);
            }
        },
    );
}
