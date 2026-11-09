//! Lowers delivered P6 control roles and continuation ownership into layouts.

use super::{
    BlockEnd, ConditionElse, ControlCompletion, EmissionControl, EmissionLayout, EmissionPlan,
    FinallyEmission, IfShape, LoopPlacement, PredicateEmission, TransferSpelling,
};
use crate::p2::BlockId;
use crate::p7::build::{bug, copy, filled, push};
use crate::p7::{
    ConditionTest, Continuation, Control, ControlId, ControlKind, ExpressionError, Finally,
    LoopSyntax, MethodCode, TerminalKind, TransferId,
};

fn child(c: &Control, n: usize) -> Result<ControlId, ExpressionError> {
    c.children
        .get(n)
        .copied()
        .ok_or(bug("syntax role has no selected child"))
}

/// A test prints the condition that selects the shared arm. `select` is an
/// original conditional edge, so negation only inverts the delivered role.
fn chained_predicate(
    code: &MethodCode,
    test: &ConditionTest,
) -> Result<PredicateEmission, ExpressionError> {
    let block = code
        .blocks
        .get(test.predicate.0 as usize)
        .ok_or(bug("condition test lost its block"))?
        .original_block;
    let TerminalKind::If {
        when_true,
        when_false,
        ..
    } = code.terminals[block.0 as usize].kind
    else {
        return Err(bug("condition test lost its original branch"));
    };
    if test.select != when_true && test.select != when_false {
        return Err(bug("condition select is not an original branch edge"));
    }
    Ok(PredicateEmission {
        terminal: block,
        when_true,
        when_false,
        negate: test.select == when_false,
    })
}

fn predicate(
    code: &MethodCode,
    terminal: BlockId,
    repeat: Option<TransferId>,
) -> Result<PredicateEmission, ExpressionError> {
    let TerminalKind::If {
        when_true,
        when_false,
        ..
    } = code.terminals[terminal.0 as usize].kind
    else {
        return Err(bug("selected condition lost its original branch"));
    };
    if repeat.is_some_and(|e| e != when_true && e != when_false) {
        return Err(bug("loop repeat is not an original condition edge"));
    }
    Ok(PredicateEmission {
        terminal,
        when_true,
        when_false,
        negate: repeat == Some(when_false),
    })
}

fn layout(
    code: &MethodCode,
    plan: &mut EmissionPlan,
    c: &Control,
) -> Result<EmissionLayout, ExpressionError> {
    Ok(match &c.kind {
        ControlKind::Block { node } => EmissionLayout::Block(*node),
        ControlKind::Sequence => EmissionLayout::Sequence,
        ControlKind::Condition {
            tests,
            routing,
            else_arm,
            other_exit,
            ..
        } => condition_layout(code, c, tests, *else_arm, *other_exit, !routing.is_empty())?,
        ControlKind::If { terminal, .. } => EmissionLayout::If {
            header: child(c, 0)?,
            yes: child(c, 1)?,
            no: child(c, 2)?,
            test: predicate(code, *terminal, None)?,
            shape: IfShape::IfElse,
        },
        ControlKind::Loop { syntax, .. } => {
            let placement = match syntax {
                LoopSyntax::Infinite => LoopPlacement::Infinite,
                LoopSyntax::While {
                    terminal, repeat, ..
                } => LoopPlacement::While {
                    predicate: child(c, 1)?,
                    test: predicate(code, *terminal, Some(*repeat))?,
                    compact: false,
                },
                LoopSyntax::DoWhile {
                    terminal, repeat, ..
                } => LoopPlacement::DoWhile {
                    predicate: child(c, 1)?,
                    test: predicate(code, *terminal, Some(*repeat))?,
                    compact: false,
                },
            };
            EmissionLayout::Loop {
                body: child(c, 0)?,
                placement,
            }
        }
        ControlKind::Switch {
            terminal,
            arms,
            cases,
        } => switch_layout(plan, c, *terminal, arms, cases.as_deref())?,
        ControlKind::LabelledBlock { .. } => EmissionLayout::Labelled,
        ControlKind::Try { finally, .. } => try_layout(code, plan, c, finally.as_ref())?,
        ControlKind::Synchronized { acquire, .. } => EmissionLayout::Synchronized {
            entry: child(c, 0)?,
            body: child(c, 1)?,
            acquire: plan
                .statement_operation(*acquire)
                .ok_or(bug("synchronized acquire lost its original operation"))?,
        },
        ControlKind::Graph { .. } => EmissionLayout::Graph,
    })
}

fn condition_layout(
    code: &MethodCode,
    c: &Control,
    tests: &[ConditionTest],
    else_arm: bool,
    other_exit: bool,
    routing: bool,
) -> Result<EmissionLayout, ExpressionError> {
    let runs = tests.len();
    let mut emitted = Vec::new();
    let mut last = None;
    for test in tests {
        last = Some(test.other);
        push(&mut emitted, chained_predicate(code, test)?)?;
    }
    Ok(EmissionLayout::Condition {
        header: child(c, 0)?,
        tests: emitted,
        yes: child(c, runs)?,
        no: if else_arm {
            ConditionElse::Body(child(c, runs + 1)?)
        } else if other_exit {
            ConditionElse::Transfer
        } else {
            ConditionElse::None
        },
        fall: last.ok_or(bug("condition control has no test"))?,
        routing: routing
            .then(|| child(c, runs + 1 + usize::from(else_arm)))
            .transpose()?,
    })
}

