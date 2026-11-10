//! Continuation reentry must agree with the normal loop run before folding.
use diana_decomp::p2::{FieldAccess, OpKind};
use diana_decomp::p6::RegionFacts;
use diana_decomp::p7::MethodCode;
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn executable(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn recovered(facts: &RegionFacts, code: &MethodCode, rendered: &Rendered) {
    let plan = facts.coroutine_loop().expect("a closed normal-run loop");
    let owned = code
        .emission()
        .protocol()
        .expect("the delivered final layout");
    let nodes: Vec<_> = plan
        .initial()
        .iter()
        .chain(plan.body())
        .chain(plan.tail())
        .chain(plan.abi())
        .copied()
        .chain(std::iter::once(plan.condition().node()))
        .collect();
    let mut unique = nodes.clone();
    unique.sort_by_key(|n| n.0);
    unique.dedup();
    assert_eq!(nodes.len(), unique.len(), "one placement per original node");
    assert_eq!(
        unique.len(),
        facts.nodes().len(),
        "no lost original content"
    );
    for &(put, get) in plan.spills() {
        let OpKind::Field {
            access: FieldAccess::Put,
            field: saved,
            ..
        } = code.operations()[put.0 as usize].original
        else {
            panic!("a saved field");
        };
        let OpKind::Field {
            access: FieldAccess::Get,
            field: resumed,
            ..
        } = code.operations()[get.0 as usize].original
        else {
            panic!("a resumed field");
        };
        assert_eq!(saved, resumed, "original field identity");
    }
    assert!(executable(rendered.text()).contains("suspend java.lang.Object total(int"));
    assert!(executable(rendered.text()).contains("Continuation"));
    assert!(executable(rendered.text()).contains("while ("));
    assert!(!executable(rendered.text()).contains("switch ("));
    assert!(!executable(rendered.text()).contains("graph<"));
    assert!(!executable(rendered.text()).contains("goto "));
    assert!(rendered.text().contains("// ABI: switch ("));
    assert!(rendered.text().contains("// ABI: return "));
    assert!(rendered.text().contains("throwOnFailure("));
    assert!(rendered.text().contains("resume state="));
    assert!(executable(rendered.text()).contains("Boxing.boxInt("));
    for forward in &owned.forwarded_copies {
        assert_ne!(forward.destination, forward.source);
        assert!(!executable(rendered.text()).contains(&format!(
            "var {}:",
            code.locals()[forward.destination.0 as usize].name
        )));
    }
}

#[test]
fn private_suspend_call_restores_the_original_total_loop() {
    inspect(
        "kotlin/kotlin-suspend-state",
        "Lfixture/Case;",
        "total",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            recovered(facts, code, rendered);
            assert_eq!(
                code.emission().protocol().unwrap().forwarded_copies.len(),
                3
            );
            let source = executable(rendered.text());
            assert_eq!(source.matches(".load(").count(), 1);
            assert!(source.contains(".intValue() +"));
            assert!(source.contains(" + 1)"));
            assert!(
                rendered
                    .text()
                    .contains("Continuation<? super java.lang.Integer>")
            );
        },
    );
}

#[test]
fn captured_expression_and_effects_stay_on_their_original_sides_of_suspension() {
    inspect(
        "kotlin/suspend-loop-captured-effects",
        "Lfixture/SuspendLoop;",
        "total",
        "(IILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            recovered(facts, code, rendered);
            let source = executable(rendered.text());
            assert_eq!(source.matches("SuspendLoop.record(").count(), 2);
            let prepare = source.find("SuspendLoop.prepare(").unwrap();
            let first_record = source.find("SuspendLoop.record(").unwrap();
            let load = source.find("SuspendLoop.load(").unwrap();
            let last_record = source.rfind("SuspendLoop.record(").unwrap();
            assert!(prepare < first_record && first_record < load && load < last_record);
            let first_spill = facts
                .coroutine_loop()
                .unwrap()
                .spills()
                .iter()
                .map(|&(put, _)| put)
                .min_by_key(|op| op.0)
                .unwrap();
            let publication = rendered.spans().iter().find_map(|span| matches!(span.origin, Origin::Operation { operation, .. } if operation == first_spill).then_some(span.start.0)).unwrap();
            assert!(rendered.text().find("SuspendLoop.prepare(").unwrap() < publication);
            assert!(publication < rendered.text().find("SuspendLoop.load(").unwrap());
            // Static ABI operands remain retained, while only the continuation
            // occurrence is implicit in the executable call.
            let load_line = source
                .lines()
                .find(|line| line.contains("SuspendLoop.load("))
                .unwrap();
            assert!(!load_line.split(';').next().unwrap().contains(','));
        },
    );
}

#[test]
fn divergent_spills_and_resumed_effects_keep_available_explicit_code() {
    for name in ["wrongSpill", "changedCapture", "resumeEffect"] {
        inspect(
            "kotlin/suspend-loop-captured-effects",
            "Lfixture/SuspendLoop;",
            name,
            "(IILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
            |facts, code, rendered| {
                assert!(
                    facts.coroutine_loop().is_none(),
                    "{name}: no source-body correspondence"
                );
                assert!(code.emission().protocol().is_none());
                assert!(rendered.text().contains("graph<Irreducible>"));
                assert!(rendered.text().contains("SuspendLoop.load("));
                assert!(rendered.text().contains("throwOnFailure("));
                assert!(!rendered.text().contains("suspend Object"));
                let expected_records = if name == "resumeEffect" { 3 } else { 2 };
                assert_eq!(
                    rendered.text().matches("SuspendLoop.record(").count(),
                    expected_records
                );
            },
        );
    }
}
