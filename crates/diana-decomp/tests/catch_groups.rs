//! Catch ownership and publication checks through ordinary Engine delivery.
//! Independent scenarios and semantic reviews stay beside the prepared inputs.

use diana_decomp::p6::{CatchTarget, RegionFacts, RegionKind, SiteProtection};
use diana_decomp::p9::Rendered;

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn grouped(facts: &RegionFacts, rendered: &Rendered) {
    let plan = facts
        .tries()
        .iter()
        .find(|p| p.handler_clauses().iter().any(|r| r.end - r.start == 2))
        .expect("selected adjacent catch aliases");
    let region = facts
        .regions()
        .iter()
        .find(|r| matches!(r.kind(), RegionKind::Try(id) if *id == plan.id()))
        .expect("grouped plan has a selected natural region");
    assert_eq!(plan.handlers().len(), plan.handler_clauses().len());
    assert_eq!(plan.handlers().len(), plan.handler_bodies().len());
    let flattened: Vec<_> = plan
        .handler_clauses()
        .iter()
        .flat_map(Clone::clone)
        .collect();
    assert_eq!(flattened, plan.clauses().collect::<Vec<_>>());
    let clauses = facts.handler_chains()[plan.chain().0 as usize].clauses();
    for (range, target) in plan.handler_clauses().iter().zip(plan.handlers()) {
        for clause in &clauses[range.start as usize..range.end as usize] {
            assert_eq!(clause.target, CatchTarget::Handler(*target));
        }
    }
    let mut parent = facts.parent(region.id());
    let mut outer = None;
    while let Some(id) = parent {
        if let RegionKind::Try(try_id) = facts.regions()[id.0 as usize].kind() {
            let p = &facts.tries()[try_id.0 as usize];
            if p.body().iter().any(|n| plan.handlers().contains(n)) {
                outer = Some(p);
                break;
            }
        }
        parent = facts.parent(id);
    }
    let outer = outer.expect("typed catch bodies retain enclosing cleanup protection");
    assert!(plan.members().iter().all(|n| outer.body().contains(n)));
    assert!(outer.handlers().iter().all(|n| !plan.members().contains(n)));
    for site in facts.throws() {
        if facts.handler_chains()[site.chain().0 as usize]
            .clauses()
            .iter()
            .any(|c| matches!(c.target, CatchTarget::Handler(_)))
        {
            assert_eq!(site.protection(), SiteProtection::Natural, "{site:?}");
        }
    }
    assert_eq!(
        rendered
            .text()
            .lines()
            .filter(|line| line.trim_start().starts_with("catch (")
                && line.contains("NumberFormatException")
                && line.contains("NullPointerException")
                && line.contains(" | "))
            .count(),
        1,
        "{}",
        rendered.text()
    );
    assert_eq!(
        rendered
            .text()
            .lines()
            .filter(|line| line.trim_start().starts_with("catch (")
                && line.contains("RuntimeException"))
            .count(),
        1,
        "{}",
        rendered.text()
    );
}

#[test]
fn shared_handler_and_outer_cleanup_have_distinct_owners() {
    inspect(
        "exceptions/typed-catch-shared-cleanup",
        "Lfixture/TypedCatchSharedCleanup;",
        "classify",
        "(Ljava/lang/String;I)I",
        |facts, _, rendered| grouped(facts, rendered),
    );
}

#[test]
fn loop_backedge_does_not_absorb_the_outer_cleanup_handler() {
    inspect(
        "exceptions/typed-catch-loop-exits",
        "Lfixture/LoopCatchFinally;",
        "scan",
        "(II)I",
        |facts, _, rendered| grouped(facts, rendered),
    );
}

#[test]
fn nonadjacent_aliases_do_not_reorder_catch_priority() {
    inspect(
        "exceptions/typed-catch-loop-exits",
        "Lfixture/LoopCatchFinally;",
        "priority",
        "(I)I",
        |facts, _, rendered| {
            assert!(
                !rendered
                    .text()
                    .lines()
                    .any(|line| line.trim_start().starts_with("catch (")
                        && line.contains("NumberFormatException")
                        && line.contains("NullPointerException")
                        && line.contains(" | "))
            );
            let chain = facts
                .handler_chains()
                .iter()
                .find(|c| c.clauses().len() == 4)
                .unwrap();
            assert_eq!(chain.clauses()[0].target, chain.clauses()[2].target);
            assert_ne!(chain.clauses()[0].target, chain.clauses()[1].target);
            assert!(rendered.text().contains("exception_chain"));
            for site in facts
                .throws()
                .iter()
                .filter(|site| site.chain() == chain.id())
            {
                assert_eq!(site.protection(), SiteProtection::Explicit);
            }
        },
    );
}
