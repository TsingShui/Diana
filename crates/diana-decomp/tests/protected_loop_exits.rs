//! Independent scenarios: quality/fixtures/exceptions/protected-loop-exits/README.md.
//! These inspect ordinary Engine products, not executed scenario behavior.

use diana_decomp::p6::{RegionKind, TransferAction};
use diana_decomp::p7::{EmissionLayout, IfShape, TerminalKind, TransferSpelling};
use diana_decomp::p9::Origin;

#[path = "support/inspection.rs"]
mod inspection;
#[path = "support/loops.rs"]
mod loops;
#[path = "support/rethrows.rs"]
mod rethrows;
use inspection::inspect;

#[test]
fn nested_private_throw_keeps_both_original_heads_inside_one_finally() {
    inspect(
        "exceptions/protected-loop-exits",
        "Lfixture/ProtectedExits;",
        "nested",
        "(IIII)I",
        |facts, code, rendered| {
            assert_eq!(facts.loops().len(), 2);
            let [scope] = facts.tries() else {
                panic!("one enclosing protection owner")
            };
            let cleanup = scope.finally().expect("complete two-call correspondence");
            let outer = facts
                .loops()
                .iter()
                .max_by_key(|p| p.members().len())
                .unwrap();
            assert_eq!(scope.entry(), outer.header());
            assert_eq!(cleanup.operations().len(), 4, "two literal/call pairs");
            for plan in facts.loops() {
                assert!(
                    plan.lexical_members()
                        .iter()
                        .all(|n| scope.body().contains(n))
                );
                let [exit] = plan.private_exits() else {
                    panic!("one original throw entrance")
                };
                let link = facts.links()[exit.0 as usize];
                assert!(!plan.members().contains(&link.to()));
                assert!(plan.lexical_members().contains(&link.to()));
                assert!(
                    facts
                        .regions()
                        .iter()
                        .any(|r| matches!(r.kind(), RegionKind::If { .. })
                            && r.nodes().contains(&link.to()))
                );
            }
            assert_eq!(loops::updates_once(facts, code, rendered).len(), 2);
            assert_eq!(rendered.text().matches("while (").count(), 2);
            assert_eq!(rendered.text().matches("finally {").count(), 1);
            assert_eq!(
                rendered.text().matches("ProtectedExits.cleanup(").count(),
                2
            );
            assert!(
                rendered.text().find("cleanup(7,").unwrap()
                    < rendered.text().find("cleanup(8,").unwrap()
            );
            assert!(!rendered.text().contains("goto "));
            assert!(!rendered.text().contains("exception_chain"));
            assert!(code.emission().controls().iter().any(|c| matches!(
                c.layout,
                EmissionLayout::If {
                    shape: IfShape::Guard { .. },
                    ..
                }
            )));
            let guard = code
                .terminals()
                .iter()
                .find_map(|terminal| {
                    if !matches!(terminal.kind, TerminalKind::Throw(_)) {
                        return None;
                    }
                    rendered.spans().iter().find_map(|span| {
                        matches!(span.origin, Origin::Terminal { block, .. }
                        if block == terminal.block && span.start.0 < span.end.0)
                        .then_some(span.start.0)
                    })
                })
                .expect("one executable original guard throw");
            assert!(rendered.text().find("ProtectedExits.probe(").unwrap() < guard);
            assert!(guard < rendered.text().find("ProtectedExits.read(").unwrap());
            rethrows::assert_pending_rethrow(facts, rendered);
        },
    );
}

#[test]
fn a_real_body_return_does_not_acquire_the_post_loop_cleanup() {
    inspect(
        "exceptions/protected-loop-exits",
        "Lfixture/ProtectedExits;",
        "early",
        "(III)I",
        |facts, code, rendered| {
            let [plan] = facts.loops() else {
                panic!("one original loop")
            };
            let [scope] = facts.tries() else {
                panic!("one catch owner")
            };
            assert_eq!(scope.entry(), plan.header());
            assert!(scope.finally().is_none());
            assert_eq!(plan.private_exits().len(), 1);
            assert_eq!(loops::updates_once(facts, code, rendered).len(), 1);
            let returns: Vec<_> = code
                .terminals()
                .iter()
                .filter(|t| matches!(t.kind, TerminalKind::Return(Some(_))))
                .collect();
            assert_eq!(returns.len(), 2);
            let mut positions = Vec::new();
            for terminal in returns {
                let spans: Vec<_> = rendered.spans().iter().filter(|s| matches!(s.origin, Origin::Terminal { block, .. } if block == terminal.block)).collect();
                assert_eq!(spans.len(), 1);
                assert!(spans[0].start.0 < spans[0].end.0);
                positions.push(spans[0].start.0);
            }
            positions.sort_unstable();
            assert!(positions[0] < rendered.text().find("ProtectedExits.cleanup(7,").unwrap());
            assert!(positions[1] > rendered.text().rfind("ProtectedExits.cleanup(8,").unwrap());
            assert_eq!(
                rendered.text().matches("ProtectedExits.cleanup(").count(),
                4
            );
            assert!(!rendered.text().contains("finally"));
        },
    );
}

