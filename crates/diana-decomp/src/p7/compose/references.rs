//! Which references and exception chains need their own annotation.

use super::super::*;
use super::*;
use crate::p5::ThrowSite;

/// A field reference is owner, name and type. The type is printed only when two
/// delivered references share owner and name but not type, or when the original
/// name form or a use issue already requires an explicit reference.
pub(super) fn field_qualifications(code: &MethodCode) -> Result<Vec<bool>, ExpressionError> {
    let mut explicit = filled(code.references.len(), false)?;
    for (index, reference) in code.references.iter().enumerate() {
        let Ok(ReferenceDeclaration::Member(MemberDeclaration::Field {
            owner,
            name,
            field_type,
        })) = &reference.declaration
        else {
            continue;
        };
        let owner_units = &code.types[owner.0 as usize].units;
        let field_units = &code.types[field_type.0 as usize].units;
        explicit[index] = name.form == NameForm::Original || reference.use_issue.is_some();
        if explicit[index] {
            continue;
        }
        for (other_index, other) in code.references.iter().enumerate() {
            if other_index == index {
                continue;
            }
            let Ok(ReferenceDeclaration::Member(MemberDeclaration::Field {
                owner: other_owner,
                name: other_name,
                field_type: other_type,
            })) = &other.declaration
            else {
                continue;
            };
            if other_name.units == name.units
                && code.types[other_owner.0 as usize].units == *owner_units
                && code.types[other_type.0 as usize].units != *field_units
            {
                explicit[index] = true;
                break;
            }
        }
    }
    Ok(explicit)
}

/// A natural route already printed as enclosing catches/finally needs no repeated
/// annotation. Every original clause must be represented in order: a printed
/// handler of an enclosing Try, delivered implicit monitor cleanup, or the
/// method-level uncaught propagation. Resolved clauses nest inner-first; two
/// clauses in one printed Try keep increasing handler positions, and an equal
/// position needs one printed union group that lists exactly those types. Any
/// other reordering keeps its explicit chain.
pub(super) fn site_annotations(code: &MethodCode) -> Result<Vec<bool>, ExpressionError> {
    let mut parent: Vec<Option<ControlId>> = filled(code.controls.len(), None)?;
    for control in &code.controls {
        for &child in &control.children {
            let slot = parent
                .get_mut(child.0 as usize)
                .ok_or(bug("control names an unknown child"))?;
            if slot.replace(control.id).is_some() {
                return Err(bug("control has two containment owners"));
            }
        }
    }
    let mut owner: Vec<Option<ControlId>> = filled(code.labels.len(), None)?;
    for label in &code.labels {
        *owner
            .get_mut(label.node.0 as usize)
            .ok_or(bug("label names an unknown node"))? = Some(label.owner);
    }
    let mut node_of_block: Vec<Option<u32>> = filled(code.terminals.len(), None)?;
    for (node, block) in code.emission.blocks.iter().enumerate() {
        if let BlockEnd::Terminal(terminal) = block.end {
            *node_of_block
                .get_mut(terminal.0 as usize)
                .ok_or(bug("block end names an unknown terminal"))? =
                Some(u32::try_from(node).map_err(|_| crate::RequestError::PlatformCapacity)?);
        }
    }
    let mut required = filled(code.sites.len(), true)?;
    for site in &code.sites {
        if matches!(
            site.syntax,
            ExceptionSyntax::Propagate | ExceptionSyntax::ImpossibleRelease
        ) {
            required[site.id.0 as usize] = false;
            continue;
        }
        if !matches!(site.syntax, ExceptionSyntax::Natural) {
            continue;
        }
        let clauses = &code.chains[site.chain.0 as usize].clauses;
        if site.routes.len() != clauses.len() || clauses.is_empty() {
            continue;
        }
        let node = match site.original {
            ThrowSite::Operation(op) => code.operations.get(op.0 as usize).map(|o| o.node),
            ThrowSite::Terminal(block) => node_of_block
                .get(block.0 as usize)
                .copied()
                .flatten()
                .map(crate::p6::NodeId),
        };
        let Some(start) = node.and_then(|node| owner.get(node.0 as usize).copied().flatten())
        else {
            continue;
        };
        let mut resolved: Vec<(ControlId, usize, u32)> = Vec::new();
        let mut previous: Option<ControlId> = None;
        let mut represented = true;
        for (offset, (route, clause)) in site.routes.iter().zip(clauses).enumerate() {
            let Some(label) = clause.target else {
                represented = matches!(route.delivery, Delivery::Explicit);
                if !represented {
                    break;
                }
                continue;
            };
            let Delivery::Catch(target) = route.delivery else {
                represented = false;
                break;
            };
            if !super::sequence::contains(&parent, start, target)
                || previous.is_some_and(|inner| !super::sequence::contains(&parent, inner, target))
            {
                represented = false;
                break;
            }
            let ControlKind::Try { handlers, .. } = &code.controls[target.0 as usize].kind else {
                represented = false;
                break;
            };
            let Some(index) = handlers.iter().position(|node| node.0 == label.0) else {
                represented = false;
                break;
            };
            let printed = matches!(&code.emission.controls[target.0 as usize].layout,
                    EmissionLayout::Try { catches, finally, .. } if finally.is_some() || index < catches.len())
                || code.controls.iter().any(|control| {
                    matches!(&control.kind,
                    ControlKind::Synchronized { implicit: Some(surface), .. }
                        if surface.cleanups.contains(&(target, index as u32))
                            && super::sequence::contains(&parent, start, control.id))
                });
            let position =
                u32::try_from(offset).map_err(|_| crate::RequestError::PlatformCapacity)?;
            if !printed
                || resolved.iter().any(|&(control, earlier, at)| {
                    control == target
                        && (index < earlier
                            || (index == earlier && !union(code, target, at, position)))
                })
            {
                represented = false;
                break;
            }
            super::super::build::push(&mut resolved, (target, index, position))?;
            previous = Some(target);
        }
        if represented {
            required[site.id.0 as usize] = false;
        }
    }
    Ok(required)
}

