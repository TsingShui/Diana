//! Planned continuations behind completed display exits print nothing.

use super::super::*;
use super::*;

#[derive(Clone, Copy)]
enum TailExit {
    Unconditional,
    Scope(ControlId),
}

fn transfer_exit(code: &MethodCode, id: TransferId) -> Option<TailExit> {
    match code.emission.transfers[id.0 as usize].spelling {
        TransferSpelling::Goto(_) => Some(TailExit::Unconditional),
        TransferSpelling::Break(scope)
        | TransferSpelling::Continue(scope)
        | TransferSpelling::BreakHere(scope)
        | TransferSpelling::ContinueHere(scope) => Some(TailExit::Scope(scope)),
        TransferSpelling::Fallthrough | TransferSpelling::Selected => None,
    }
}

/// A planned continuation cannot execute after an unconditional printed exit.
/// Only a last-child Sequence chain carries this tail fact upward. Scope exits
/// stop before their own target, and protected/loop/branch containers are not
/// transparent: their completion can still perform work or receive an arrival.
pub(super) fn drop_completed_continuations(code: &mut MethodCode) -> Result<(), ExpressionError> {
    let mut parent: Vec<Option<ControlId>> = filled(code.controls.len(), None)?;
    for control in &code.controls {
        for &child in &control.children {
            *parent
                .get_mut(child.0 as usize)
                .ok_or(bug("control names an unknown child"))? = Some(control.id);
        }
    }
    let mut owner: Vec<Option<ControlId>> = filled(code.labels.len(), None)?;
    for label in &code.labels {
        *owner
            .get_mut(label.node.0 as usize)
            .ok_or(bug("label names an unknown node"))? = Some(label.owner);
    }
    let mut jumps = filled(code.blocks.len(), None)?;
    for (node, block) in code.emission.blocks.iter().enumerate() {
        jumps[node] = match &block.end {
            BlockEnd::Terminal(terminal)
                if !code.emission.origin_only_terminals[terminal.0 as usize] =>
            {
                match code.terminals[terminal.0 as usize].kind {
                    TerminalKind::Goto(transfer) => transfer_exit(code, transfer),
                    TerminalKind::Return(_) | TerminalKind::Throw(_) => {
                        Some(TailExit::Unconditional)
                    }
                    _ => None,
                }
            }
            BlockEnd::Transfers(transfers) => {
                transfers.last().and_then(|&id| transfer_exit(code, id))
            }
            _ => None,
        };
    }
    for control in 0..code.emission.controls.len() {
        if code.emission.controls[control].continuation.is_none() {
            continue;
        }
        for (node, jump) in jumps.iter().enumerate() {
            let Some(jump) = jump else { continue };
            let Some(mut current) = owner.get(node).copied().flatten() else {
                continue;
            };
            let mut last = false;
            for _ in 0..=code.controls.len() {
                if matches!(jump, TailExit::Scope(target) if *target == current) {
                    break;
                }
                if current.0 as usize == control {
                    last = true;
                    break;
                }
                let Some(up) = parent.get(current.0 as usize).copied().flatten() else {
                    break;
                };
                let Some(parent_control) = code.controls.get(up.0 as usize) else {
                    break;
                };
                let emitted = &code.emission.controls[up.0 as usize];
                if code.emission.controls[current.0 as usize]
                    .continuation
                    .is_some()
                    || !matches!(emitted.layout, EmissionLayout::Sequence)
                    || emitted.children(parent_control).last() != Some(&current)
                {
                    break;
                }
                current = up;
            }
            if last {
                code.emission.controls[control].continuation = None;
                break;
            }
        }
    }
    Ok(())
}

pub(super) fn collapse_planned_continuations(code: &mut MethodCode) -> Result<(), ExpressionError> {
    let mut parent = filled(code.controls.len(), None)?;
    let mut buffer = filled(code.controls.len().max(3), ControlId(0))?;
    for control in &code.controls {
        for &child in &control.children {
            *parent
                .get_mut(child.0 as usize)
                .ok_or(bug("control names an unknown child"))? = Some(control.id);
        }
    }
    for index in 0..code.emission.controls.len() {
        let Some(ControlCompletion::Goto(label)) = code.emission.controls[index].continuation
        else {
            continue;
        };
        let control = ControlId(super::build::id(index)?);
        let Some(next) = super::sequence::printed_successor(code, &parent, control, &mut buffer)
        else {
            continue;
        };
        if super::sequence::printed_entry_is(code, next, label) {
            code.emission.controls[index].continuation = None;
        }
    }
    Ok(())
}
