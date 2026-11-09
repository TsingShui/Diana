//! Binds original copy locations to phases and transfers without rescheduling.

use super::{EmissionPlan, EmissionTransfer, TransferSpelling};
use crate::p5::{CopyLocation, ThrowSite, TransferSite};
use crate::p7::build::{bug, filled, push};
use crate::p7::{CopyEmission, ExpressionError, MethodCode, TransferId, TransferKind};

pub(super) fn transfers(
    code: &MethodCode,
    plan: &mut EmissionPlan,
) -> Result<Vec<Option<TransferId>>, ExpressionError> {
    let mut edge_links = filled(
        code.transfers
            .iter()
            .filter_map(|t| t.edge)
            .map(|e| e.0)
            .max()
            .map(|n| {
                usize::try_from(n)
                    .ok()
                    .and_then(|n| n.checked_add(1))
                    .ok_or(ExpressionError::from(crate::RequestError::PlatformCapacity))
            })
            .transpose()?
            .unwrap_or(0),
        None,
    )?;
    for t in &code.transfers {
        if let Some(edge) = t.edge {
            edge_links[edge.0 as usize] = Some(t.id);
        }
        let spelling = match t.kind {
            TransferKind::Goto(l) => TransferSpelling::Goto(l),
            TransferKind::Next(_) => TransferSpelling::Fallthrough,
            TransferKind::Break(c) => TransferSpelling::Break(c),
            TransferKind::Continue(c) => TransferSpelling::Continue(c),
            TransferKind::LeaveSync { .. } => TransferSpelling::Goto(t.destination),
            // P6's release-block continuation carries this exact outlet. An
            // ordinary goto would execute first and bypass its Continue/Break
            // delivery, including original ordinary body work after release.
            // Copies and any original routing nodes retain their own phases.
            TransferKind::ReleasedSync { .. } => TransferSpelling::Fallthrough,
            TransferKind::SwitchArm(_) => TransferSpelling::Goto(t.destination),
            TransferKind::IfArm { .. }
            | TransferKind::LoopCondition { .. }
            | TransferKind::EnterSync(_) => TransferSpelling::Selected,
        };
        push(
            &mut plan.transfers,
            EmissionTransfer {
                copies: Vec::new(),
                spelling,
            },
        )?;
    }
    Ok(edge_links)
}

pub(super) fn copies(
    code: &MethodCode,
    plan: &mut EmissionPlan,
    edge_links: &[Option<TransferId>],
) -> Result<(), ExpressionError> {
    for bundle in &code.copies {
        if matches!(bundle.emission, CopyEmission::Construction(_)) {
            continue;
        }
        let at = bundle.id;
        match bundle.location {
            CopyLocation::Transfer {
                site: TransferSite::InitialEntry,
                ..
            } => push(&mut plan.initial, at)?,
            CopyLocation::Transfer {
                site: TransferSite::BeforeOperation(op),
                ..
            } => push(&mut plan.operations[op.0 as usize].before, at)?,
            CopyLocation::BeforeThrow(ThrowSite::Operation(op)) => {
                push(&mut plan.operations[op.0 as usize].snapshots, at)?
            }
            CopyLocation::Transfer {
                site: TransferSite::AfterCommit(op),
                ..
            } => push(&mut plan.operations[op.0 as usize].after, at)?,
            CopyLocation::Transfer {
                site: TransferSite::BeforeTerminal(b),
                ..
            } => push(&mut plan.terminals[b.0 as usize].before, at)?,
            CopyLocation::BeforeThrow(ThrowSite::Terminal(b)) => {
                push(&mut plan.terminals[b.0 as usize].snapshots, at)?
            }
            CopyLocation::Transfer {
                site: TransferSite::NormalEdge(edge),
                ..
            } => {
                let link = edge_links
                    .get(edge.0 as usize)
                    .copied()
                    .flatten()
                    .ok_or(bug("copy location lost its original normal transfer"))?;
                push(&mut plan.transfers[link.0 as usize].copies, at)?;
            }
            CopyLocation::HandlerEntry { .. } => {}
        }
    }
    for g in &code.constructions {
        if let Some(prelude) = g.prelude {
            push(
                &mut plan.operations[g.allocation.0 as usize].snapshots,
                prelude,
            )?;
        }
    }
    Ok(())
}