#[test]
fn a_foreign_normal_entrance_keeps_one_shared_construction_and_explicit_protection() {
    inspect(
        "exceptions/protected-loop-exits",
        "Lfixture/ProtectedExits;",
        "foreign",
        "(IIII)I",
        |facts, _, rendered| {
            let [plan] = facts.loops() else {
                panic!("one original loop")
            };
            assert!(facts.tries().is_empty());
            assert!(plan.private_exits().is_empty());
            assert_eq!(plan.members(), plan.lexical_members());
            let exit = plan
                .exits()
                .iter()
                .map(|id| facts.links()[id.0 as usize])
                .find(|l| Some(l.to()) != plan.follow())
                .unwrap();
            assert_eq!(
                facts.links().iter().filter(|l| l.to() == exit.to()).count(),
                2
            );
            assert_eq!(rendered.text().matches("new ").count(), 1);
            assert!(rendered.text().contains("[exn c0]"));
            assert!(rendered.text().contains("goto L"));
            assert_eq!(
                rendered.text().matches("ProtectedExits.cleanup(").count(),
                4
            );
            assert!(!rendered.text().contains("for ("));
        },
    );
}

#[test]
fn a_recovering_handler_is_not_an_abrupt_enclosing_corridor_owner() {
    inspect(
        "exceptions/protected-loop-exits",
        "Lfixture/ProtectedExits;",
        "recover",
        "(III)I",
        |facts, _, rendered| {
            let [plan] = facts.loops() else {
                panic!("one loop")
            };
            assert!(plan.private_exits().is_empty());
            assert!(facts.tries().iter().all(|p| p.finally().is_none()));
            assert_eq!(
                rendered.text().matches("ProtectedExits.cleanup(").count(),
                2
            );
            let cleanup = rendered.text().find("ProtectedExits.cleanup(8,").unwrap();
            let recovery_update = rendered.text().find("+ 2)").unwrap();
            let rejoin = rendered
                .text()
                .rfind(&format!("goto L{};", plan.header().0))
                .unwrap();
            assert!(cleanup < recovery_update && recovery_update < rejoin);
            assert!(rendered.text().contains("+ 1)"));
            assert!(!rendered.text().contains("finally"));
            assert!(!rendered.text().contains("for ("));
        },
    );
}

#[test]
fn raw_selector_edges_keep_explicit_targets_even_next_to_a_sequence_child() {
    inspect(
        "exceptions/protected-loop-exits",
        "Lfixture/ProtectedExits;",
        "switchExit",
        "(IIII)I",
        |facts, code, rendered| {
            let mut selectors = 0;
            for terminal in code.terminals() {
                let TerminalKind::Switch { arms, .. } = &terminal.kind else {
                    continue;
                };
                selectors += 1;
                for id in arms {
                    let transfer = &code.transfers()[id.0 as usize];
                    assert!(
                        matches!(code.emission().transfers()[id.0 as usize].spelling, TransferSpelling::Goto(label) if label == transfer.destination)
                    );
                    assert!(matches!(
                        facts.link_plans()[id.0 as usize].action(),
                        TransferAction::Goto(_) | TransferAction::SwitchArm { .. }
                    ));
                    let spans: Vec<_> = rendered.spans().iter().filter(|s| matches!(s.origin, Origin::Transfer { transfer, .. } if transfer == *id)).collect();
                    assert_eq!(spans.len(), 1);
                    assert!(rendered.text()[spans[0].start.0..spans[0].end.0].contains("goto "));
                }
            }
            assert_eq!(selectors, 1);
            assert!(rendered.text().contains("default: goto L"));
            assert!(!rendered.text().contains("for ("));
        },
    );
}
