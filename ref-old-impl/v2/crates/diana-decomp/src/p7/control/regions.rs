//! Copies selected P6 control roles without reconstructing regions.

use super::mapping::*;
use super::*;
use crate::p7::build::{bug, push};
use crate::p7::values::Context;
fn kind(c: &mut Context<'_, '_>, r: &p6::Region) -> Result<ControlKind, ExpressionError> {
    let f = c.planned.facts();
    Ok(match r.kind() {
        RegionKind::Block(n) => ControlKind::Block { node: *n },
        RegionKind::Sequence => ControlKind::Sequence,
        RegionKind::If { .. } => ControlKind::If,
        RegionKind::Condition(id) => condition(c, &f.conditions()[id.0 as usize])?,
        RegionKind::Loop(id) => {
            let p = &f.loops()[id.0 as usize];
            ControlKind::Loop {
                continue_target: label(c, p.continue_target())?,
            }
        }
        RegionKind::Switch { arms, .. } => ControlKind::Switch {
            arms: ids(arms, |a| TransferId(a.0))?,
        },
        RegionKind::LabelledBlock { target } => ControlKind::LabelledBlock {
            target: label(c, *target)?,
        },
        RegionKind::Try(id) => {
            let p = &f.tries()[id.0 as usize];
            ControlKind::Try {
                bindings: {
                    let mut bindings = Vec::new();
                    for &capture in p.bindings() {
                        let binding = if let Some(operation) = capture {
                            let statement = statement(c, operation)?;
                            let [definition] =
                                c.code.require_statement(statement)?.outputs.as_slice()
                            else {
                                return Err(bug("catch binding lost its definition"));
                            };
                            let crate::p5::Materialization::ExceptionCapture(local) =
                                definition.materialization()
                            else {
                                return Err(bug("catch binding lost exception storage"));
                            };
                            Some(CatchBinding {
                                operation,
                                statement,
                                local: StorageId(local.0),
                            })
                        } else {
                            None
                        };
                        push(&mut bindings, binding)?;
                    }
                    bindings
                },
                chain: p.chain(),
                clauses: p.clauses(),
                handler_clauses: {
                    let mut groups = Vec::new();
                    for range in p.handler_clauses() {
                        push(&mut groups, range.clone())?;
                    }
                    groups
                },
                handlers: ids(p.handlers(), |n| n)?,
                planning_scope: p.outer_sync(),
                finally: p
                    .finally()
                    .map(|p| -> Result<Finally, ExpressionError> {
                        Ok(Finally {
                            capture: p.capture(),
                            rethrows: ids(p.rethrows(), |node| terminal(c, node))?,
                            terminals: ids(p.terminals(), |(a, b)| {
                                (terminal(c, a), terminal(c, b))
                            })?,
                            operations: ids(p.operations(), |pair| pair)?,
                            predicates: ids(p.predicates(), |(a, b)| {
                                (terminal(c, a), terminal(c, b))
                            })?,
                        })
                    })
                    .transpose()?,
            }
        }
        RegionKind::Synchronized(id) => {
            super::synchronized::construct(c, &f.synchronized()[id.0 as usize])?
        }
        RegionKind::Graph {
            reason,
            entries,
            exits,
        } => {
            let mut ent = Vec::new();
            for &n in entries {
                push(&mut ent, label(c, n)?)?;
            }
            ControlKind::Graph {
                reason: *reason,
                entries: ent,
                exits: ids(exits, |l| TransferId(l.0))?,
            }
        }
    })
}

fn condition(
    c: &Context<'_, '_>,
    plan: &p6::ConditionPlan,
) -> Result<ControlKind, ExpressionError> {
    let mut tests = Vec::new();
    for test in plan.tests() {
        push(
            &mut tests,
            ConditionTest {
                predicate: test.predicate(),
                select: TransferId(test.select().0),
                other: TransferId(test.other().0),
            },
        )?;
    }
    let explicit = plan
        .explicit()
        .ok_or(bug("selected condition has no explicit schedule"))?;
    let mut completions = Vec::new();
    for completion in explicit.completions() {
        push(
            &mut completions,
            ConditionCompletion {
                child: ControlId(completion.child().0),
                continuation: continuation(c, completion.continuation())?,
            },
        )?;
    }
    Ok(ControlKind::Condition {
        tests,
        routing: ids(plan.routing(), |l| TransferId(l.0))?,
        else_arm: plan.else_arm(),
        other_exit: plan.other_exit(),
        explicit: ConditionSchedule {
            children: ids(explicit.children(), |r| ControlId(r.0))?,
            completions,
            gotos: ids(explicit.gotos(), |l| TransferId(l.0))?,
        },
    })
}

fn decision(
    c: &Context<'_, '_>,
    plan: Option<&p6::DecisionPlan>,
) -> Result<Option<Decision>, ExpressionError> {
    Ok(if let Some(plan) = plan {
        let mut arms = Vec::new();
        for test in plan.tests() {
            let block = terminal(c, test.predicate());
            let terminal = &c.code.terminals[block.0 as usize];
            let (selector, key_expression) = match terminal.kind {
                TerminalKind::If {
                    condition: Condition::Zero { value, .. },
                    ..
                } => (value, None),
                TerminalKind::If {
                    condition: Condition::Pair { left, right, .. },
                    ..
                } => {
                    if test.argument() == 0 {
                        (left, Some(right))
                    } else {
                        (right, Some(left))
                    }
                }
                _ => return Err(bug("decision test lost its original branch operands")),
            };
            push(
                &mut arms,
                DecisionArm {
                    source: ControlId(test.region().0),
                    header: ControlId(test.header().0),
                    terminal: block,
                    predicate: test.predicate(),
                    selector,
                    key_expression,
                    key: test.key(),
                    selected: TransferId(test.selected().0),
                    remaining: TransferId(test.remaining().0),
                    body: ControlId(test.body().0),
                },
            )?;
        }
        Some(Decision {
            arms,
            default: ControlId(plan.default().0),
        })
    } else {
        None
    })
}

pub(super) fn construct(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    let f = c.planned.facts();
    for (index, record) in f.controls().iter().enumerate() {
        let source = record.source.map(|id| &f.regions()[id.0 as usize]);
        let kind = if let Some(r) = source {
            kind(c, r)?
        } else if let EmissionLayout::Loop {
            continue_target, ..
        } = record.layout
        {
            ControlKind::Loop {
                continue_target: label(c, continue_target)?,
            }
        } else {
            ControlKind::Sequence
        };
        let decision = decision(c, source.and_then(|r| r.decision()))?;
        let source_continuation = continuation(
            c,
            source.map_or(p6::Continuation::None, |r| r.continuation()),
        )?;
        let selected_continuation = record
            .continuation
            .map(|value| continuation(c, value))
            .transpose()?;
        let completion = selected_continuation
            .as_ref()
            .unwrap_or(&source_continuation)
            .completion();
        let source_children = ids(source.map_or(&[], |r| r.children()), |r| r)?.into();
        let id = ControlId(super::super::build::id(index)?);
        let entry = record.entry.map(|n| label(c, n)).transpose()?;
        let follow = record.follow.map(|n| label(c, n)).transpose()?;
        push(
            &mut c.code.controls,
            Control {
                decision,
                id,
                kind,
                source_children,
                children: record.children.clone(),
                source_continuation,
                label: crate::p7::references::text(format_args!("R{}", id.0))?,
                label_required: true,
                omitted: false,
                transparent: false,
                source_only: ids(&record.source_only, |r| r)?,
                layout: record.layout.clone(),
                entry,
                follow,
                continuation: completion,
            },
        )?;
    }
    Ok(())
}
