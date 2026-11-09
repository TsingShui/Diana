//! Finally correspondence keeps selected scope, body copies and cleanup values.

use diana_decomp::p5::{CopyLocation, TransferSite};
use diana_decomp::p6::{EdgeRoute, RegionKind};
use diana_decomp::p9::Origin;

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

#[test]
fn suffix_finally_keeps_body_entry_copies_before_cleanup() {
    inspect(
        "exceptions/typed-catch-shared-cleanup",
        "Lfixture/TypedCatchSharedCleanup;",
        "classify",
        "(Ljava/lang/String;I)I",
        |facts, code, rendered| {
            let scope = facts
                .tries()
                .iter()
                .find(|p| p.finally().is_some())
                .unwrap();
            assert!(scope.clauses().start > 0);
            let finally = scope.finally().unwrap();
            assert_eq!(finally.operations().len(), 1);
            let entry = finally.normal()[0];
            let offset = rendered.text().find("finally {").unwrap();
            let mut retained = 0;
            for bundle in code.copies() {
                if bundle.steps.is_empty() {
                    continue;
                }
                let CopyLocation::Transfer {
                    site: TransferSite::NormalEdge(edge),
                    ..
                } = bundle.location
                else {
                    continue;
                };
                let EdgeRoute::Normal(link) = facts.edges()[edge.0 as usize].route() else {
                    panic!("normal copy edge");
                };
                let link = &facts.links()[link.0 as usize];
                if link.to() == entry && scope.body().contains(&link.from()) {
                    let spans: Vec<_> = rendered
                        .spans()
                        .iter()
                        .filter(
                            |s| matches!(s.origin, Origin::Copy { copy, .. } if copy == bundle.id),
                        )
                        .collect();
                    assert_eq!(spans.len(), 1);
                    assert!(spans[0].start.0 < spans[0].end.0 && spans[0].end.0 <= offset);
                    retained += 1;
                }
            }
            assert_eq!(retained, 2);
        },
    );
}

#[test]
fn branching_cleanup_closes_once_outside_the_protected_loop() {
    inspect(
        "exceptions/branching-finally-suffix",
        "Lfixture/BranchFinally;",
        "run",
        "(III)I",
        |facts, _, rendered| {
            let scope = facts
                .tries()
                .iter()
                .find(|p| p.finally().is_some())
                .unwrap();
            let [loop_plan] = facts.loops() else {
                panic!("one original loop");
            };
            assert!(loop_plan.members().iter().all(|n| scope.body().contains(n)));
            assert_eq!(scope.entry(), loop_plan.header());
            assert_eq!(scope.finally().unwrap().predicates().len(), 1);
            assert!(
                facts
                    .regions()
                    .iter()
                    .any(|r| matches!(r.kind(), RegionKind::Try(id) if *id == scope.id()))
            );
            assert_eq!(rendered.text().matches("finally {").count(), 1);
            assert_eq!(
                rendered
                    .text()
                    .matches("BranchFinally.finish(p0, p1)")
                    .count(),
                1
            );
        },
    );
}

#[test]
fn per_iteration_cleanup_is_not_hoisted_across_its_unguarded_call() {
    inspect(
        "exceptions/typed-catch-loop-exits",
        "Lfixture/LoopCatchFinally;",
        "scan",
        "(II)I",
        |facts, _, _| {
            let [loop_plan] = facts.loops() else {
                panic!("one loop");
            };
            let scope = facts
                .tries()
                .iter()
                .find(|p| p.finally().is_some())
                .unwrap();
            assert!(!scope.body().contains(&loop_plan.header()));
            assert!(
                scope
                    .members()
                    .iter()
                    .all(|n| loop_plan.lexical_members().contains(n))
            );
        },
    );
}

#[test]
fn implicit_rethrow_does_not_erase_an_outside_handler_route() {
    inspect(
        "exceptions/branching-finally-suffix",
        "Lfixture/BranchFinally;",
        "guardedRethrow",
        "(II)I",
        |facts, code, rendered| {
            assert!(facts.tries().iter().all(|p| p.finally().is_none()));
            let rethrows: Vec<_> = code
                .sites()
                .iter()
                .filter(|s| matches!(s.original, diana_decomp::p5::ThrowSite::Terminal(_)))
                .collect();
            assert_eq!(rethrows.len(), 1);
            assert!(matches!(
                rethrows[0].syntax,
                diana_decomp::p7::ExceptionSyntax::Explicit
            ));
            assert!(rendered.text().contains("return 300"));
            assert!(!rendered.text().contains("finally {"));
        },
    );
}

#[test]
fn unequal_cleanup_arguments_do_not_become_one_finally() {
    inspect(
        "exceptions/branching-finally-suffix",
        "Lfixture/BranchFinally;",
        "different",
        "(I)I",
        |facts, _, rendered| {
            assert!(facts.tries().iter().all(|p| p.finally().is_none()));
            assert!(!rendered.text().contains("finally {"));
            assert!(rendered.text().contains("BranchFinally.finish(1, p0)"));
            assert!(rendered.text().contains("BranchFinally.finish(2, p0)"));
        },
    );
}
