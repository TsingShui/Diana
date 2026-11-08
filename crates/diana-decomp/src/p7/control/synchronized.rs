//! Keeps release operands, copy phases and routing terminals in original order.

use super::mapping::*;
use super::*;
use crate::p7::build::{bug, push};
use crate::p7::values::Context;

fn outlet_copies(
    c: &Context<'_, '_>,
    exit: &p6::SyncExit,
    at: impl Fn(crate::p5::CopyLocation) -> bool,
) -> Result<Vec<CopyPlanId>, ExpressionError> {
    let mut out = Vec::new();
    for p in exit.after_release() {
        if at(c.planned.storage_facts().copy_plans()[p.0 as usize].location()) {
            push(&mut out, CopyPlanId(p.0))?;
        }
    }
    Ok(out)
}

fn outlet_transfer(
    c: &Context<'_, '_>,
    exit: &p6::SyncExit,
    link: p6::LinkId,
) -> Result<OutletStep, ExpressionError> {
    let copies = if let LinkOrigin::Edge(edge) = c.planned.facts().links()[link.0 as usize].origin()
    {
        outlet_copies(
            c,
            exit,
            |l| matches!(l,crate::p5::CopyLocation::Transfer{site:crate::p5::TransferSite::NormalEdge(id),..}if id==edge),
        )?
    } else {
        Vec::new()
    };
    Ok(OutletStep::Transfer {
        transfer: TransferId(link.0),
        copies,
    })
}

fn outlet_completion(
    c: &Context<'_, '_>,
    exit: &p6::SyncExit,
) -> Result<Vec<OutletStep>, ExpressionError> {
    use crate::p5::{CopyLocation, TransferSite};
    let mut steps = Vec::new();
    let commits = outlet_copies(
        c,
        exit,
        |l| matches!(l,CopyLocation::Transfer{site:TransferSite::AfterCommit(op),..}if op==exit.release()),
    )?;
    if !commits.is_empty() {
        push(&mut steps, OutletStep::Copies(commits))?;
    }
    push(&mut steps, outlet_transfer(c, exit, exit.release_link())?)?;
    for node in exit.routing() {
        let n = &c.planned.facts().nodes()[node.0 as usize];
        if n.has_terminal() {
            let before_operand = outlet_copies(
                c,
                exit,
                |l| matches!(l,CopyLocation::Transfer{site:TransferSite::BeforeTerminal(b),..}if b==n.block()),
            )?;
            let snapshots = outlet_copies(c, exit, |l| {
                l == CopyLocation::BeforeThrow(ThrowSite::Terminal(n.block()))
            })?;
            push(
                &mut steps,
                OutletStep::Terminal {
                    terminal: n.block(),
                    before_operand,
                    snapshots,
                },
            )?;
        }
        for link in &c.code.blocks[node.0 as usize].transfers {
            push(&mut steps, outlet_transfer(c, exit, p6::LinkId(link.0))?)?;
        }
    }
    Ok(steps)
}

pub(super) fn construct(
    c: &mut Context<'_, '_>,
    p: &p6::SyncPlan,
) -> Result<ControlKind, ExpressionError> {
    let f = c.planned.facts();
    let acquire = statement(c, p.enter())?;
    let StatementKind::Monitor { enter: true, lock } = c.code.statements[acquire.0 as usize].kind
    else {
        return Err(bug("synchronized acquire lost its monitor expression"));
    };
    let mut outlets = Vec::new();
    for exit in p.exits() {
        let release = statement(c, exit.release())?;
        let resume = match exit.kind() {
            SyncExitKind::Follow(n) => Resume::Follow(label(c, n)?),
            SyncExitKind::Loop {
                loop_id,
                kind,
                target,
            } => {
                let at = control(c.loops[loop_id.0 as usize])?;
                let target = label(c, target)?;
                match kind {
                    SyncLoopTransfer::Break => Resume::Break {
                        control: at,
                        target,
                    },
                    SyncLoopTransfer::Continue => Resume::Continue {
                        control: at,
                        target,
                    },
                }
            }
            SyncExitKind::ReturnVoid { terminal: n } => Resume::ReturnVoid {
                terminal: terminal(c, n),
            },
            SyncExitKind::Rethrow {
                terminal: n,
                capture,
                ..
            } => {
                let block = terminal(c, n);
                let TerminalKind::Throw(exception) = c.code.terminals[block.0 as usize].kind else {
                    return Err(bug("synchronized rethrow lost its original expression"));
                };
                Resume::Rethrow {
                    terminal: block,
                    exception,
                    capture,
                }
            }
        };
        // These are the already selected outlet phases. The original
        // operand read stays between before-operation and snapshot saves.
        let completion = outlet_completion(c, exit)?;
        let entry = label(c, c.code.operations[exit.release().0 as usize].node)?;
        let s = &mut c.code.statements[release.0 as usize];
        let before_operand = std::mem::take(&mut s.before);
        let snapshots = std::mem::take(&mut s.snapshots);
        s.after.clear();
        for n in exit.routing() {
            if f.nodes()[n.0 as usize].has_terminal() {
                let t = &mut c.code.terminals[f.nodes()[n.0 as usize].block().0 as usize];
                t.before.clear();
                t.snapshots.clear();
            }
        }
        push(
            &mut outlets,
            SyncOutlet {
                entry,
                release,
                transfer: TransferId(exit.release_link().0),
                destination: label(c, exit.target())?,
                resume,
                routing: ids(exit.routing(), |n| n)?,
                before_operand,
                snapshots,
                after_release: ids(exit.after_release(), |p| CopyPlanId(p.0))?,
                completion,
            },
        )?;
    }
    Ok(ControlKind::Synchronized {
        acquire,
        lock,
        enter: TransferId(p.enter_link().0),
        held: ids(p.held_nodes(), |n| n)?,
        outlets,
    })
}
