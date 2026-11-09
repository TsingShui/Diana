//! Short-circuit recovery preserves original routing and refuses effect barriers.

use diana_decomp::p6::RegionFacts;
use diana_decomp::p7::{ControlKind, EmissionLayout, MethodCode, TerminalKind, TransferId};
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn original_edges(code: &MethodCode, rendered: &Rendered) {
    for transfer in code.transfers().iter().filter(|t| t.edge.is_some()) {
        assert_eq!(
            rendered
                .spans()
                .iter()
                .filter(|span| matches!(span.origin, Origin::Transfer { transfer: id, .. } if id == transfer.id))
                .count(),
            1,
            "one anchor for original edge {:?}",
            transfer.edge
        );
    }
}

fn compact_routes(facts: &RegionFacts, code: &MethodCode, rendered: &Rendered, tests: usize) {
    let plans: Vec<_> = facts
        .conditions()
        .iter()
        .filter(|plan| !plan.routing().is_empty())
        .collect();
    assert_eq!(plans.len(), 1, "one shared result owns the selected routes");
    assert_eq!(plans[0].tests().len(), tests);
    assert!(code.emission().controls().iter().any(|control| {
        matches!(&control.layout, EmissionLayout::Condition { tests: predicates, routing: Some(_), .. } if predicates.len() == tests)
    }));
    assert!(!rendered.text().contains("goto "));
    for &link in plans[0].routing() {
        let spans: Vec<_> = rendered.spans().iter().filter(|span| {
            matches!(span.origin, Origin::Transfer { transfer, .. } if transfer == TransferId(link.0))
        }).collect();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].start, spans[0].end);
        let node = facts.links()[link.0 as usize].from();
        let block = facts.nodes()[node.0 as usize].block();
        let terminals: Vec<_> = rendered
            .spans()
            .iter()
            .filter(|span| matches!(span.origin, Origin::Terminal { block: id, .. } if id == block))
            .collect();
        assert_eq!(terminals.len(), 1);
        assert_eq!(terminals[0].start, terminals[0].end);
    }
    original_edges(code, rendered);
}

#[test]
fn boolean_or_keeps_both_predicates_and_the_original_empty_goto() {
    inspect(
        "atoms/expr-compare",
        "Lfixture/ExprCompare;",
        "either",
        "(ZZ)Z",
        |facts, code, rendered| {
            compact_routes(facts, code, rendered, 2);
            assert!(rendered.text().contains("p0 != 0 || p1 != 0"));
            assert!(!rendered.text().lines().any(|line| {
                let line = line.trim();
                line.starts_with('L') && line.ends_with(':')
            }));
        },
    );
}

#[test]
fn three_effectful_tests_keep_order_and_exclusive_result_arms() {
    inspect(
        "branches/short-circuit-shared-routes",
        "Lfixture/OrRoutes;",
        "ordered",
        "(II)I",
        |facts, code, rendered| {
            compact_routes(facts, code, rendered, 3);
            let text = rendered.text();
            let stages: Vec<_> = [0, 1, 2]
                .into_iter()
                .map(|stage| {
                    text.find(&format!("OrRoutes.probe({stage}, p0, p1)"))
                        .expect("original stage argument at one predicate")
                })
                .collect();
            assert!(stages.windows(2).all(|pair| pair[0] < pair[1]));
            assert_eq!(text.matches(" || ").count(), 2);
            assert_eq!(text.matches("OrRoutes.mark(40)").count(), 1);
            assert_eq!(text.matches("OrRoutes.mark(50)").count(), 1);
            assert_eq!(text.matches("OrRoutes.mark(60)").count(), 1);
            assert!(text.find("OrRoutes.mark(40)").unwrap() < text.find("else").unwrap());
            assert!(
                text.find("OrRoutes.mark(50)").unwrap() < text.find("OrRoutes.mark(60)").unwrap()
            );
        },
    );
}

#[test]
fn an_effect_on_the_selected_path_stays_visible_and_exclusive() {
    inspect(
        "branches/short-circuit-shared-routes",
        "Lfixture/OrRoutes;",
        "effectful",
        "(I)I",
        |_, code, rendered| {
            assert!(!rendered.text().contains(" || "));
            assert_eq!(rendered.text().matches("OrRoutes.mark(30)").count(), 1);
            original_edges(code, rendered);
        },
    );
}

#[test]
fn an_uncomposable_later_prelude_uses_the_complete_explicit_route() {
    inspect(
        "branches/short-circuit-shared-routes",
        "Lfixture/OrRoutes;",
        "blocked",
        "(I)I",
        |_, code, rendered| {
            let control = code.controls().iter().find(|control| {
                matches!(&control.kind, ControlKind::Condition { routing, .. } if !routing.is_empty())
            }).expect("P6 admits the empty selected route");
            let emitted = &code.emission().controls()[control.id.0 as usize];
            assert!(matches!(emitted.layout, EmissionLayout::Sequence));
            assert!(emitted.explicit_children.is_some());
            assert!(!rendered.text().contains(" || "));
            assert_eq!(rendered.text().matches("OrRoutes.mark(25)").count(), 1);
            original_edges(code, rendered);
        },
    );
}

