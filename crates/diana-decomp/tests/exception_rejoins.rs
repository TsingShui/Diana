//! Ordinary-Engine ownership regressions, not execution of the fixture models.
use diana_decomp::p2::{OpId, OpKind};
use diana_decomp::p4::Interpretation;
use diana_decomp::p5::{Materialization, ThrowSite};
use diana_decomp::p6::{RegionFacts, RegionKind};
use diana_decomp::p7::{
    CopyStatement, ExceptionSyntax, ExprId, ExprKind, HandlerCapture, MemberDeclaration,
    MethodCode, ReferenceDeclaration,
};
use diana_decomp::p9::Origin;

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn calls(code: &MethodCode, name: &str) -> Vec<OpId> {
    code.operations()
        .iter()
        .filter_map(|operation| {
            let statement = operation.statement?;
            let ExprKind::Call(call) = code.statement(statement)?.operator(code) else {
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
        .collect()
}
fn selected_tries(facts: &RegionFacts) -> usize {
    facts
        .regions()
        .iter()
        .filter(|region| matches!(region.kind(), RegionKind::Try(_)))
        .count()
}
fn selected_loops(facts: &RegionFacts) -> usize {
    facts
        .regions()
        .iter()
        .filter(|region| matches!(region.kind(), RegionKind::Loop(_)))
        .count()
}
fn node(code: &MethodCode, operation: OpId) -> diana_decomp::p6::NodeId {
    code.operations()
        .iter()
        .find(|op| op.source == operation)
        .unwrap()
        .node
}
fn snapshots(code: &MethodCode, operation: OpId) -> Vec<ExprId> {
    code.emission().operations()[operation.0 as usize]
        .snapshots
        .iter()
        .flat_map(|id| &code.copies()[id.0 as usize].steps)
        .filter_map(|step| match step {
            CopyStatement::Assign { expression, .. } => Some(*expression),
            _ => None,
        })
        .collect()
}
fn committed_handle(code: &MethodCode, operation: OpId) -> diana_decomp::p5::LocalId {
    let statement = code
        .operations()
        .iter()
        .find(|op| op.source == operation)
        .unwrap()
        .statement
        .unwrap();
    let Materialization::Store(local) =
        code.statement(statement).unwrap().outputs[0].materialization()
    else {
        panic!("the original normal acquisition commit")
    };
    local
}

#[test]
fn the_original_retry_owns_the_shared_captureless_update_once() {
    inspect(
        "exceptions/handler-retry-rejoin",
        "Lfixture/RetryRejoin;",
        "run",
        "(III)I",
        |facts, code, rendered| {
            assert_eq!(selected_loops(facts), 1);
            let plan = &facts.loops()[0];
            let shared = facts
                .handler_entries()
                .iter()
                .find(|entry| facts.links().iter().any(|link| link.to() == entry.node()))
                .unwrap();
            assert!(shared.capture().is_none());
            assert!(plan.lexical_members().contains(&shared.node()));
            assert!(!rendered.text().contains("graph<"));
            assert_eq!(calls(code, "attempt").len(), 1);
            assert_eq!(calls(code, "reset").len(), 1);
            let reset = calls(code, "reset")[0];
            assert!(
                code.sites()
                    .iter()
                    .any(|site| site.original == ThrowSite::Operation(reset)
                        && matches!(site.syntax, ExceptionSyntax::Propagate))
            );
        },
    );
}

#[test]
fn nested_captureless_rejoins_keep_exception_only_receives_and_unguarded_advance() {
    inspect(
        "exceptions/captureless-retry-nesting",
        "Lfixture/RetryNesting;",
        "nested",
        "(III)I",
        |facts, code, rendered| {
            assert_eq!(selected_loops(facts), 2);
            let inner = facts
                .loops()
                .iter()
                .min_by_key(|plan| plan.members().len())
                .unwrap();
            let outer = facts
                .loops()
                .iter()
                .max_by_key(|plan| plan.members().len())
                .unwrap();
            let [handler] = code.handlers() else {
                panic!("one original shared entry")
            };
            assert!(matches!(handler.capture, HandlerCapture::None));
            assert!(!handler.receives.is_empty());
            assert!(inner.lexical_members().contains(&handler.node));
            assert!(
                inner
                    .lexical_members()
                    .iter()
                    .all(|node| outer.lexical_members().contains(node))
            );
            let advance = calls(code, "advance")[0];
            assert_eq!(node(code, advance), handler.node);
            let site = code
                .sites()
                .iter()
                .find(|site| site.original == ThrowSite::Operation(advance))
                .unwrap();
            assert!(matches!(site.syntax, ExceptionSyntax::Propagate));
            assert!(
                !code
                    .operations()
                    .iter()
                    .any(|operation| matches!(operation.original, OpKind::MoveException))
            );
            assert_eq!(rendered.text().matches("on_exception_entry").count(), 1);
            assert!(!rendered.text().contains("graph<"));
        },
    );
}

#[test]
fn a_captureless_retreating_latch_keeps_its_increment_before_the_attempt() {
    inspect(
        "exceptions/captureless-retry-nesting",
        "Lfixture/RetryNesting;",
        "terminal",
        "(II)I",
        |facts, code, rendered| {
            assert_eq!(selected_loops(facts), 1);
            let plan = &facts.loops()[0];
            let [handler] = facts.handler_entries() else {
                panic!("one original latch handler")
            };
            assert!(handler.capture().is_none());
            assert!(
                plan.latches()
                    .iter()
                    .any(|id| facts.links()[id.0 as usize].from() == handler.node())
            );
            // Independent Smali coordinates: add-int/lit8 at6, invoke at8.
            let start = |pc| {
                rendered
                    .spans()
                    .iter()
                    .find_map(|span| match span.origin {
                        Origin::Operation { pc: original, .. } if original.0 == pc => {
                            Some(span.start.0)
                        }
                        _ => None,
                    })
                    .unwrap()
            };
            assert!(start(6) < start(8));
            assert_eq!(calls(code, "attempt").len(), 1);
            assert!(!rendered.text().contains("graph<"));
        },
    );
}

#[test]
fn an_exception_before_the_header_is_not_an_interior_loop_entry() {
    inspect(
        "exceptions/captureless-retry-nesting",
        "Lfixture/RetryNesting;",
        "foreign",
        "(II)I",
        |facts, code, rendered| {
            assert!(facts.loops().is_empty());
            assert_eq!(calls(code, "attempt").len(), 2);
            assert!(
                code.handlers()
                    .iter()
                    .all(|handler| matches!(handler.capture, HandlerCapture::None))
            );
            assert!(rendered.text().contains("graph<"));
            assert_eq!(rendered.text().matches("on_exception_entry").count(), 1);
        },
    );
}

fn close_outlets(facts: &RegionFacts, code: &MethodCode, outside: usize, canonical: usize) {
    let outer = facts
        .tries()
        .iter()
        .max_by_key(|plan| plan.members().len())
        .unwrap();
    let closes = calls(code, "close");
    assert_eq!(
        closes
            .iter()
            .filter(|&&op| !outer.members().contains(&node(code, op)))
            .count(),
        outside
    );
    assert_eq!(
        closes
            .iter()
            .filter(|&&op| outer.members().contains(&node(code, op)))
            .count(),
        canonical
    );
    assert!(
        closes
            .iter()
            .all(|&op| !outer.body().contains(&node(code, op)))
    );
    for op in closes {
        assert!(
            code.sites()
                .iter()
                .any(|site| site.original == ThrowSite::Operation(op)
                    && matches!(site.syntax, ExceptionSyntax::Propagate))
        );
    }
}

#[test]
fn response_catches_are_natural_without_acquiring_unguarded_close_suffixes() {
    inspect(
        "exceptions/response-close-snapshots",
        "Lfixture/ResponseClose;",
        "response",
        "(I)I",
        |facts, code, rendered| {
            assert_eq!(selected_tries(facts), 2);
            assert!(!rendered.text().contains("graph<"));
            close_outlets(facts, code, 2, 1);
            let outer = facts
                .tries()
                .iter()
                .max_by_key(|plan| plan.members().len())
                .unwrap();
            let inner = facts
                .tries()
                .iter()
                .min_by_key(|plan| plan.members().len())
                .unwrap();
            assert!(
                inner
                    .members()
                    .iter()
                    .all(|node| outer.body().contains(node))
            );
            assert_eq!(inner.clauses(), 0..2);
            assert_eq!(outer.clauses(), 2..3);
            assert!(!rendered.text().contains("addSuppressed"));
        },
    );
}

#[test]
fn nested_dual_acquisition_keeps_partial_identity_commits_and_close_order() {
    inspect(
        "exceptions/cleanup-outlet-frontiers",
        "Lfixture/CleanupFrontiers;",
        "dual",
        "(I)I",
        |facts, code, rendered| {
            assert_eq!(selected_tries(facts), 3);
            assert!(!rendered.text().contains("graph<"));
            close_outlets(facts, code, 4, 2);
            let acquire = calls(code, "acquire");
            assert_eq!(acquire.len(), 2);
            let first = committed_handle(code, acquire[0]);
            let second = committed_handle(code, acquire[1]);
            assert_ne!(first, second);
            let before_first = snapshots(code, acquire[0]);
            assert!(!before_first.is_empty());
            assert!(before_first.iter().all(|id| matches!(
                code.expressions()[id.0 as usize].kind,
                ExprKind::Literal {
                    view: Interpretation::Null,
                    ..
                }
            )));
            let before_second = snapshots(code, acquire[1]);
            assert!(before_second.iter().any(|id| matches!(code.expressions()[id.0 as usize].kind, ExprKind::Read { storage, .. } if storage.0 == first.0)));
            assert!(before_second.iter().any(|id| matches!(
                code.expressions()[id.0 as usize].kind,
                ExprKind::Literal {
                    view: Interpretation::Null,
                    ..
                }
            )));
            assert!(!before_second.iter().any(|id| matches!(code.expressions()[id.0 as usize].kind, ExprKind::Read { storage, .. } if storage.0 == second.0)));
            let ready = calls(code, "ready")[0];
            let after_second = snapshots(code, ready);
            for handle in [first, second] {
                assert!(after_second.iter().any(|id| matches!(code.expressions()[id.0 as usize].kind, ExprKind::Read { storage, .. } if storage.0 == handle.0)));
            }
            // In the independent Smali each route places B.close before A.close.
            // Rendering must retain each original operation anchor in that order.
            let closes = calls(code, "close");
            assert_eq!(closes.len(), 6);
            let at = |operation| {
                rendered
                    .spans()
                    .iter()
                    .find_map(|span| match span.origin {
                        Origin::Operation {
                            operation: original,
                            ..
                        } if original == operation => Some(span.start.0),
                        _ => None,
                    })
                    .unwrap()
            };
            for pair in closes.as_chunks::<2>().0 {
                assert!(at(pair[0]) < at(pair[1]));
            }
            assert!(!rendered.text().contains("addSuppressed"));
        },
    );
}

#[test]
fn split_captures_keep_their_shared_cleanup_guard_outside_both_contexts() {
    inspect(
        "exceptions/shared-cleanup-contexts",
        "Lfixture/CleanupContexts;",
        "split",
        "(I)I",
        |facts, code, rendered| {
            assert_eq!(selected_tries(facts), 2);
            assert!(!rendered.text().contains("graph<"));
            let outer = facts
                .tries()
                .iter()
                .max_by_key(|plan| plan.members().len())
                .unwrap();
            let roots: std::collections::BTreeSet<_> = facts
                .handler_chains()
                .iter()
                .flat_map(|chain| chain.clauses())
                .filter_map(|clause| match clause.target {
                    diana_decomp::p6::CatchTarget::Handler(root) if clause.catch.is_none() => {
                        Some(root)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                roots.len(),
                2,
                "two original all-capture entries, not an alias"
            );
            assert!(roots.iter().all(|root| outer.members().contains(root)));
            assert_eq!(
                roots
                    .iter()
                    .filter(|root| outer.body().contains(root))
                    .count(),
                1
            );
            for root in roots {
                let entry = facts
                    .handler_entries()
                    .iter()
                    .find(|entry| entry.node() == root)
                    .unwrap();
                assert!(entry.capture().is_some());
                assert!(!entry.receives().is_empty());
            }
            // Independent Smali coordinate: the shared pure close guard is47.
            let guard = code
                .terminals()
                .iter()
                .find(|terminal| terminal.pc.is_some_and(|pc| pc.0 == 47))
                .unwrap();
            let labels: Vec<_> = code
                .labels()
                .iter()
                .filter(|label| label.block == guard.block)
                .collect();
            assert!(!labels.is_empty());
            assert!(
                labels
                    .iter()
                    .all(|label| !outer.members().contains(&label.node))
            );
            assert_eq!(rendered.text().matches("capture_exception").count(), 6);
            assert_eq!(calls(code, "close").len(), 3);
            for close in calls(code, "close") {
                assert!(
                    code.sites()
                        .iter()
                        .any(|site| site.original == ThrowSite::Operation(close)
                            && matches!(site.syntax, ExceptionSyntax::Propagate))
                );
            }
            assert!(!rendered.text().contains("addSuppressed"));
        },
    );
}

#[test]
fn resource_outlets_do_not_acquire_the_outer_io_recovery_or_replace_the_primary() {
    inspect(
        "exceptions/shared-cleanup-contexts",
        "Lfixture/CleanupContexts;",
        "resource",
        "(I)I",
        |facts, code, rendered| {
            assert_eq!(selected_tries(facts), 3);
            let read = calls(code, "read")[0];
            let note = calls(code, "note")[0];
            let resource = facts
                .tries()
                .iter()
                .filter(|plan| plan.body().contains(&node(code, read)))
                .min_by_key(|plan| plan.members().len())
                .unwrap();
            assert!(resource.body().contains(&node(code, note)));
            let log = calls(code, "log")[0];
            assert!(
                !resource.members().contains(&node(code, log)),
                "outer recovery is not private resource content"
            );
            let closes = calls(code, "close");
            assert_eq!(closes.len(), 3);
            assert_eq!(
                closes
                    .iter()
                    .filter(|&&op| !resource.members().contains(&node(code, op)))
                    .count(),
                2
            );
            assert!(
                closes
                    .iter()
                    .all(|&op| !resource.body().contains(&node(code, op)))
            );
            for op in [read, note] {
                assert!(
                    code.sites()
                        .iter()
                        .any(|site| site.original == ThrowSite::Operation(op)
                            && matches!(site.syntax, ExceptionSyntax::Natural))
                );
            }
            let suppress = calls(code, "addSuppressed")[0];
            let statement = code
                .operations()
                .iter()
                .find(|op| op.source == suppress)
                .unwrap()
                .statement
                .unwrap();
            let ExprKind::Call(call) = code.statement(statement).unwrap().operator(code) else {
                panic!("suppression call")
            };
            let diana_decomp::p7::Arguments::Known(arguments) = &call.arguments else {
                panic!("original secondary operand")
            };
            let [secondary] = arguments.as_slice() else {
                panic!("one secondary operand")
            };
            let ExprKind::Read {
                storage: primary, ..
            } = code.expressions()[call.receiver.unwrap().0 as usize].kind
            else {
                panic!("primary read")
            };
            let ExprKind::Read {
                storage: secondary, ..
            } = code.expressions()[secondary.0 as usize].kind
            else {
                panic!("secondary read")
            };
            assert_ne!(primary, secondary);
            assert_eq!(rendered.text().matches(".addSuppressed(").count(), 1);
            assert!(!rendered.text().contains("finally"));
        },
    );
}

#[test]
fn an_unguarded_gap_cannot_be_absorbed_or_retried_from_a_later_seed() {
    inspect(
        "exceptions/cleanup-outlet-frontiers",
        "Lfixture/CleanupFrontiers;",
        "gap",
        "(I)I",
        |facts, code, rendered| {
            assert!(facts.tries().is_empty());
            let probe = calls(code, "probe")[0];
            assert!(
                code.sites()
                    .iter()
                    .any(|site| site.original == ThrowSite::Operation(probe)
                        && matches!(site.syntax, ExceptionSyntax::Propagate))
            );
            assert_eq!(calls(code, "acquire").len(), 1);
            assert_eq!(calls(code, "read").len(), 1);
            assert_eq!(rendered.text().matches("capture_exception").count(), 2);
            assert!(rendered.text().contains("[exn c0]"));
        },
    );
}
