//! Maps the delivered common control structure and retains P7-owned effect
//! presentation. P6 owns named child roles and original predicate polarity.

use super::{BlockEnd, EmissionLayout, EmissionPlan, LoopPlacement, TransferSpelling};
use crate::p7::build::{bug, copy, filled, push};
use crate::p7::{ControlKind, ExpressionError, Finally, MethodCode};

fn retain_cleanup_effects(
    code: &MethodCode,
    plan: &mut EmissionPlan,
    finally: &Finally,
) -> Result<(), ExpressionError> {
    plan.origin_only_operations[finally.capture.0 as usize] = Some(super::OriginOnly::Attachment);
    if let Some(statement) = code.operations[finally.capture.0 as usize].statement {
        for d in &code.require_statement(statement)?.outputs {
            if let crate::p5::Materialization::ExceptionCapture(local) = d.materialization() {
                plan.hidden_locals[local.0 as usize] = true;
            }
        }
    }
    for rethrow in &finally.rethrows {
        plan.origin_only_terminals[rethrow.0 as usize] = true;
    }
    for &(normal, _) in &finally.terminals {
        plan.origin_only_terminals[normal.0 as usize] = true;
    }
    for &(normal, _) in &finally.predicates {
        plan.origin_only_terminals[normal.0 as usize] = true;
    }
    for &(normal, _) in &finally.operations {
        plan.origin_only_operations[normal.0 as usize] = Some(super::OriginOnly::Attachment);
        if let Some(statement) = code.operations[normal.0 as usize].statement {
            for d in &code.require_statement(statement)?.outputs {
                if let crate::p5::Materialization::Store(local) = d.materialization() {
                    plan.hidden_locals[local.0 as usize] = true;
                }
            }
        }
    }
    for group in &code.constructions {
        if finally
            .operations
            .iter()
            .any(|&(normal, _)| normal == group.initialization)
        {
            for publication in &group.publications {
                plan.hidden_locals[publication.destination.0 as usize] = true;
            }
        }
    }
    Ok(())
}

pub(super) fn construct(code: &MethodCode, plan: &mut EmissionPlan) -> Result<(), ExpressionError> {
    for control in &code.controls {
        if let ControlKind::Try {
            finally: Some(finally),
            ..
        } = &control.kind
        {
            retain_cleanup_effects(code, plan, finally)?;
        }
        if let ControlKind::Switch { arms } = &control.kind
            && matches!(
                control.layout,
                EmissionLayout::Switch { cases: Some(_), .. }
            )
        {
            for &arm in arms {
                plan.transfers[arm.0 as usize].spelling = TransferSpelling::Fallthrough;
            }
        }
    }
    Ok(())
}

pub(super) fn complete(
    code: &mut MethodCode,
    plan: &mut EmissionPlan,
) -> Result<(), ExpressionError> {
    for c in &mut code.controls {
        if let ControlKind::Try {
            finally: Some(_), ..
        } = c.kind
        {
            c.children = copy(&c.source_children[..2])?.into();
        }
    }
    let mut headers = filled(code.controls.len(), false)?;
    for c in &code.controls {
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
    for (i, c) in code.controls.iter_mut().enumerate() {
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

/// Apply only the surface roles delivered by P6. Source-only cleanup children
/// retain their arenas and origins, while the boundary owns implicit release.
pub(super) fn surfaces(
    code: &mut MethodCode,
    plan: &mut EmissionPlan,
) -> Result<(), ExpressionError> {
    let mut cleanups = Vec::new();
    for control in &code.controls {
        if let ControlKind::Synchronized {
            implicit: Some(surface),
            outlets,
            ..
        } = &control.kind
        {
            for &cleanup in &surface.cleanups {
                push(&mut cleanups, cleanup)?;
            }
            for outlet in outlets {
                let operation = code
                    .statement_operation(outlet.release)
                    .ok_or(bug("implicit release lost its operation"))?;
                plan.origin_only_operations[operation.0 as usize] =
                    Some(super::OriginOnly::Attachment);
            }
            for &capture in &surface.captures {
                plan.origin_only_operations[capture.0 as usize] =
                    Some(super::OriginOnly::Attachment);
                if let Some(statement) = code.operations[capture.0 as usize].statement {
                    for output in &code.require_statement(statement)?.outputs {
                        if let crate::p5::Materialization::ExceptionCapture(local) =
                            output.materialization()
                        {
                            plan.hidden_locals[local.0 as usize] = true;
                        }
                    }
                }
            }
            for &block in &surface.rethrows {
                plan.origin_only_terminals[block.0 as usize] = true;
            }
        }
    }
    for (cleanup, index) in cleanups {
        let control = &mut code.controls[cleanup.0 as usize];
        let index = usize::try_from(index).map_err(|_| crate::RequestError::PlatformCapacity)?;
        if control.source_children.len().checked_sub(2) != Some(index) {
            return Err(bug("implicit monitor cleanup lost its last catch"));
        }
        if index == 0 {
            control.layout = EmissionLayout::Sequence;
        } else {
            let EmissionLayout::Try { catches, .. } = &mut control.layout else {
                return Err(bug("implicit cleanup lost its catch layout"));
            };
            catches.truncate(index)?;
        }
        control.children = copy(&control.source_children[..index + 1])?.into();
        push(&mut control.source_only, control.source_children[index + 1])?;
    }
    for control in &code.controls {
        let ControlKind::Try {
            bindings,
            finally: None,
            ..
        } = &control.kind
        else {
            continue;
        };
        if !matches!(
            code.controls[control.id.0 as usize].layout,
            EmissionLayout::Try { .. }
        ) {
            continue;
        }
        let EmissionLayout::Try { catches, .. } = &code.controls[control.id.0 as usize].layout
        else {
            continue;
        };
        let visible = catches.len();
        for binding in bindings.iter().take(visible).flatten() {
            plan.hidden_locals[binding.local.0 as usize] = true;
            for block in &mut plan.blocks {
                block.actions.retain(|action| !matches!(action, super::BodyAction::Statement { operation, .. } if *operation == binding.operation));
            }
        }
    }
    Ok(())
}
