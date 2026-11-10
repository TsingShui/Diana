//! Planned continuations behind completed display exits print nothing.

use super::super::build::push;
use super::super::*;
use super::*;

#[derive(Clone, Copy)]
enum TailExit {
    Jump,
    Terminal,
    Scope(ControlId),
}

fn transfer_exit(code: &MethodCode, id: TransferId) -> Option<TailExit> {
    match code.emission.transfers[id.0 as usize].spelling {
        TransferSpelling::Goto(_) => Some(TailExit::Jump),
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
    let parent = super::sequence::parents(code)?;
    let mut owner: Vec<Option<ControlId>> = filled(code.labels.len(), None)?;
    for label in &code.labels {
        *owner
            .get_mut(label.node.0 as usize)
            .ok_or(bug("label names an unknown node"))? = Some(label.owner);
    }
    let mut jumps = filled(code.blocks.len(), None)?;
    for (node, block) in code.emission.blocks.iter().enumerate() {
        jumps[node] = block_exit(code, block);
    }
    for control in 0..code.controls.len() {
        if code.controls[control].continuation.is_none() {
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
                let Some(emitted) = code.controls.get(up.0 as usize) else {
                    break;
                };
                if code.controls[current.0 as usize].continuation.is_some()
                    || !matches!(emitted.layout, EmissionLayout::Sequence)
                    || emitted.children().last() != Some(&current)
                {
                    break;
                }
                current = up;
            }
            if last {
                code.controls[control].continuation = None;
                break;
            }
        }
    }
    Ok(())
}

// One classification of the selected spelling serves both display consumers;
// no original graph route is replayed.
fn block_exit(code: &MethodCode, block: &EmissionBlock) -> Option<TailExit> {
    match &block.end {
        BlockEnd::Terminal(terminal) if !code.emission.origin_only_terminal(*terminal) => {
            match code.terminals[terminal.0 as usize].kind {
                TerminalKind::Goto(transfer) => transfer_exit(code, transfer),
                TerminalKind::Return(_) if !code.emission.omitted_return(*terminal) => {
                    Some(TailExit::Terminal)
                }
                TerminalKind::Throw(_) => Some(TailExit::Terminal),
                _ => None,
            }
        }
        BlockEnd::Transfers(transfers) => transfers.last().and_then(|&id| transfer_exit(code, id)),
        _ => None,
    }
}

/// The whole arm is a selected linear statement schedule ending in a printed
/// return/throw. A recovering Try or any other opaque control anywhere in the
/// arm is not transparent, even if another source block later returns.
pub(super) fn ends_in_terminal_exit(
    code: &MethodCode,
    root: ControlId,
) -> Result<bool, ExpressionError> {
    let mut work = Vec::new();
    push(&mut work, root)?;
    let mut previous = None;
    while let Some(id) = work.pop() {
        let c = &code.controls[id.0 as usize];
        if c.omitted || c.label_required || c.continuation.is_some() {
            return Ok(false);
        }
        match c.layout {
            EmissionLayout::Sequence => {
                for &child in c.children().iter().rev() {
                    push(&mut work, child)?;
                }
            }
            EmissionLayout::Block(node) => {
                let block = &code.emission.blocks[node.0 as usize];
                if block.show_label
                    || block
                        .actions
                        .iter()
                        .any(|a| !matches!(a, BodyAction::Statement { .. }))
                {
                    return Ok(false);
                }
                if let Some(old) = previous
                    && !linear_prefix(code, &code.emission.blocks[old])
                {
                    return Ok(false);
                }
                previous = Some(node.0 as usize);
            }
            _ => return Ok(false),
        }
    }
    Ok(previous.is_some_and(|last| {
        matches!(
            block_exit(code, &code.emission.blocks[last]),
            Some(TailExit::Terminal)
        )
    }))
}

fn linear_prefix(code: &MethodCode, block: &EmissionBlock) -> bool {
    let fallthrough = |id: TransferId| {
        matches!(
            code.emission.transfers[id.0 as usize].spelling,
            TransferSpelling::Fallthrough
        )
    };
    match &block.end {
        BlockEnd::Annotated { .. } => false,
        BlockEnd::None => true,
        BlockEnd::Transfers(transfers) => transfers.iter().all(|&id| fallthrough(id)),
        BlockEnd::Terminal(id)
            if code.emission.origin_only_terminal(*id) || code.emission.omitted_return(*id) =>
        {
            true
        }
        BlockEnd::Terminal(id) => matches!(
            code.terminals[id.0 as usize].kind,
            TerminalKind::Goto(transfer) if fallthrough(transfer)
        ),
    }
}

pub(super) fn collapse_planned_continuations(code: &mut MethodCode) -> Result<(), ExpressionError> {
    let parent = super::sequence::parents(code)?;
    let mut buffer = filled(code.controls.len().max(3), ControlId(0))?;
    for index in 0..code.controls.len() {
        let Some(ControlCompletion::Goto(label)) = code.controls[index].continuation else {
            continue;
        };
        let control = ControlId(super::build::id(index)?);
        let Some(next) = super::sequence::printed_successor(code, &parent, control, &mut buffer)
        else {
            continue;
        };
        if super::sequence::printed_entry_is(code, next, label) {
            code.controls[index].continuation = None;
        }
    }
    Ok(())
}
