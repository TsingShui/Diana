//! Guard spelling retains the complete other arm, copies and lexical scope.

use diana_decomp::p2::BlockId;
use diana_decomp::p7::{
    BlockEnd, ControlId, CopyStatement, EmissionLayout, IfShape, MethodCode, PredicateEmission,
    TerminalKind,
};
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn guards(code: &MethodCode) -> Vec<(ControlId, PredicateEmission, ControlId, ControlId, bool)> {
    code.controls()
        .iter()
        .enumerate()
        .filter_map(|(index, control)| {
            if let EmissionLayout::If {
                yes,
                no,
                test,
                shape: IfShape::Guard { inverted },
                ..
            } = control.layout
            {
                let (exit, tail) = if inverted { (no, yes) } else { (yes, no) };
                Some((ControlId(index as u32), test, exit, tail, inverted))
            } else {
                None
            }
        })
        .collect()
}

fn terminal_of(code: &MethodCode, mut id: ControlId) -> BlockId {
    loop {
        let emission = &code.controls()[id.0 as usize];
        match emission.layout {
            EmissionLayout::Sequence => id = *emission.children().last().unwrap(),
            EmissionLayout::Block(node) => {
                let BlockEnd::Terminal(terminal) = code.emission().blocks()[node.0 as usize].end
                else {
                    panic!("guard must end at its actual terminal")
                };
                return terminal;
            }
            _ => panic!("opaque scopes cannot provide a guard exit"),
        }
    }
}

fn span_of_terminal(rendered: &Rendered, terminal: BlockId) -> (usize, usize) {
    let spans: Vec<_> = rendered
        .spans()
        .iter()
        .filter(|s| matches!(s.origin, Origin::Terminal { block, .. } if block == terminal))
        .collect();
    assert_eq!(spans.len(), 1);
    (spans[0].start.0, spans[0].end.0)
}

#[test]
fn recovered_scan_values_keep_the_original_update_after_the_guard() {
    inspect(
        "exceptions/typed-catch-loop-exits",
        "Lfixture/LoopCatchFinally;",
        "scan",
        "(II)I",
        |facts, code, rendered| {
            assert_eq!(facts.loops().len(), 1);
            let gs = guards(code);
            assert_eq!(gs.len(), 1);
            assert!(matches!(
                code.terminals()[terminal_of(code, gs[0].2).0 as usize].kind,
                TerminalKind::Return(Some(_))
            ));
            assert!(!rendered.text().contains("else {"));
            assert!(rendered.text().contains("while ("));
            assert!(rendered.text().contains("cleanup("));
        },
    );
}

#[test]
fn the_inverted_guard_keeps_actual_phi_assignments_after_the_exit_once() {
    inspect(
        "branches/terminal-guard-arms",
        "Lfixture/GuardArms;",
        "select",
        "(III)I",
        |_, code, rendered| {
            let gs = guards(code);
            assert_eq!(gs.len(), 1);
            let (_, test, exit, _, inverted) = gs[0];
            assert!(
                inverted,
                "the original taken edge enters the Phi, not the return"
            );
            assert!(test.negate);
            let edge = test.when_true;
            let copies = &code.emission().transfers()[edge.0 as usize].copies;
            assert_eq!(copies.len(), 1);
            let copy = &code.copies()[copies[0].0 as usize];
            assert!(copy.assignments.iter().all(|a| a.issue().is_none()));
            assert!(
                matches!(copy.steps.as_slice(), [CopyStatement::Assign { .. }]),
                "real executable Phi copy, not origin-only coverage"
            );
            let selected: Vec<_> = rendered
                .spans()
                .iter()
                .filter(
                    |s| matches!(s.origin, Origin::Transfer { transfer, .. } if transfer == edge),
                )
                .collect();
            assert_eq!(selected.len(), 1);
            let copied: Vec<_> = rendered
                .spans()
                .iter()
                .filter(|s| matches!(s.origin, Origin::Copy { copy, .. } if copy == copies[0]))
                .collect();
            assert_eq!(copied.len(), 1);
            let (_, end) = span_of_terminal(rendered, terminal_of(code, exit));
            assert!(copied[0].start.0 > end);
            assert!(copied[0].start.0 < copied[0].end.0);
            assert!(
                selected[0].start.0 <= copied[0].start.0 && copied[0].end.0 <= selected[0].end.0
            );
            assert!(copied[0].end.0 < rendered.text().find("GuardArms.mark(50)").unwrap());
        },
    );
}

#[test]
fn a_guard_and_later_false_work_are_not_misclassified_as_one_else_if() {
    inspect(
        "branches/terminal-guard-arms",
        "Lfixture/GuardArms;",
        "outer",
        "(III)I",
        |_, code, rendered| {
            assert_eq!(guards(code).len(), 1);
            assert_eq!(
                code.controls()
                    .iter()
                    .filter(|c| matches!(
                        c.layout,
                        EmissionLayout::If {
                            shape: IfShape::IfElse,
                            ..
                        }
                    ))
                    .count(),
                1
            );
            assert!(!code.controls().iter().any(|c| matches!(
                c.layout,
                EmissionLayout::If {
                    shape: IfShape::ElseIf,
                    ..
                }
            )));
            assert_eq!(rendered.text().matches("else {").count(), 1);
            assert!(
                rendered.text().find("GuardArms.mark(50)").unwrap()
                    < rendered.text().find("GuardArms.mark(70)").unwrap()
            );
        },
    );
}