/// One printed union catch clause lists every original type of its group, so two
/// clauses sharing that handler position keep both types without a chain line.
fn union(code: &MethodCode, control: ControlId, first: u32, second: u32) -> bool {
    let Some(ControlKind::Try {
        handler_clauses, ..
    }) = code.controls.get(control.0 as usize).map(|c| &c.kind)
    else {
        return false;
    };
    handler_clauses.iter().any(|group| {
        group.start <= first && first < group.end && group.start <= second && second < group.end
    })
}

/// Owner omission depends on the receiver that still prints after composition.
pub(super) fn call_owners(code: &mut MethodCode) -> Result<(), ExpressionError> {
    for index in 0..code.expressions.len() {
        let ExprKind::Call(call) = &code.expressions[index].kind else {
            continue;
        };
        let owner = match &code.references[call.target.0 as usize].declaration {
            Ok(ReferenceDeclaration::Member(MemberDeclaration::Method { owner, .. })) => *owner,
            _ => continue,
        };
        let disclose = call.receiver.is_some_and(|receiver| {
            if code.emission.composed_use(receiver).is_some()
                || code.emission.inline_checks.iter().any(|check| check.occurrence == receiver)
            {
                return true;
            }
            let ExprKind::Read { storage, .. } = code.expressions[receiver.0 as usize].kind else {
                return true;
            };
            !matches!(code.emission.local_types[storage.0 as usize],
                LocalSpelling::Named(ty) if code.types[ty.0 as usize].units == code.types[owner.0 as usize].units)
        });
        let object_owner = !object_type(code, owner)
            && call
                .receiver
                .is_some_and(|receiver| object_receiver(code, receiver))
            && !matches!(&code.references[call.target.0 as usize].declaration,
                Ok(ReferenceDeclaration::Member(MemberDeclaration::Method { name, .. })) if name.units.iter().copied().eq("<init>".encode_utf16()));
        let ExprKind::Call(call) = &mut code.expressions[index].kind else {
            return Err(super::bug("owner selection lost its call"));
        };
        match &mut call.qualification {
            Qualification::Natural => {
                if object_owner {
                    call.qualification = Qualification::Owner;
                }
            }
            Qualification::Owner => {}
            Qualification::Parameters { owner } | Qualification::FullSignature { owner } => {
                *owner = disclose
            }
        }
    }
    Ok(())
}

fn object_receiver(code: &MethodCode, expression: ExprId) -> bool {
    if code.emission.composed_use(expression).is_some()
        || code
            .emission
            .inline_checks
            .iter()
            .any(|check| check.occurrence == expression)
    {
        return false;
    }
    let Some(ExprKind::Read { storage, .. }) = code
        .expressions
        .get(expression.0 as usize)
        .map(|expression| &expression.kind)
    else {
        return false;
    };
    matches!(code.emission.local_types[storage.0 as usize], LocalSpelling::Named(ty)
        if code.types[ty.0 as usize].units.iter().copied().eq(b"Ljava/lang/Object;".iter().copied().map(u16::from)))
}

fn object_type(code: &MethodCode, ty: TypeUseId) -> bool {
    code.types[ty.0 as usize]
        .units
        .iter()
        .copied()
        .eq(b"Ljava/lang/Object;".iter().copied().map(u16::from))
}

pub(super) fn field_owners(code: &mut MethodCode) -> Result<(), ExpressionError> {
    let mut owners = filled(code.references.len(), None)?;
    let mut select = |reference: ReferenceId, receiver: ExprId| {
        if !object_receiver(code, receiver) {
            return;
        }
        if let Ok(ReferenceDeclaration::Member(MemberDeclaration::Field { owner, .. })) =
            &code.references[reference.0 as usize].declaration
            && !object_type(code, *owner)
        {
            owners[reference.0 as usize] = Some(*owner);
        }
    };
    for expression in &code.expressions {
        if let ExprKind::Field {
            reference,
            receiver: Some(receiver),
        } = expression.kind
        {
            select(reference, receiver);
        }
    }
    for statement in &code.statements {
        if let StatementKind::FieldStore {
            reference,
            receiver: Some(receiver),
            ..
        } = statement.kind
        {
            select(reference, receiver);
        }
    }
    code.emission.field_receiver_owners = owners;
    Ok(())
}
