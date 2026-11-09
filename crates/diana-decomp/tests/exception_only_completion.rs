//! Completion-interface recovery through ordinary Engine products. The native
//! scenarios beside the prepared fixture are not executed by these checks.
use diana_decomp::p2::{OpId, OpKind};
use diana_decomp::p5::ThrowSite;
use diana_decomp::p6::{CatchTarget, EdgeRoute, ExceptionDelivery, RegionFacts, RegionKind};
use diana_decomp::p7::{
    ExprKind, MemberDeclaration, MethodCode, ReferenceDeclaration, StatementKind,
};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn call(code: &MethodCode, name: &str) -> OpId {
    code.operations()
        .iter()
        .find_map(|operation| {
            let statement = operation.statement?;
            let StatementKind::Evaluate { expression } =
                code.statements()[statement.0 as usize].kind
            else {
                return None;
            };
            let ExprKind::Call(call) = &code.expressions()[expression.0 as usize].kind else {
                return None;
            };
            match &code.references()[call.target.0 as usize].declaration {
                Ok(ReferenceDeclaration::Member(MemberDeclaration::Method {
                    name: original,
                    ..
                })) if original.units == name.encode_utf16().collect::<Vec<_>>() => {
                    Some(operation.source)
                }
                _ => None,
            }
        })
        .expect("original invocation")
}
fn selected(facts: &RegionFacts) -> usize {
    facts
        .regions()
        .iter()
        .filter(|region| matches!(region.kind(), RegionKind::Try(_)))
        .count()
}

#[test]
fn protected_construction_throw_has_no_normal_follow_into_its_catch() {
    inspect(
        "atoms/constructor-argument-chains",
        "Lfixture/ConstructorArguments;",
        "bodyError",
        "(I)V",
        |facts, code, rendered| {
            assert_eq!(selected(facts), 1);
            let [scope] = facts.tries() else {
                panic!("one original protection interval")
            };
            assert_eq!(scope.follow(), None);
            assert!(
                facts
                    .links()
                    .iter()
                    .all(|link| !scope.body().contains(&link.from())
                        || scope.body().contains(&link.to()))
            );
            assert_eq!(rendered.text().matches("throw new ").count(), 1);
            assert!(rendered.text().contains("catch (all"));
            assert!(!rendered.text().contains("[exn c0]"));
            assert_eq!(
                code.operations()
                    .iter()
                    .filter(|op| matches!(op.original, OpKind::MoveException))
                    .count(),
                1
            );
            // The original pre-body return remains before allocation/protection.
            assert!(
                rendered.text().find("return;").unwrap() < rendered.text().find("try {").unwrap()
            );
        },
    );
}

#[test]
fn typed_exception_only_core_preserves_selected_and_uncaught_outlets() {
    inspect(
        "exceptions/exception-only-completion",
        "Lfixture/ExceptionCompletion;",
        "typed",
        "(ILjava/lang/Throwable;)I",
        |facts, _, rendered| {
            assert_eq!(selected(facts), 1);
            let [scope] = facts.tries() else {
                panic!("one typed scope")
            };
            assert_eq!(scope.follow(), None);
            assert_eq!(scope.clauses(), 0..1);
            assert!(
                facts
                    .links()
                    .iter()
                    .all(|link| !scope.body().contains(&link.from())
                        || scope.body().contains(&link.to()))
            );
            let sites: Vec<_> = facts
                .throws()
                .iter()
                .filter(|site| scope.body().contains(&site.node()))
                .collect();
            assert_eq!(sites.len(), 2, "touch and original throw");
            for site in sites {
                let clauses = facts.handler_chains()[site.chain().0 as usize].clauses();
                assert_eq!(clauses.len(), 2);
                assert!(clauses[0].catch.is_some());
                assert_eq!(clauses[0].target, CatchTarget::Handler(scope.handlers()[0]));
                assert_eq!(clauses[1].target, CatchTarget::Uncaught);
                let [handled, uncaught] = site.edges() else {
                    panic!("both original completions")
                };
                assert!(
                    matches!(facts.edges()[handled.0 as usize].route(), EdgeRoute::Exception { order: 0, delivery: ExceptionDelivery::Catch(id), .. } if id == scope.id())
                );
                assert!(matches!(
                    facts.edges()[uncaught.0 as usize].route(),
                    EdgeRoute::Exception {
                        order: 1,
                        delivery: ExceptionDelivery::Explicit,
                        ..
                    }
                ));
            }
            assert_eq!(
                rendered
                    .text()
                    .matches("ExceptionCompletion.touch(")
                    .count(),
                1
            );
            assert_eq!(
                rendered
                    .text()
                    .matches("ExceptionCompletion.consume(")
                    .count(),
                1
            );
            assert!(rendered.text().contains("throw p1"));
            assert!(!rendered.text().contains("graph<"));
        },
    );
}

#[test]
fn typed_handler_failure_escapes_to_the_original_outer_cleanup_once() {
    inspect(
        "exceptions/exception-only-completion",
        "Lfixture/ExceptionCompletion;",
        "nested",
        "(ILjava/lang/Throwable;)I",
        |facts, code, rendered| {
            assert_eq!(selected(facts), 2);
            let inner = facts
                .tries()
                .iter()
                .find(|scope| scope.clauses() == (0..1))
                .unwrap();
            let outer = facts
                .tries()
                .iter()
                .find(|scope| scope.clauses() == (1..2))
                .unwrap();
            assert_eq!(inner.follow(), None);
            assert!(
                inner
                    .members()
                    .iter()
                    .all(|node| outer.body().contains(node))
            );
            let consume = call(code, "consume");
            let consume_site = facts
                .throws()
                .iter()
                .find(|site| site.site() == ThrowSite::Operation(consume))
                .unwrap();
            assert!(!inner.body().contains(&consume_site.node()));
            assert!(outer.body().contains(&consume_site.node()));
            assert!(consume_site.edges().iter().all(|edge| matches!(facts.edges()[edge.0 as usize].route(), EdgeRoute::Exception { delivery: ExceptionDelivery::Catch(id), .. } if id == outer.id())));
            assert!(outer.finally().is_some());
            assert_eq!(
                rendered
                    .text()
                    .matches("ExceptionCompletion.finish(")
                    .count(),
                1
            );
            // The shared inspection helper requires each original call/capture
            // anchor once, including both matched original finish sites.
            assert!(!rendered.text().contains("graph<"));
            assert!(!rendered.text().contains("addSuppressed"));
        },
    );
}

#[test]
fn genuine_normal_entrance_cannot_pull_an_unprotected_effect_into_the_try() {
    inspect(
        "exceptions/exception-only-completion",
        "Lfixture/ExceptionCompletion;",
        "foreign",
        "(ILjava/lang/Throwable;)I",
        |facts, code, rendered| {
            assert!(facts.tries().is_empty());
            let outside = call(code, "outside");
            let site = facts
                .throws()
                .iter()
                .find(|site| site.site() == ThrowSite::Operation(outside))
                .unwrap();
            assert!(
                facts.handler_chains()[site.chain().0 as usize]
                    .clauses()
                    .iter()
                    .all(|clause| clause.target == CatchTarget::Uncaught)
            );
            assert!(rendered.text().contains("[exn c1]"));
            assert_eq!(
                rendered
                    .text()
                    .matches("ExceptionCompletion.outside(")
                    .count(),
                1
            );
            assert_eq!(
                rendered
                    .text()
                    .matches("ExceptionCompletion.touch(")
                    .count(),
                1
            );
            assert_eq!(rendered.text().matches("throw p1").count(), 1);
        },
    );
}
