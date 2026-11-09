//! Ordinary-Engine finally exit plans preserve actual and origin-only terminals.

use diana_decomp::p2::{BinaryOp, OpKind};
use diana_decomp::p5::{Materialization, StorageRead};
use diana_decomp::p6::{FinallyPlan, RegionFacts, RegionKind};
use diana_decomp::p7::TerminalKind;
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn selected<'a>(facts: &'a RegionFacts, rendered: &Rendered) -> &'a FinallyPlan {
    let scope = facts
        .tries()
        .iter()
        .find(|p| p.finally().is_some())
        .expect("matched cleanup product");
    assert!(
        facts
            .regions()
            .iter()
            .any(|r| matches!(r.kind(), RegionKind::Try(id) if *id == scope.id()))
    );
    assert_eq!(rendered.text().matches("finally {").count(), 1);
    scope.finally().unwrap()
}

fn terminal_origins(facts: &RegionFacts, plan: &FinallyPlan, rendered: &Rendered) {
    for &(normal, canonical) in plan.terminals() {
        for (node, executable) in [(normal, false), (canonical, true)] {
            let block = facts.nodes()[node.0 as usize].block();
            let spans: Vec<_> = rendered
                .spans()
                .iter()
                .filter(
                    |span| matches!(span.origin, Origin::Terminal { block: id, .. } if id == block),
                )
                .collect();
            assert_eq!(spans.len(), 1, "one original terminal anchor");
            assert_eq!(
                spans[0].start.0 < spans[0].end.0,
                executable,
                "only the cleanup spelling executes"
            );
        }
    }
    for &rethrow in plan.rethrows() {
        let block = facts.nodes()[rethrow.0 as usize].block();
        let spans: Vec<_> = rendered
            .spans()
            .iter()
            .filter(|s| matches!(s.origin, Origin::Terminal { block: id, .. } if id == block))
            .collect();
        assert_eq!(spans.len(), 1);
        assert_eq!(
            spans[0].start, spans[0].end,
            "pending rethrow is implicit, not erased"
        );
    }
}

#[test]
fn the_original_return_and_constructed_throw_replace_the_pending_exit_once() {
    inspect(
        "exceptions/finally-overrides-exit",
        "Lfixture/FinallyOverridesExit;",
        "replaceExit",
        "(I)I",
        |facts, _, rendered| {
            let plan = selected(facts, rendered);
            assert_eq!(plan.rethrows().len(), 1);
            assert_eq!(plan.terminals().len(), 2);
            terminal_origins(facts, plan, rendered);
            assert_eq!(rendered.text().matches("return -7;").count(), 1);
            assert_eq!(
                rendered
                    .text()
                    .matches("IllegalArgumentException(\"finally\")")
                    .count(),
                1
            );
            assert_eq!(
                rendered
                    .text()
                    .matches("FinallyOverridesExit.close(")
                    .count(),
                1
            );
        },
    );
}

#[test]
fn body_return_preparation_is_not_confused_with_the_first_cleanup_add() {
    inspect(
        "exceptions/abrupt-finally-completion",
        "Lfixture/CleanupExits;",
        "capture",
        "(III)I",
        |facts, code, rendered| {
            let plan = selected(facts, rendered);
            terminal_origins(facts, plan, rendered);
            let scope = facts
                .tries()
                .iter()
                .find(|p| p.finally().is_some())
                .unwrap();
            let body_add = code
                .operations()
                .iter()
                .find(|op| {
                    matches!(
                        op.original,
                        OpKind::Binary {
                            op: BinaryOp::AddInt,
                            ..
                        }
                    ) && facts.nodes().iter().any(|node| {
                        scope.body().contains(&node.id())
                            && node.operations().contains(&op.source.0)
                    })
                })
                .expect("original body-return add belongs before cleanup");
            let definition = &code.statements()[body_add.statement.unwrap().0 as usize].outputs[0];
            let Materialization::Store(local) = definition.materialization() else {
                panic!("saved return materialization");
            };
            let saved_return = code.terminals().iter().any(|terminal| {
                let TerminalKind::Return(Some(expression)) = terminal.kind else {
                    return false;
                };
                code.expressions()[expression.0 as usize]
                    .origin
                    .as_ref()
                    .is_some_and(|origin| {
                        matches!(origin.read,
                            Ok(StorageRead::Value { local: at, state, .. })
                            if at == local && state == definition.state())
                    })
            });
            assert!(
                saved_return,
                "normal return reads the captured body state, not the changed base"
            );
            let text = rendered.text();
            assert!(text.find(" + p1)").unwrap() < text.find("finally {").unwrap());
            assert!(
                text.find("CleanupExits.close(").unwrap()
                    < text.find("CleanupExits.mark(").unwrap()
            );
        },
    );
}

