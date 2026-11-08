//! Places lowered operations and handler receives in their delivered blocks.

use super::{BlockEnd, BodyAction, EmissionBlock, EmissionPlan, TransferSpelling};
use crate::p2::OpId;
use crate::p7::build::{bug, filled, id, push};
use crate::p7::{
    BlockCompletion, Emission, ExpressionError, HandlerCapture, MethodCode, TerminalEmission,
};

pub(super) fn index_operations(
    code: &MethodCode,
    plan: &mut EmissionPlan,
) -> Result<Vec<Vec<OpId>>, ExpressionError> {
    let mut nodes = Vec::new();
    crate::substrate::owned::reserve(&mut nodes, code.blocks.len())?;
    nodes.resize_with(code.blocks.len(), Vec::new);
    for op in &code.operations {
        push(&mut nodes[op.node.0 as usize], op.source)?;
        if let Some(s) = op.statement {
            plan.statement_operations[s.0 as usize] = Some(op.source);
        }
    }
    Ok(nodes)
}

pub(super) fn construct(
    code: &MethodCode,
    plan: &mut EmissionPlan,
    nodes: &[Vec<OpId>],
) -> Result<(), ExpressionError> {
    let mut handlers = filled(code.blocks.len(), None)?;
    for (i, h) in code.handlers.iter().enumerate() {
        handlers[h.node.0 as usize] = Some(id(i)?);
    }
    for b in &code.blocks {
        let h = handlers[b.node.0 as usize];
        let mut actions = Vec::new();
        if let Some(handler) = h
            && matches!(
                code.handlers[handler as usize].capture,
                HandlerCapture::None
            )
        {
            push(&mut actions, BodyAction::Receive { handler })?;
        }
        for &op in &nodes[b.node.0 as usize] {
            let o = &code.operations[op.0 as usize];
            let action = match o.emission {
                Emission::Block { .. } | Emission::Catch(_) => Some(BodyAction::Statement {
                    operation: op,
                    statement: o
                        .statement
                        .ok_or(bug("emitted operation has no statement"))?,
                }),
                Emission::Construction(g)
                    if code.constructions[g.0 as usize].initialization == op =>
                {
                    Some(BodyAction::Statement {
                        operation: op,
                        statement: code.constructions[g.0 as usize].statement,
                    })
                }
                Emission::Release(scope) => Some(BodyAction::Release {
                    operation: op,
                    statement: o.statement.ok_or(bug("release has no statement"))?,
                    scope,
                }),
                Emission::Acquire(_) | Emission::Literal | Emission::Construction(_) => None,
            };
            if let Some(a) = action {
                push(&mut actions, a)?;
            }
            if let Some(handler) = h {
                let capture = match code.handlers[handler as usize].capture {
                    HandlerCapture::None => None,
                    HandlerCapture::Body(s) | HandlerCapture::Catch { statement: s, .. } => Some(s),
                };
                if capture.is_some() && capture == o.statement {
                    push(&mut actions, BodyAction::Receive { handler })?;
                }
            }
        }
        let end = match b.completion {
            BlockCompletion::Ordinary(t)
            | BlockCompletion::Syntax {
                terminal: t,
                owner: TerminalEmission::SyncExit(_),
            } => BlockEnd::Terminal(t),
            BlockCompletion::Syntax { .. } => BlockEnd::None,
            BlockCompletion::None => {
                let mut links = Vec::new();
                for &t in &b.transfers {
                    if code.transfers[t.0 as usize].construction.is_none()
                        && !matches!(
                            plan.transfers[t.0 as usize].spelling,
                            TransferSpelling::Selected
                        )
                    {
                        push(&mut links, t)?;
                    }
                }
                BlockEnd::Transfers(links)
            }
        };
        push(
            &mut plan.blocks,
            EmissionBlock {
                show_label: true,
                actions,
                end,
            },
        )?;
    }
    Ok(())
}
