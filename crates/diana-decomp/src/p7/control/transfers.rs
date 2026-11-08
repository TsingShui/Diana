//! Original labels and normal transfer roles, in producer order.

use super::mapping::*;
use super::*;
use crate::p7::build::{bug, push};
use crate::p7::values::Context;

pub(super) fn labels(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    let f = c.planned.facts();
    for p in f.labels() {
        let prefix = if p.kind() == p6::LabelKind::Handler {
            "H"
        } else {
            "L"
        };
        push(
            &mut c.code.labels,
            Label {
                id: LabelId(p.id().0),
                name: super::references::text(format_args!("{prefix}{}", p.id().0))?,
                node: p.node(),
                block: p.block(),
                owner: ControlId(p.owner().0),
                kind: p.kind(),
            },
        )?;
    }
    Ok(())
}
pub(super) fn transfers(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    let f = c.planned.facts();
    let flow = c.planned.execution();
    for (link, plan) in f.links().iter().zip(f.link_plans()) {
        let kind = match plan.action() {
            TransferAction::Goto(id) => TransferKind::Goto(LabelId(id.0)),
            TransferAction::Next(n) => TransferKind::Next(label(c, n)?),
            TransferAction::Break(id) => TransferKind::Break(ControlId(id.0)),
            TransferAction::Continue(id) => {
                TransferKind::Continue(control(c.loops[id.0 as usize])?)
            }
            TransferAction::IfArm { region, taken } => TransferKind::IfArm {
                control: ControlId(region.0),
                taken,
            },
            TransferAction::SwitchArm { region } => TransferKind::SwitchArm(ControlId(region.0)),
            TransferAction::LoopCondition { region, repeat } => TransferKind::LoopCondition {
                control: ControlId(region.0),
                repeat,
            },
            TransferAction::EnterSynchronized(id) => TransferKind::EnterSync(ControlId(id.0)),
            TransferAction::LeaveSynchronized { region, release } => TransferKind::LeaveSync {
                control: ControlId(region.0),
                release: statement(c, release)?,
            },
            TransferAction::SynchronizedReleased { region, release } => {
                TransferKind::ReleasedSync {
                    control: ControlId(region.0),
                    release: statement(c, release)?,
                }
            }
        };
        let (edge, role, construction) = match link.origin() {
            LinkOrigin::WithinBlock(_) => (None, None, None),
            LinkOrigin::Edge(id) => {
                let EdgeRole::Normal(role) = flow.edges()[id.0 as usize].role() else {
                    return Err(bug("normal expression link lost its original role"));
                };
                (Some(id), Some(role), c.edge_groups[id.0 as usize])
            }
        };
        let destination = label(c, link.to())?;
        push(
            &mut c.code.transfers,
            Transfer {
                id: TransferId(link.id().0),
                from: link.from(),
                destination,
                edge,
                role,
                kind,
                copies: ids(plan.copies(), |p| CopyPlanId(p.0))?,
                construction,
            },
        )?;
    }
    Ok(())
}
