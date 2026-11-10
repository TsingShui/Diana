//! Multi-site loop syntax consumes existing SCC and original transfer copies.
use diana_decomp::p7::{EmissionLayout, LoopPlacement, TransferSpelling};
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
fn two_reentries_recover_the_loop_and_keep_the_negative_exit() {
    inspect(
        "kotlin/suspend-loop-two-sites",
        "Lfixture/SuspendTwoSites;",
        "total",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            assert!(
                facts.coroutine_loop().is_none(),
                "multi-site input must not use the single-site loop plan"
            );
            let plan = facts.coroutine_flow().unwrap();
            let cycle = plan.cycle().unwrap();
            let header = code
                .controls()
                .iter()
                .find_map(|control| match control.layout {
                    EmissionLayout::Loop {
                        placement: LoopPlacement::While { predicate, .. },
                        ..
                    } => Some(predicate),
                    _ => None,
                })
                .expect("ordinary loop control");
            assert_eq!(plan.sites().len(), 2);
            assert!(!cycle.body().is_empty());
            assert_eq!(header.0, cycle.header().0);
            assert!(matches!(
                code.emission().transfers()[cycle.implicit_latch().0 as usize].spelling,
                TransferSpelling::Fallthrough
            ));
            assert!(matches!(
                code.emission().transfers()[cycle.initial_header().0 as usize].spelling,
                TransferSpelling::Fallthrough
            ));
            let source = executable(rendered.text());
            assert!(source.contains("suspend java.lang.Object total("));
            assert!(source.contains("while ("));
            assert!(!source.contains("switch (") && !source.contains("graph<"));
            assert_eq!(source.matches("SuspendTwoSites.load(").count(), 2);
            assert_eq!(source.matches("SuspendTwoSites.record(").count(), 1);
            assert!(source.contains(" < 0)"));
            assert!(source.contains("[exit:"));
        },
    );
}

#[test]
fn checksum_captures_and_effects_stay_on_the_original_sides_of_suspension() {
    inspect(
        "kotlin/suspend-loop-two-site-checksum",
        "Lfixture/SuspendChecksum;",
        "total",
        "(IILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            let plan = facts.coroutine_flow().unwrap();
            assert!(plan.cycle().is_some());
            assert_eq!(plan.sites().len(), 2);
            assert!(
                plan.sites()
                    .iter()
                    .find(|s| s.state() == 2)
                    .unwrap()
                    .spills()
                    .len()
                    >= 4
            );
            assert!(
                code.controls()
                    .iter()
                    .any(|control| matches!(control.layout, EmissionLayout::Loop { .. }))
            );
            let source = executable(rendered.text());
            assert!(
                source.contains("while (")
                    && !source.contains("switch (")
                    && !source.contains("graph<")
            );
            assert_eq!(source.matches("SuspendChecksum.load(").count(), 2);
            assert_eq!(source.matches("SuspendChecksum.record(").count(), 2);
            let first = source.find("SuspendChecksum.record(").unwrap();
            let last = source.rfind("SuspendChecksum.record(").unwrap();
            let load = source.rfind("SuspendChecksum.load(").unwrap();
            assert!(first < load && load < last);
            assert!(source.contains("[exit:"));
        },
    );
}
