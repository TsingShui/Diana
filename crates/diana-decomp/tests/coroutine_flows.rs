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
            let plan = facts
                .coroutine_flow()
                .expect("closed multi-site source body");
            let flow = code.emission().coroutine_flow().unwrap();
            assert_eq!(flow.sites.len(), 2);
            assert_eq!(
                plan.source_nodes().len() + plan.abi().len(),
                facts.nodes().len()
            );
            assert!(plan.source_nodes().iter().all(|n| !plan.abi().contains(n)));
            let source = executable(rendered.text());
            assert!(source.contains("suspend java.lang.Object choose(int"));
            assert!(!source.contains("switch ("));
            assert!(!source.contains("goto "));
            assert_eq!(source.matches("SuspendBranch.load(").count(), 2);
            assert_eq!(source.matches("SuspendBranch.record(").count(), 2);
            assert!(source.find("SuspendBranch.record(").unwrap() < source.find(" < 0)").unwrap());
            assert!(source.contains("Boxing.boxInt(-1)"));
            assert!(source.contains("var ") && source.contains(": kotlin.Int"));
            for site in &flow.sites {
                let BlockEnd::Transfers(links) =
                    &code.emission().blocks()[site.guard.0 as usize].end
                else {
                    panic!("guard owns one original normal transfer");
                };
                assert_eq!(links.as_slice(), [site.normal]);
                assert_eq!(rendered.spans().iter().filter(|s|matches!(s.origin,Origin::Terminal{block,..}if block==site.predicate)).count(),1);
                assert_eq!(rendered.spans().iter().filter(|s|matches!(s.origin,Origin::Transfer{transfer,..}if transfer==site.normal)).count(),1);
            }
            for &op in plan.protocol_operations() {
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
            let plan = facts.coroutine_flow().unwrap();
            assert_eq!(plan.sites().len(), 3);
            assert_eq!(
                plan.sites()
                    .iter()
                    .find(|s| s.state() == 3)
                    .unwrap()
                    .spills()
                    .len(),
                2
            );
            let source = executable(rendered.text());
            assert!(code.emission().coroutine_flow().is_some());
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
            assert!(facts.coroutine_flow().is_none());
            assert!(code.emission().coroutine_flow().is_none());
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
            assert_eq!(facts.coroutine_flow().unwrap().sites().len(), 2);
            assert_eq!(code.emission().coroutine_flow().unwrap().sites.len(), 2);
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
            assert_eq!(facts.coroutine_flow().unwrap().sites().len(), 3);
            assert_eq!(code.emission().coroutine_flow().unwrap().sites.len(), 3);
            assert_eq!(
                executable(rendered.text())
                    .matches("SuspendThree.load(")
                    .count(),
                3
            );
            assert!(!executable(rendered.text()).contains("switch ("));
            assert!(rendered.text().matches("// ABI: return ").count() >= 3);
            for site in &code.emission().coroutine_flow().unwrap().sites {
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