fn switch_layout(
    plan: &mut EmissionPlan,
    c: &Control,
    terminal: BlockId,
    arms: &[TransferId],
    cases: Option<&[Vec<TransferId>]>,
) -> Result<EmissionLayout, ExpressionError> {
    let cases = if let Some(cases) = cases {
        if c.children.len() != cases.len() + 1 {
            return Err(bug("switch lost its case-group children"));
        }
        for &arm in arms {
            plan.transfers[arm.0 as usize].spelling = TransferSpelling::Fallthrough;
        }
        let mut groups = Vec::new();
        for group in cases {
            push(&mut groups, copy(group)?)?;
        }
        Some(groups)
    } else {
        None
    };
    Ok(EmissionLayout::Switch {
        header: child(c, 0)?,
        bodies: copy(c.children.get(1..).ok_or(bug("switch lost its body"))?)?,
        cases,
        terminal,
    })
}

fn try_layout(
    code: &MethodCode,
    plan: &mut EmissionPlan,
    c: &Control,
    finally: Option<&Finally>,
) -> Result<EmissionLayout, ExpressionError> {
    if let Some(finally) = finally {
        if c.children.len() != 3 {
            return Err(bug(
                "finally lost its protected, cleanup or duplicate child",
            ));
        }
        plan.origin_only_operations[finally.capture.0 as usize] = true;
        if let Some(statement) = code.operations[finally.capture.0 as usize].statement {
            for d in &code.statements[statement.0 as usize].outputs {
                if let crate::p5::Materialization::ExceptionCapture(local) = d.materialization() {
                    plan.hidden_locals[local.0 as usize] = true;
                }
            }
        }
        plan.origin_only_terminals[finally.rethrow.0 as usize] = true;
        for &(normal, _) in &finally.predicates {
            plan.origin_only_terminals[normal.0 as usize] = true;
        }
        for &(normal, _) in &finally.operations {
            plan.origin_only_operations[normal.0 as usize] = true;
            if let Some(statement) = code.operations[normal.0 as usize].statement {
                for d in &code.statements[statement.0 as usize].outputs {
                    if let crate::p5::Materialization::Store(local) = d.materialization() {
                        plan.hidden_locals[local.0 as usize] = true;
                    }
                }
            }
        }
        Ok(EmissionLayout::Try {
            body: child(c, 0)?,
            catches: copy(&c.children[1..2])?,
            finally: Some(FinallyEmission {
                cleanup: child(c, 1)?,
                duplicate: child(c, 2)?,
            }),
        })
    } else {
        Ok(EmissionLayout::Try {
            body: child(c, 0)?,
            catches: copy(
                c.children
                    .get(1..)
                    .ok_or(bug("try lost handler children"))?,
            )?,
            finally: None,
        })
    }
}

pub(super) fn construct(code: &MethodCode, plan: &mut EmissionPlan) -> Result<(), ExpressionError> {
    for c in &code.controls {
        let layout = layout(code, plan, c)?;
        let layout = if let Some(decision) = &c.decision {
            let mut bodies = Vec::new();
            for arm in &decision.arms {
                push(&mut bodies, arm.body)?;
            }
            push(&mut bodies, decision.default)?;
            EmissionLayout::When { bodies }
        } else {
            layout
        };
        push(
            &mut plan.controls,
            EmissionControl {
                label: crate::p7::references::text(format_args!("R{}", c.id.0))?,
                label_required: true,
                omitted: false,
                transparent: false,
                explicit_children: None,
                continuation: match c.continuation {
                    Continuation::Goto(label) => Some(ControlCompletion::Goto(label)),
                    Continuation::Break(scope) => Some(ControlCompletion::Break(scope)),
                    Continuation::Continue(scope) => Some(ControlCompletion::Continue(scope)),
                    _ => None,
                },
                layout,
            },
        )?;
    }
    Ok(())
}

pub(super) fn complete(code: &MethodCode, plan: &mut EmissionPlan) -> Result<(), ExpressionError> {
    for c in &code.controls {
        if let ControlKind::Try {
            finally: Some(_), ..
        } = c.kind
        {
            plan.controls[c.id.0 as usize].explicit_children = Some(copy(&c.children[..2])?);
        }
    }
    let mut headers = filled(plan.controls.len(), false)?;
    for c in &plan.controls {
        let header = match c.layout {
            EmissionLayout::If { header, .. } | EmissionLayout::Switch { header, .. } => {
                Some(header)
            }
            EmissionLayout::Loop {
                placement:
                    LoopPlacement::While { predicate, .. } | LoopPlacement::DoWhile { predicate, .. },
                ..
            } => Some(predicate),
            EmissionLayout::Synchronized { entry, .. } => Some(entry),
            _ => None,
        };
        if let Some(header) = header {
            headers[header.0 as usize] = true;
        }
    }
    for (i, c) in plan.controls.iter_mut().enumerate() {
        let deferred = match c.layout {
            EmissionLayout::Block(n) => match &plan.blocks[n.0 as usize].end {
                BlockEnd::None => true,
                BlockEnd::Transfers(ts) => ts.is_empty(),
                BlockEnd::Terminal(_) => false,
            },
            _ => false,
        };
        if headers[i] || deferred {
            c.continuation = None;
        }
    }
    Ok(())
}