#[test]
fn always_abrupt_cleanup_needs_no_fabricated_follow_or_pending_rethrow() {
    for (name, prototype) in [
        ("always", "(II)I"),
        ("voidAlways", "(I)V"),
        ("bareVoid", "(I)V"),
        ("parameterThrow", "(Ljava/lang/Throwable;I)V"),
    ] {
        inspect(
            "exceptions/abrupt-finally-completion",
            "Lfixture/CleanupExits;",
            name,
            prototype,
            |facts, _, rendered| {
                let plan = selected(facts, rendered);
                assert!(
                    plan.rethrows().is_empty(),
                    "all cleanup paths replace the pending exit"
                );
                assert!(!plan.terminals().is_empty());
                assert!(
                    facts
                        .tries()
                        .iter()
                        .find(|p| p.finally().is_some())
                        .unwrap()
                        .follow()
                        .is_none()
                );
                terminal_origins(facts, plan, rendered);
                if name == "bareVoid" || name == "voidAlways" {
                    assert_eq!(rendered.text().matches("return;").count(), 1);
                }
                if name == "always" {
                    assert_eq!(rendered.text().matches("return 8;").count(), 1);
                }
                if name == "parameterThrow" {
                    assert_eq!(rendered.text().matches("throw p0;").count(), 1);
                }
            },
        );
    }
}

#[test]
fn both_original_rethrows_share_only_the_established_normal_continuation() {
    inspect(
        "exceptions/abrupt-finally-completion",
        "Lfixture/CleanupExits;",
        "twoRethrows",
        "(II)I",
        |facts, _, rendered| {
            let plan = selected(facts, rendered);
            assert_eq!(plan.rethrows().len(), 2);
            assert!(plan.terminals().is_empty());
            terminal_origins(facts, plan, rendered);
            assert_eq!(rendered.text().matches("CleanupExits.close(").count(), 2);
            assert_eq!(rendered.text().matches("return 8;").count(), 1);
        },
    );
}

#[test]
fn unequal_cleanup_returns_and_throw_arguments_keep_both_original_paths() {
    for (name, prototype) in [("differentReturn", "(I)I"), ("differentThrow", "(I)V")] {
        inspect(
            "exceptions/abrupt-finally-completion",
            "Lfixture/CleanupExits;",
            name,
            prototype,
            |facts, _, rendered| {
                assert!(facts.tries().iter().all(|p| p.finally().is_none()));
                assert!(!rendered.text().contains("finally {"));
                if name == "differentReturn" {
                    assert!(rendered.text().contains("return -7;"));
                    assert!(rendered.text().contains("return -8;"));
                } else {
                    assert!(rendered.text().contains("\"normal\""));
                    assert!(rendered.text().contains("\"handler\""));
                }
            },
        );
    }
}

#[test]
fn a_foreign_normal_cleanup_entry_keeps_its_original_body_bypass() {
    inspect(
        "exceptions/abrupt-finally-completion",
        "Lfixture/CleanupExits;",
        "foreign",
        "(II)I",
        |facts, _, rendered| {
            assert!(facts.tries().iter().all(|p| p.finally().is_none()));
            assert!(!rendered.text().contains("finally {"));
            assert_eq!(rendered.text().matches("CleanupExits.read(").count(), 1);
            assert_eq!(rendered.text().matches("CleanupExits.close(").count(), 2);
        },
    );
}
