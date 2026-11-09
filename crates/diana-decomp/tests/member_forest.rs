//! Composite/member-forest checks use independently traversed child ownership.

use diana_decomp::p6::{RegionFacts, RegionKind};
use diana_decomp::p9::{Origin, Rendered};
#[path = "support/inspection.rs"]
mod inspection;
#[path = "support/selected_execution.rs"]
mod selected_execution;
use selected_execution::{Executor, Fault, Value};

fn membership(facts: &RegionFacts) {
    let root = facts.region_nodes(facts.root().unwrap()).unwrap();
    let mut original = root.to_vec();
    original.sort_unstable();
    assert_eq!(
        original,
        facts
            .nodes()
            .iter()
            .map(|node| node.id())
            .collect::<Vec<_>>()
    );
    for region in facts.regions() {
        let actual = facts
            .region_nodes(region.id())
            .expect("complete source membership");
        let mut work = vec![region.id()];
        let mut expected = Vec::new();
        while let Some(id) = work.pop() {
            let child = &facts.regions()[id.0 as usize];
            if let RegionKind::Block(node) = child.kind() {
                expected.push(*node);
            }
            work.extend_from_slice(child.children());
        }
        expected.sort_unstable();
        let mut sorted = actual.to_vec();
        sorted.sort_unstable();
        assert_eq!(sorted, expected, "original leaf set for {:?}", region.id());
        if let Some(first) = actual.first() {
            let start = root.iter().position(|node| node == first).unwrap();
            assert_eq!(&root[start..start + actual.len()], actual);
            assert!(
                std::ptr::eq(&root[start], first),
                "members borrow one shared buffer"
            );
        }
    }
}

fn origins(rendered: &Rendered, source: &diana_decomp::p7::MethodCode) {
    for op in source.operations() {
        assert_eq!(
            rendered
                .spans()
                .iter()
                .filter(
                    |span| matches!(span.origin, Origin::Operation { operation, pc, .. }
                if operation == op.source && pc == op.pc)
                )
                .count(),
            1
        );
    }
}

#[test]
fn physically_interleaved_arms_share_members_without_changing_execution() {
    inspection::inspect(
        "composite/member-forest",
        "Lfixture/MemberForest;",
        "interleaved",
        "(I)I",
        |facts, code, rendered| {
            membership(facts);
            origins(rendered, code);
            assert!(
                facts.regions().iter().any(|region| {
                    let mut nodes = facts.region_nodes(region.id()).unwrap().to_vec();
                    nodes.sort_unstable();
                    nodes.windows(2).any(|pair| pair[1].0 > pair[0].0 + 1)
                }),
                "the input retains a noncontiguous original arm"
            );
            for (flag, result) in [(0, 7), (1, 4), (-1, 4)] {
                let mut executor = Executor::new(code, &[Value::Int(flag)], |_, _| {
                    Err(Fault::Unsupported("unexpected interleaved helper"))
                });
                assert_eq!(executor.run(), Ok(Value::Int(result)));
            }
        },
    );
}

#[test]
fn loop_switch_and_handler_members_keep_one_leaf_and_effect_owner() {
    inspection::inspect(
        "composite/member-forest",
        "Lfixture/MemberForest;",
        "nested",
        "(II)I",
        |facts, code, rendered| {
            membership(facts);
            origins(rendered, code);
            for predicate in [
                |kind: &RegionKind| matches!(kind, RegionKind::Loop(_)),
                |kind: &RegionKind| matches!(kind, RegionKind::Switch { .. }),
                |kind: &RegionKind| matches!(kind, RegionKind::Try(_)),
            ] {
                assert_eq!(
                    facts
                        .regions()
                        .iter()
                        .filter(|region| predicate(region.kind()))
                        .count(),
                    1
                );
            }
            let text = rendered.text();
            assert!(text.contains("while (") && text.contains("catch (java.lang.RuntimeException"));
            assert!(text.contains("case 0:") && text.contains("case 1:"));
            assert_eq!(text.matches("MemberForest.tick(").count(), 1);
        },
    );
}

#[test]
fn a_locked_loop_keeps_release_and_original_membership() {
    inspection::inspect(
        "composite/member-forest",
        "Lfixture/MemberForest;",
        "locked",
        "(Ljava/lang/Object;I)I",
        |facts, code, rendered| {
            membership(facts);
            origins(rendered, code);
            assert_eq!(
                facts
                    .regions()
                    .iter()
                    .filter(|region| matches!(region.kind(), RegionKind::Synchronized(_)))
                    .count(),
                1
            );
            assert!(
                rendered.text().contains("synchronized (") && rendered.text().contains("while (")
            );
            assert_eq!(rendered.text().matches("MemberForest.tick(").count(), 1);
        },
    );
}
