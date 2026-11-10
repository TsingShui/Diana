//! Closed multi-site bodies retain original controls and exact copy delivery.
use diana_decomp::p7::{BlockEnd, BodyAction};
use diana_decomp::p9::Origin;
#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn executable(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn two_sites_keep_the_original_negative_branch_and_call_order() {
    inspect(
        "kotlin/suspend-two-step-branch",
        "Lfixture/SuspendBranch;",
        "choose",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            let plan = facts.protocol().expect("closed multi-site source body");
            let flow = code.emission().protocol().unwrap();
            assert_eq!(flow.calls.len(), 2);
            let leaves = inspection::protocol_leaves(code);
            let mut ownership = vec![0; facts.nodes().len()];
            for &(node, _) in &leaves {
                ownership[node.0 as usize] += 1;
            }
            assert_eq!(ownership, vec![1; facts.nodes().len()]);
            let source = executable(rendered.text());
            assert!(source.contains("suspend java.lang.Object choose(int"));
            assert!(!source.contains("switch ("));
            assert!(!source.contains("goto "));
            assert_eq!(source.matches("SuspendBranch.load(").count(), 2);
            assert_eq!(source.matches("SuspendBranch.record(").count(), 2);
            assert!(source.find("SuspendBranch.record(").unwrap() < source.find(" < 0)").unwrap());
            assert!(source.contains("Boxing.boxInt(-1)"));
            assert!(source.contains("var ") && source.contains(": kotlin.Int"));
            for site in &plan.calls {
                let BlockEnd::Annotated {
                    terminal,
                    normal,
                    suspended,
                    ..
                } = &code.emission().blocks()[site.guard.0 as usize].end
                else {
                    panic!("guard owns original correspondence and the normal transfer");
                };
                assert_eq!(normal.0, site.normal.0);
                assert_eq!(suspended.0, site.suspended.0);
                assert_eq!(
                    rendered
                        .spans()
                        .iter()
                        .filter(
                            |s| matches!(s.origin,Origin::Terminal{block,..}if block==*terminal)
                        )
                        .count(),
                    1
                );
                assert_eq!(
                    rendered
                        .spans()
                        .iter()
                        .filter(
                            |s| matches!(s.origin,Origin::Transfer{transfer,..}if transfer==*normal)
                        )
                        .count(),
                    1
                );
            }
            for event in &plan.operations {
                let op = event.operation;
                let node = code.operations()[op.0 as usize].node;
                if code.operations()[op.0 as usize].statement.is_some() {
                    assert!(code.emission().blocks()[node.0 as usize].actions.iter().any(|a|matches!(a,BodyAction::AbiOperation{operation,..}if *operation==op)));
                }
            }
        },
    );
}

#[test]
fn three_sites_keep_two_captures_and_both_early_returns() {
    inspect(
        "kotlin/suspend-three-step-effects",
        "Lfixture/SuspendThree;",
        "walk",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            let plan = facts.protocol().unwrap();
            assert_eq!(plan.calls.len(), 3);
            assert_eq!(
                plan.calls
                    .iter()
                    .find(|s| s.state == 3)
                    .unwrap()
                    .spills
                    .len(),
                2
            );
            let source = executable(rendered.text());
            assert!(code.emission().protocol().is_some());
            assert_eq!(source.matches("SuspendThree.load(").count(), 3);
            assert_eq!(source.matches("SuspendThree.record(").count(), 3);
            assert!(source.contains(" < 0)") && source.contains(" == 0)"));
            assert!(!source.contains("switch (") && !source.contains("goto "));
            assert!(rendered.text().contains("case 3: goto "));
        },
    );
}

#[test]
fn normal_only_effect_cannot_become_shared_source_work() {
    inspect(
        "kotlin/suspend-three-step-effects",
        "Lfixture/SuspendThree;",
        "normalOnly",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            assert!(facts.protocol().is_none());
            assert!(code.emission().protocol().is_none());
            assert!(rendered.text().contains("switch ("));
            assert!(rendered.text().contains("SuspendThree.record(777)"));
            assert!(
                !rendered
                    .text()
                    .contains("suspend java.lang.Object normalOnly")
            );
        },
    );
}

#[test]
fn compiled_fetch_and_private_marker_returns_consume_selected_guard_owners() {
    inspect(
        "kotlin/kotlin-suspend-state",
        "Lfixture/Case;",
        "fetch",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            assert_eq!(facts.protocol().unwrap().calls.len(), 2);
            assert_eq!(code.emission().protocol().unwrap().calls.len(), 2);
            let source = executable(rendered.text());
            assert!(source.contains("suspend java.lang.Object fetch("));
            assert_eq!(source.matches("this.load(").count(), 2);
            assert!(!source.contains("switch (") && !source.contains("goto "));
            assert!(source.contains("Continuation<? super java.lang.Integer>"));
        },
    );
    inspect(
        "kotlin/suspend-three-step-effects",
        "Lfixture/SuspendThree;",
        "privateReturns",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            assert_eq!(facts.protocol().unwrap().calls.len(), 3);
            assert_eq!(code.emission().protocol().unwrap().calls.len(), 3);
            assert_eq!(
                executable(rendered.text())
                    .matches("SuspendThree.load(")
                    .count(),
                3
            );
            assert!(!executable(rendered.text()).contains("switch ("));
            assert!(rendered.text().matches("// ABI: return ").count() >= 3);
            for site in &facts.protocol().unwrap().calls {
                for id in [site.normal, site.suspended] {
                    let destination = code.transfers()[id.0 as usize].destination;
                    assert!(
                        rendered
                            .text()
                            .contains(&format!("{}:", code.labels()[destination.0 as usize].name))
                    );
                }
            }
        },
    );
}

#[test]
fn sparse_non_monotone_states_do_not_reintroduce_dispatch_context() {
    inspect(
        "kotlin/suspend-three-step-effects",
        "Lfixture/SuspendThree;",
        "reorderedStates",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            let plan = facts.protocol().unwrap();
            assert_eq!(
                plan.calls.iter().map(|s| s.state).collect::<Vec<_>>(),
                [9, 2, 5]
            );
            assert_eq!(code.emission().protocol().unwrap().calls.len(), 3);
            let source = executable(rendered.text());
            assert_eq!(source.matches("SuspendThree.load(").count(), 3);
            assert!(
                !source.contains("switch (")
                    && !source.contains("goto ")
                    && !source.contains("break;")
            );
            assert!(rendered.text().contains("case 9: goto "));
        },
    );
}