#[test]
fn an_internal_recovering_try_cannot_supply_the_guard_proof() {
    inspect(
        "branches/terminal-guard-arms",
        "Lfixture/GuardArms;",
        "recover",
        "(II)I",
        |_, code, rendered| {
            assert!(guards(code).is_empty());
            assert!(
                code.controls()
                    .iter()
                    .any(|c| matches!(c.layout, EmissionLayout::Try { .. }))
            );
            assert!(rendered.text().contains("else {"));
            assert!(rendered.text().contains("GuardArms.mark(60)"));
            assert!(rendered.text().contains("GuardArms.mark(70)"));
        },
    );
}

#[test]
fn completed_cleanup_precedes_every_guard_predicate_and_return() {
    inspect(
        "branches/terminal-guard-arms",
        "Lfixture/GuardArms;",
        "after",
        "(II)I",
        |facts, code, rendered| {
            assert_eq!(
                facts
                    .tries()
                    .iter()
                    .filter(|p| p.finally().is_some())
                    .count(),
                1
            );
            assert_eq!(guards(code).len(), 1);
            assert!(
                rendered.text().find("GuardArms.cleanup(").unwrap()
                    < rendered.text().find("GuardArms.probe(").unwrap()
            );
            assert!(!rendered.text().contains("else {"));
        },
    );
}

#[test]
fn a_constructed_throw_keeps_its_selected_guard_and_normal_effects() {
    inspect(
        "branches/terminal-guard-arms",
        "Lfixture/GuardArms;",
        "throwing",
        "(II)I",
        |_, code, rendered| {
            let gs = guards(code);
            assert_eq!(gs.len(), 1);
            assert!(!gs[0].4);
            assert!(matches!(
                code.terminals()[terminal_of(code, gs[0].2).0 as usize].kind,
                TerminalKind::Throw(_)
            ));
            assert!(
                rendered.text().find("throw ").unwrap()
                    < rendered.text().find("GuardArms.mark(50)").unwrap()
            );
            assert!(!rendered.text().contains("else {"));
        },
    );
}

#[test]
fn an_early_void_return_is_not_omitted_after_flattening_the_other_arm() {
    inspect(
        "branches/terminal-guard-arms",
        "Lfixture/GuardArms;",
        "voided",
        "(I)V",
        |_, code, rendered| {
            let gs = guards(code);
            assert_eq!(gs.len(), 1);
            let terminal = terminal_of(code, gs[0].2);
            assert!(matches!(
                code.terminals()[terminal.0 as usize].kind,
                TerminalKind::Return(None)
            ));
            assert!(!code.emission().omitted_return(terminal));
            let (start, end) = span_of_terminal(rendered, terminal);
            assert!(rendered.text()[start..end].contains("return;"));
            assert!(!rendered.text().contains("else {"));
        },
    );
}

#[test]
fn the_flattened_body_declaration_keeps_its_original_initializer_position() {
    inspect(
        "branches/terminal-guard-arms",
        "Lfixture/GuardArms;",
        "declared",
        "(II)I",
        |_, code, rendered| {
            let gs = guards(code);
            assert_eq!(gs.len(), 1);
            let (operation, statement, local) = code
                .operations()
                .iter()
                .find_map(|op| {
                    if !matches!(op.original, diana_decomp::p2::OpKind::Invoke { .. })
                        || code.nested_operation(op.source)
                    {
                        return None;
                    }
                    let id = op.statement?;
                    let [definition] = code.statement(id).unwrap().outputs.as_slice() else {
                        return None;
                    };
                    let diana_decomp::p5::Materialization::Store(local) =
                        definition.materialization()
                    else {
                        return None;
                    };
                    Some((op.source, id, diana_decomp::p7::StorageId(local.0)))
                })
                .expect("the read call's ordinary stored result");
            assert_eq!(code.emission().local_initializer(local), Some(statement));
            let span = rendered.spans().iter().find(|s| matches!(s.origin, Origin::Operation { operation: id, .. } if id == operation)).unwrap();
            let (_, end) = span_of_terminal(rendered, terminal_of(code, gs[0].2));
            assert!(span.start.0 > end);
            assert!(rendered.text()[span.start.0..span.end.0].contains("int "));
            assert!(!rendered.text().contains("else {"));
        },
    );
}

#[test]
fn owning_protection_facts_authorize_the_guard_inside_the_original_loop() {
    inspect(
        "branches/terminal-guard-arms",
        "Lfixture/GuardArms;",
        "loop",
        "(III)I",
        |facts, code, rendered| {
            assert_eq!(guards(code).len(), 1);
            let [loop_plan] = facts.loops() else {
                panic!("one original loop")
            };
            let scope = facts
                .tries()
                .iter()
                .find(|p| p.finally().is_some())
                .unwrap();
            assert!(
                loop_plan
                    .lexical_members()
                    .iter()
                    .all(|n| scope.body().contains(n))
            );
            assert_eq!(scope.entry(), loop_plan.header());
            assert_eq!(loop_plan.private_exits().len(), 1);
            assert!(!rendered.text().contains("[exn c0]"));
            assert_eq!(rendered.text().matches("GuardArms.cleanup(").count(), 1);
            assert!(rendered.text().contains("finally {"));
            assert!(!rendered.text().contains("goto L"));
            assert!(!rendered.text().contains("for ("));
        },
    );
}
