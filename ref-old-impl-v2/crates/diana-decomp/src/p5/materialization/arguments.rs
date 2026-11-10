//! Argument readiness and ordered single-use preparation admission.

use super::candidate::Admission;
use super::*;

pub(super) struct Arguments<'f> {
    inputs: &'f [UseAccess],
    pub(super) reads: Vec<StorageRead>,
}

pub(super) fn reads<'f>(
    typed: &TypedMethod<'_>,
    f: &'f StorageFacts,
    init: crate::p2::OpId,
) -> Admission<Arguments<'f>> {
    let s = typed.ssa_facts();
    let mut arguments = Vec::new();
    let inputs = match f.operation_uses(init) {
        Some(Ok(v)) => v,
        Some(Err(g)) => return Err(Separation::MissingStorage(g).into()),
        None => {
            return Err(StorageError::InternalInvariant("constructor lost storage inputs").into());
        }
    };
    // Arguments must be ready at the constructor call.
    let init_point = s
        .operation_site(init)
        .ok_or(StorageError::InternalInvariant(
            "constructor lost its execute point",
        ))?
        .execute();
    for input in inputs.iter().skip(1) {
        let read = match input.read() {
            Ok(r) => r,
            Err(g) => return Err(Separation::MissingStorage(g).into()),
        };
        if !source_ready(typed, read, init_point) {
            return Err(Separation::ArgumentNotAvailable(input.site()).into());
        }
        push(&mut arguments, read)?;
    }
    Ok(Arguments {
        inputs,
        reads: arguments,
    })
}

pub(super) fn prepare(
    typed: &TypedMethod<'_>,
    f: &StorageFacts,
    allocation: crate::p2::OpId,
    init: crate::p2::OpId,
    preparations: &[crate::p2::OpId],
    arguments: &Arguments<'_>,
) -> Admission<Vec<PreparedArgument>> {
    let mut prepared_arguments = Vec::new();
    for (position, &operation) in preparations.iter().enumerate() {
        let prepared = prepare_argument(
            typed,
            f,
            allocation,
            init,
            operation,
            &preparations[position + 1..],
        )?;
        push(&mut prepared_arguments, prepared)?;
    }
    ordered_trees(
        typed,
        f,
        allocation,
        preparations,
        &prepared_arguments,
        arguments,
    )?;
    Ok(prepared_arguments)
}

// The printed expression evaluates each operand tree in order, then its parent.
// Accept only that exact original preparation sequence. One-use edges and later
// targets make this finite; no retry or expression solver is involved.
fn ordered_trees(
    typed: &TypedMethod<'_>,
    f: &StorageFacts,
    allocation: crate::p2::OpId,
    operations: &[crate::p2::OpId],
    prepared: &[PreparedArgument],
    arguments: &Arguments<'_>,
) -> Admission<()> {
    enum Work<'a> {
        Read(&'a UseAccess),
        Finish(crate::p2::OpId),
    }
    let anchor = typed
        .ssa_facts()
        .operation_site(allocation)
        .ok_or(StorageError::InternalInvariant(
            "allocation lost its execute point",
        ))?
        .execute();
    let mut work = Vec::new();
    for input in arguments.inputs.iter().skip(1).rev() {
        push(&mut work, Work::Read(input))?;
    }
    let mut next = 0usize;
    while let Some(item) = work.pop() {
        match item {
            Work::Read(input) => {
                if let Some(producer) = prepared.iter().find(|p| p.target == input.site()) {
                    let uses = match f.operation_uses(producer.operation) {
                        Some(Ok(uses)) => uses,
                        _ => {
                            return Err(Separation::InterveningOperation(producer.operation).into());
                        }
                    };
                    push(&mut work, Work::Finish(producer.operation))?;
                    for usage in uses.iter().rev() {
                        push(&mut work, Work::Read(usage))?;
                    }
                } else if !input
                    .read()
                    .is_ok_and(|read| source_ready(typed, read, anchor))
                {
                    return Err(Separation::ArgumentNotAvailable(input.site()).into());
                }
            }
            Work::Finish(operation) => {
                if operations.get(next) != Some(&operation) {
                    return Err(Separation::InterveningOperation(operation).into());
                }
                next = next
                    .checked_add(1)
                    .ok_or(StorageError::InternalInvariant("argument order overflow"))?;
            }
        }
    }
    if next != operations.len() {
        return Err(
            Separation::InterveningOperation(*operations.get(next).ok_or(
                StorageError::InternalInvariant("argument order exceeds preparation"),
            )?)
            .into(),
        );
    }
    Ok(())
}

