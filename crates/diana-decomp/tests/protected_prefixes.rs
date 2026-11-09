//! Optional prefix completion retains guarded seeds and exact entry frontiers.

use diana_decomp::p6::{NodeId, RegionFacts, RegionKind, TryPlan};
use diana_decomp::p7::TerminalKind;

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn typed(facts: &RegionFacts) -> &TryPlan {
    facts
        .tries()
        .iter()
        .find(|p| {
            let r = p.clauses();
            r.end - r.start == 2 && p.handlers().len() == 1
        })
        .expect("one adjacent typed scope delivered by P6")
}

#[test]
fn an_optional_goto_prefix_does_not_destroy_the_guarded_core_entry() {
    inspect(
        "branches/case-entry-joins",
        "Lfixture/CaseEntryJoins;",
        "route",
        "(III)I",
        |facts, _, rendered| {
            let plan = typed(facts);
            let sites: Vec<_> = facts
                .throws()
                .iter()
                .filter(|t| t.chain() == plan.chain())
                .collect();
            assert_eq!(sites.len(), 1);
            assert_eq!(plan.entry(), sites[0].node());
            assert!(sites.iter().all(|t| plan.body().contains(&t.node())));
            assert!(
                facts
                    .regions()
                    .iter()
                    .any(|r| matches!(r.kind(), RegionKind::Try(id) if *id == plan.id()))
            );
            assert!(!rendered.text().contains("graph<"));
        },
    );
}

#[test]
fn a_branched_core_grows_a_common_prefix_without_absorbing_foreign_throws() {
    inspect(
        "exceptions/protected-prefix-frontiers",
        "Lfixture/PrefixFrontiers;",
        "branch",
        "(III)I",
        |facts, code, rendered| {
            let plan = typed(facts);
            let sites: Vec<_> = facts
                .throws()
                .iter()
                .filter(|t| t.chain() == plan.chain())
                .collect();
            assert_eq!(sites.len(), 2);
            assert!(sites.iter().all(|t| plan.body().contains(&t.node())));
            let entry = &facts.nodes()[plan.entry().0 as usize];
            assert!(
                entry.has_terminal(),
                "common original flag test is the entry"
            );
            assert!(matches!(
                code.terminals()[entry.block().0 as usize].kind,
                TerminalKind::If { .. }
            ));
            assert!(
                facts
                    .throws()
                    .iter()
                    .filter(|t| plan.body().contains(&t.node()))
                    .all(|t| t.chain() == plan.chain()),
                "outer-only throwing prefixes remain outside the typed scope"
            );
            assert!(!rendered.text().contains("graph<"));
        },
    );
}

#[test]
fn a_real_multi_entry_guarded_set_keeps_both_original_read_sites() {
    inspect(
        "exceptions/protected-prefix-frontiers",
        "Lfixture/PrefixFrontiers;",
        "split",
        "(II)I",
        |facts, _, rendered| {
            assert!(!facts.tries().iter().any(|p| {
                let r = p.clauses();
                r.end - r.start == 2
            }));
            let sites: Vec<_> = facts
                .throws()
                .iter()
                .filter(|t| facts.handler_chains()[t.chain().0 as usize].clauses().len() == 3)
                .collect();
            assert_eq!(
                sites.len(),
                2,
                "no guarded seed is dropped to fabricate one entry"
            );
            assert_ne!(sites[0].node(), sites[1].node());
            assert!(rendered.text().contains("graph<ProtectionBoundary>"));
            assert_eq!(rendered.text().matches("PrefixFrontiers.read(").count(), 2);
        },
    );
}

#[test]
fn complete_outer_prefix_keeps_method_entry_and_nested_guarded_coverage() {
    inspect(
        "exceptions/protected-prefix-frontiers",
        "Lfixture/PrefixFrontiers;",
        "branch",
        "(III)I",
        |facts, _, _| {
            let inner = typed(facts);
            let outer = facts
                .tries()
                .iter()
                .find(|p| p.entry() == NodeId(0) && p.clauses().end - p.clauses().start == 1)
                .expect("full original method-entry prefix remains available");
            assert!(inner.members().iter().all(|n| outer.body().contains(n)));
            assert!(
                facts
                    .throws()
                    .iter()
                    .filter(|t| t.chain() == inner.chain() || t.chain() == outer.chain())
                    .all(|t| outer.body().contains(&t.node()))
            );
            assert!(
                facts
                    .regions()
                    .iter()
                    .any(|r| matches!(r.kind(), RegionKind::Try(id) if *id==outer.id()))
            );
        },
    );
}