#[test]
fn floating_nan_comparisons_keep_their_ordered_explicit_semantics() {
    inspect(
        "atoms/expr-wide",
        "Lfixture/ExprWide;",
        "nanComparison",
        "(D)Z",
        |_, code, rendered| {
            let control = code.controls().iter().find(|control| {
                matches!(&control.kind, ControlKind::Condition { tests, routing, .. } if tests.len() == 3 && !routing.is_empty())
            }).expect("three original floating predicates own the selected route");
            assert!(matches!(
                code.emission().controls()[control.id.0 as usize].layout,
                EmissionLayout::Sequence
            ));
            assert_eq!(rendered.text().matches("compare<CmplDouble>").count(), 2);
            assert_eq!(rendered.text().matches("compare<CmpgDouble>").count(), 1);
            assert!(!rendered.text().contains(" || "));
            original_edges(code, rendered);
        },
    );
}

#[test]
fn an_actual_phi_assignment_on_the_empty_goto_is_not_skipped() {
    inspect(
        "branches/short-circuit-shared-routes",
        "Lfixture/OrRoutes;",
        "copied",
        "(III)I",
        |facts, code, rendered| {
            assert!(
                code.terminals().iter().any(|terminal| {
                    let TerminalKind::Goto(link) = terminal.kind else {
                        return false;
                    };
                    code.transfers()[link.0 as usize]
                        .copies
                        .iter()
                        .any(|copy| !code.copies()[copy.0 as usize].steps.is_empty())
                }),
                "the guard must contain a real executable edge copy"
            );
            assert!(
                facts
                    .conditions()
                    .iter()
                    .all(|plan| plan.routing().is_empty())
            );
            assert!(!rendered.text().contains(" || "));
            original_edges(code, rendered);
        },
    );
}

#[test]
fn a_foreign_route_entry_cannot_select_a_probe_that_was_not_run() {
    inspect(
        "branches/short-circuit-shared-routes",
        "Lfixture/OrRoutes;",
        "foreign",
        "(II)I",
        |facts, code, rendered| {
            assert!(
                facts
                    .conditions()
                    .iter()
                    .all(|plan| plan.routing().is_empty())
            );
            original_edges(code, rendered);
        },
    );
}

#[test]
fn an_empty_goto_cycle_keeps_its_original_backedge() {
    inspect(
        "branches/short-circuit-shared-routes",
        "Lfixture/OrRoutes;",
        "cyclic",
        "(I)I",
        |facts, code, rendered| {
            assert!(
                facts
                    .conditions()
                    .iter()
                    .all(|plan| plan.routing().is_empty())
            );
            assert!(!rendered.text().contains(" || "));
            assert!(!facts.loops().is_empty());
            original_edges(code, rendered);
        },
    );
}

#[test]
fn a_failed_conjunction_that_is_the_common_follow_keeps_one_condition() {
    inspect(
        "branches/branch-if-inverted",
        "Lfixture/Inverted;",
        "nested",
        "(II)I",
        |facts, code, rendered| {
            assert_eq!(facts.conditions().len(), 1);
            assert_eq!(facts.conditions()[0].tests().len(), 2);
            assert!(code.emission().controls().iter().any(|control| {
                matches!(&control.layout, EmissionLayout::Condition { tests, .. } if tests.len() == 2)
            }));
            assert!(rendered.text().contains("p0 == 0 || p1 == 0"));
            assert!(!rendered.text().contains("goto "));
            original_edges(code, rendered);
        },
    );
}

#[test]
fn equals_shaped_tests_reach_the_terminal_follow_as_one_condition() {
    inspect(
        "branches/terminal-follow-conjunction",
        "Lfixture/TerminalFollow;",
        "matches",
        "(Ljava/lang/Object;)Z",
        |facts, code, rendered| {
            assert_eq!(facts.conditions().len(), 1);
            assert_eq!(facts.conditions()[0].tests().len(), 4);
            assert!(code.emission().controls().iter().any(|control| {
                matches!(&control.layout, EmissionLayout::Condition { tests, .. } if tests.len() == 4)
            }));
            let text = rendered.text();
            assert_eq!(text.matches(" || ").count(), 3);
            assert!(!text.contains("goto "));
            let left = text.find("p0.left").expect("left field test");
            let middle = text.find("p0.middle").expect("middle field test");
            let right = text.find("p0.right").expect("right field test");
            assert!(left < middle && middle < right);
            assert_eq!(text.matches("TerminalFollow.eq(").count(), 3);
            assert!(text.contains("return false"));
            assert!(text.contains("return true"));
            original_edges(code, rendered);
        },
    );
}

#[test]
fn effectful_conjunction_keeps_its_tests_and_result_effects_in_order() {
    inspect(
        "branches/terminal-follow-conjunction",
        "Lfixture/TerminalFollow;",
        "effectful",
        "(Ljava/lang/Object;)Z",
        |facts, _, rendered| {
            assert_eq!(facts.conditions().len(), 1);
            assert_eq!(facts.conditions()[0].tests().len(), 4);
            let text = rendered.text();
            let first = text.find("TerminalFollow.probe(1)").expect("first probe");
            let equality = text
                .find("TerminalFollow.eq(p0.left, this.left)")
                .expect("field equality");
            let second = text.find("TerminalFollow.probe(2)").expect("second probe");
            let failure = text
                .find("TerminalFollow.mark(20)")
                .expect("failure effect");
            let success = text
                .find("TerminalFollow.mark(10)")
                .expect("success effect");
            assert!(first < equality && equality < second);
            assert!(failure < success);
            assert!(!text.contains("goto "));
        },
    );
}