fn prepare_argument(
    typed: &TypedMethod<'_>,
    f: &StorageFacts,
    allocation: crate::p2::OpId,
    init: crate::p2::OpId,
    operation: crate::p2::OpId,
    later: &[crate::p2::OpId],
) -> Admission<PreparedArgument> {
    let s = typed.ssa_facts();
    let flow = typed.execution();
    let op = &flow.operations()[operation.0 as usize];
    preparation_effects(typed, f, allocation, operation)?;
    let Some([definition]) = f.operation_definitions(operation) else {
        return Err(Separation::InterveningOperation(operation).into());
    };
    let Materialization::Store(local) = definition.materialization() else {
        return Err(Separation::InterveningOperation(operation).into());
    };
    // An operation that commits its result at a following result capture
    // keeps that commit position: the composed expression still delivers
    // the operation PC, result-commit PC and site as separate origins.
    let committed = match definition.commit() {
        crate::p2::CommitRule::AtOperation => true,
        crate::p2::CommitRule::AfterResult => op.result_pc().is_some(),
        _ => false,
    };
    if !committed || f.local_gap(local).is_some() {
        return Err(Separation::InterveningOperation(operation).into());
    }
    let Some(produced) = value(s, definition.state()) else {
        return Err(Separation::InterveningOperation(operation).into());
    };
    let Some(definition_state) = s.state(definition.state()) else {
        return Err(Separation::InterveningOperation(operation).into());
    };
    if !matches!(f.locals.get(local.0 as usize).map(|local| local.kind()), Some(LocalKind::ReferenceCarrier(value) | LocalKind::Value(value)) if value == produced)
    {
        return Err(Separation::InterveningOperation(operation).into());
    }
    let mut matching = s
        .uses()
        .iter()
        .filter(|usage| value(s, usage.state()) == Some(produced));
    let Some(usage) = matching.next() else {
        return Err(Separation::InterveningOperation(operation).into());
    };
    let UseSite::Operation {
        operation: target,
        argument,
    } = usage.site()
    else {
        return Err(Separation::InterveningOperation(operation).into());
    };
    if !(target == init && argument != 0 || later.contains(&target))
        || s.state(usage.state())
            .is_none_or(|state| !state.availability().is_available())
        || matching.next().is_some()
        || s.uses().iter().any(|other| {
            other.slot() == definition_state.slot() && value(s, other.state()).is_none()
        })
        || s.phis().iter().any(|phi| {
            phi.incoming().iter().any(|incoming| {
                value(s, incoming.state()) == Some(produced)
                    || (phi.slot() == definition_state.slot()
                        && value(s, incoming.state()).is_none())
            })
        })
        || f.copies.iter().any(|copy| {
            copy.assignments.iter().any(|assignment| {
                assignment.issue.is_some()
                    || assignment.destination.is_err()
                    || assignment.destination == Ok(local)
                    || assignment
                        .source
                        .map_or(true, |read| read.local() == Some(local))
            }) || copy.steps.iter().any(|step| match *step {
                CopyStep::Save {
                    destination,
                    source,
                } => destination == local || source == local,
                CopyStep::Assign {
                    destination,
                    source,
                    ..
                } => destination == local || source.local() == Some(local),
            })
        })
    {
        return Err(Separation::InterveningOperation(operation).into());
    }
    let input = f
        .operation_uses(target)
        .and_then(Result::ok)
        .and_then(|uses| uses.iter().find(|input| input.site() == usage.site()));
    if !input.is_some_and(|input| {
        let initialization = match f.locals[local.0 as usize].storage_type() {
            StorageType::Family {
                family: crate::p4::Family::Reference,
                ..
            } => input.initialization() == Some(Initialization::Initialized),
            // P4 publishes initialization only for references. Numeric state
            // correspondence uses its existing read/view, not a fabricated flag.
            _ => input.initialization().is_none(),
        };
        initialization
            && matches!(input.read(), Ok(StorageRead::Value { value, local: source, .. })
                if value == produced && source == local)
    }) {
        return Err(Separation::InterveningOperation(operation).into());
    }
    Ok(PreparedArgument {
        operation,
        target: usage.site(),
        local,
    })
}

fn preparation_effects(
    typed: &TypedMethod<'_>,
    f: &StorageFacts,
    allocation: crate::p2::OpId,
    operation: crate::p2::OpId,
) -> Admission<()> {
    let flow = typed.execution();
    let op = &flow.operations()[operation.0 as usize];
    let handlers = protection(typed, operation)?;
    let string = matches!(
        flow.reference_fact(op.pc(), crate::raw::ReferenceSlot::First),
        Some(Ok(crate::p1::CheckedReferenceFact::String { .. }))
    );
    // Every throwing member keeps the same ordered handler chain. Pure
    // preparations have no throw route; snapshots are owned by the group below.
    if flow
        .reference_use_issue(op.pc(), crate::raw::ReferenceSlot::First)
        .is_some()
        || (op.effects().may_throw()
            && (op.exceptional().is_none() || handlers != protection(typed, allocation)?))
    {
        return Err(Separation::InterveningOperation(operation).into());
    }
    if string {
        if !matches!(op.kind(), OpKind::ConstString { .. })
            || !op.effects().may_throw()
            || op.exceptional().is_none()
        {
            return Err(Separation::InterveningOperation(operation).into());
        }
    } else if !matches!(
        f.operation_emission[operation.0 as usize],
        OperationEmission::Ordinary
    ) || matches!(
        op.kind(),
        OpKind::Monitor { .. }
            | OpKind::MoveException
            | OpKind::NewInstance { .. }
            | OpKind::NewArray { .. }
            | OpKind::FilledNewArray { .. }
            | OpKind::FillArrayData { .. }
            | OpKind::UnresolvedInvoke { .. }
            | OpKind::UnresolvedResult { .. }
    ) {
        return Err(Separation::InterveningOperation(operation).into());
    }
    // A preparation must not observe the object whose construction it
    // precedes; that read would still be uninitialized at its own position.
    if typed.facts().initialization().iter().any(|use_| {
        matches!(
            use_.site,
            UseSite::Operation {
                operation: at,
                ..
            } if at == operation
        ) && use_.state == Initialization::BeforeConstructor
            && use_
                .allocation
                .is_some_and(|binding| binding.operation == allocation)
    }) {
        return Err(Separation::InterveningOperation(operation).into());
    }
    Ok(())
}
