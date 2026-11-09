//! Acyclic case-entry joins retain nonadjacent original gotos and copy barriers.

use diana_decomp::p2::NormalEdge;
use diana_decomp::p7::{ControlId, EmissionLayout, MethodCode, TransferId, TransferSpelling};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn groups(code: &MethodCode) -> Option<(&[ControlId], &[Vec<TransferId>])> {
    code.emission()
        .controls()
        .iter()
        .find_map(|c| match &c.layout {
            EmissionLayout::Switch {
                bodies,
                cases: Some(groups),
                ..
            } => Some((bodies.as_slice(), groups.as_slice())),
            _ => None,
        })
}
fn group(code: &MethodCode, key: i64) -> usize {
    groups(code)
        .unwrap()
        .1
        .iter()
        .position(|g| {
            g.iter().any(|t| {
                matches!(code.transfers()[t.0 as usize].role,
        Some(NormalEdge::SwitchCase { key: k, .. }) if k == key)
            })
        })
        .unwrap()
}

#[test]
fn three_predecessors_share_one_case_body_with_exact_remaining_gotos() {
    inspect(
        "branches/case-entry-joins",
        "Lfixture/CaseEntryJoins;",
        "choice",
        "(II)I",
        |_, code, rendered| {
            let (bodies, cases) = groups(code).expect("acyclic case-entry schedule");
            assert_eq!(cases.iter().map(Vec::len).sum::<usize>(), 6);
            assert_eq!(group(code, 5) + 1, group(code, 3));
            let target = code.controls()[bodies[group(code, 3)].0 as usize]
                .entry
                .unwrap();
            let gotos = code
                .emission()
                .transfers()
                .iter()
                .filter(|t| {
                    matches!(t.spelling,
                TransferSpelling::Goto(label) if label == target)
                })
                .count();
            assert_eq!(gotos, 2, "other predecessors skip unrelated case work");
            assert_eq!(rendered.text().matches(".mark(3)").count(), 1);
            assert!(!rendered.text().contains("graph<"));
        },
    );
}

#[test]
fn direct_selector_phi_copy_is_not_run_on_predecessor_entry() {
    inspect(
        "branches/case-entry-joins",
        "Lfixture/CaseEntryJoins;",
        "parameterJoin",
        "(II)I",
        |_, code, rendered| {
            assert!(groups(code).is_none());
            let entry = code
                .transfers()
                .iter()
                .find(|t| matches!(t.role, Some(NormalEdge::SwitchCase { key: 3, .. })))
                .expect("original direct case3 selector");
            assert!(
                code.emission().transfers()[entry.id.0 as usize]
                    .copies
                    .iter()
                    .any(|p| !code.copies()[p.0 as usize].steps.is_empty())
            );
            assert_eq!(rendered.text().matches(".mark(3)").count(), 1);
        },
    );
}

#[test]
fn a_cycle_with_empty_selector_copies_is_not_a_case_dag() {
    inspect(
        "branches/case-entry-joins",
        "Lfixture/CaseEntryJoins;",
        "literalCycle",
        "(I)I",
        |_, code, rendered| {
            assert!(groups(code).is_none());
            assert!(rendered.text().contains("graph<"));
            let selected: Vec<_> = code
                .transfers()
                .iter()
                .filter(|t| matches!(t.role, Some(NormalEdge::SwitchCase { .. })))
                .collect();
            assert_eq!(selected.len(), 2);
            assert!(
                selected
                    .iter()
                    .all(|t| code.emission().transfers()[t.id.0 as usize]
                        .copies
                        .iter()
                        .all(|p| {
                            let bundle = &code.copies()[p.0 as usize];
                            bundle.steps.is_empty()
                                && bundle.assignments.iter().all(|a| a.issue().is_none())
                        })),
                "cycle guard is not merely blocked by an executable entry copy"
            );
            assert_eq!(rendered.text().matches(".mark(1)").count(), 1);
            assert_eq!(rendered.text().matches(".mark(2)").count(), 1);
        },
    );
}

#[test]
fn non_case_sharing_and_protected_recovery_keep_original_fragments() {
    inspect(
        "branches/case-entry-joins",
        "Lfixture/CaseEntryJoins;",
        "nonCaseTail",
        "(II)I",
        |_, code, rendered| {
            assert!(
                groups(code).is_none(),
                "shared tail is not a selector entry"
            );
            assert!(rendered.text().contains("-2"));
        },
    );
    inspect(
        "branches/case-entry-joins",
        "Lfixture/CaseEntryJoins;",
        "route",
        "(III)I",
        |_, _, rendered| {
            assert!(rendered.text().contains(".cleanup(0, p2)"));
            assert!(rendered.text().contains(".mark(40)"));
            assert!(rendered.text().contains("throw "));
        },
    );
}
