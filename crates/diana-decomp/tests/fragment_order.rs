//! Ordinary Engine coverage for fragment-order and its loop/switch challenge.

use diana_decomp::p6::{RegionFacts, RegionKind};
use diana_decomp::p7::MethodCode;
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;

fn coverage(facts: &RegionFacts, code: &MethodCode, rendered: &Rendered) {
    let mut leaves = vec![0; facts.nodes().len()];
    let mut stack = vec![facts.root().expect("complete original root")];
    while let Some(id) = stack.pop() {
        let region = &facts.regions()[id.0 as usize];
        if let RegionKind::Block(node) = region.kind() {
            leaves[node.0 as usize] += 1;
        }
        for &child in region.children() {
            assert_eq!(facts.parent(child), Some(id), "one static execution owner");
            stack.push(child);
        }
    }
    assert!(leaves.iter().all(|&count| count == 1));
    for operation in code.operations() {
        assert_eq!(
            rendered
                .spans()
                .iter()
                .filter(|span| matches!(span.origin,
                Origin::Operation { operation: actual, pc, .. }
                    if actual == operation.source && pc == operation.pc))
                .count(),
            1,
            "one original operation anchor"
        );
    }
    assert!(!rendered.text().contains("for ("));
}

#[test]
fn a_guarded_loop_keeps_its_update_and_source_continuation() {
    inspection::inspect(
        "branches/fragment-order",
        "Lfixture/FragmentOrder;",
        "branchLoop",
        "(I)I",
        |facts, code, rendered| {
            coverage(facts, code, rendered);
            assert_eq!(code.operations().len(), 4);
            assert_eq!(
                facts
                    .regions()
                    .iter()
                    .filter(|region| matches!(region.kind(), RegionKind::Loop(_)))
                    .count(),
                1
            );
            assert!(rendered.text().contains("return 7;"));
            assert!(rendered.text().contains("while ("));
            assert_eq!(rendered.text().matches(" + 1").count(), 1);
        },
    );
}

#[test]
fn two_cycle_entries_remain_explicit_and_each_update_has_one_owner() {
    inspection::inspect(
        "branches/fragment-order",
        "Lfixture/FragmentOrder;",
        "irreducible",
        "(I)I",
        |facts, code, rendered| {
            coverage(facts, code, rendered);
            assert_eq!(code.operations().len(), 4);
            assert!(facts.regions().iter().any(|region| matches!(
                region.kind(),
                RegionKind::Graph {
                    reason: diana_decomp::p6::RecoveryReason::Irreducible,
                    ..
                }
            )));
            assert!(
                !facts
                    .regions()
                    .iter()
                    .any(|region| matches!(region.kind(), RegionKind::Loop(_)))
            );
            assert!(rendered.text().contains("goto "));
            assert_eq!(rendered.text().matches(" + 1").count(), 1);
            assert_eq!(rendered.text().matches(" + 2").count(), 1);
        },
    );
}

#[test]
fn an_infinite_loop_does_not_make_analysis_repeat() {
    inspection::inspect(
        "branches/fragment-order",
        "Lfixture/FragmentOrder;",
        "selfCycle",
        "()V",
        |facts, code, rendered| {
            coverage(facts, code, rendered);
            assert!(rendered.text().contains("while (true)"));
            assert!(!rendered.text().contains("return"));
        },
    );
}

#[test]
fn aliased_switch_edges_inside_a_loop_share_one_body() {
    inspection::inspect(
        "composite/fragment-order-loops",
        "Lfixture/FragmentOrderLoops;",
        "nestedSwitch",
        "(II)I",
        |facts, code, rendered| {
            coverage(facts, code, rendered);
            assert_eq!(code.operations().len(), 6);
            assert_eq!(
                facts
                    .regions()
                    .iter()
                    .filter(|region| matches!(region.kind(), RegionKind::Loop(_)))
                    .count(),
                1
            );
            assert_eq!(
                facts
                    .regions()
                    .iter()
                    .filter(|region| matches!(region.kind(), RegionKind::Switch { .. }))
                    .count(),
                1
            );
            let text = rendered.text();
            assert!(
                text.contains("while (") && text.contains("switch (") && text.contains("return 7;")
            );
            assert_eq!(text.matches("case 0:").count(), 1);
            assert_eq!(text.matches("case 1:").count(), 1);
            assert_eq!(text.matches("case 2:").count(), 1);
            assert_eq!(
                text.matches(" + 1").count(),
                2,
                "default and one shared case body"
            );
            assert_eq!(text.matches(" + 2").count(), 1);
        },
    );
}
