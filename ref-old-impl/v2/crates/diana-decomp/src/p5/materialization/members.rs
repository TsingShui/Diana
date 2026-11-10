//! Constructor binding, corridor membership and protection obligations.

use super::candidate::Admission;
use super::*;

pub(super) fn receiver(
    typed: &TypedMethod<'_>,
    constructor: crate::p4::ConstructorFact,
    allocation: crate::p2::OpId,
) -> Admission<()> {
    let init = constructor.operation;
    let receiver = typed
        .facts()
        .initialization()
        .iter()
        .find(|u| matches!(u.site,UseSite::Operation{operation,argument:0} if operation==init));
    let Some(receiver) = receiver else {
        return Err(Separation::NoAllocationBinding.into());
    };
    if receiver.state != Initialization::BeforeConstructor {
        return Err(Separation::Initialization(receiver.state).into());
    }
    if constructor.relation != ConstructorRelation::Allocation {
        return Err(Separation::NonAllocationConstructor.into());
    }
    if constructor
        .allocation
        .is_none_or(|a| a.operation != allocation || a.relation != AllocationRelation::Same)
    {
        return Err(Separation::InstanceNotEstablished.into());
    }
    Ok(())
}

pub(super) fn preparations(
    typed: &TypedMethod<'_>,
    f: &StorageFacts,
    allocation: crate::p2::OpId,
    init: crate::p2::OpId,
    members: &[crate::p2::OpId],
    own_member: &[bool],
) -> Admission<Vec<crate::p2::OpId>> {
    let flow = typed.execution();
    let aliases_object = |operation| aliases_allocation(typed, allocation, operation);
    let mut preparations = Vec::new();
    for &operation in members {
        match f.operation_emission[operation.0 as usize] {
            OperationEmission::Ordinary => {}
            OperationEmission::Construction { group, .. } => {
                let nested = &f.construction_groups()[group.0 as usize];
                if nested
                    .members()
                    .iter()
                    .any(|member| !own_member[member.0 as usize])
                {
                    return Err(Separation::SharedAllocation.into());
                }
                return Err(Separation::InterveningOperation(operation).into());
            }
        }
        if operation == allocation || operation == init || aliases_object(operation) {
            continue;
        }
        let op = &flow.operations()[operation.0 as usize];
        let literal = matches!(op.kind(), OpKind::Const { .. })
            && f.operation_definitions(operation).is_some_and(|defs| {
                defs.iter()
                    .all(|d| matches!(d.materialization(), Materialization::Literal(_)))
            });
        if literal {
            continue;
        }
        // Another operation may prepare one constructor argument. The strict
        // admission below decides whether its single use, order, copy and
        // protection facts preserve the original evaluation order; any
        // rejection keeps the whole construction explicit.
        push(&mut preparations, operation)?;
    }
    Ok(preparations)
}

pub(super) fn compatible(
    typed: &TypedMethod<'_>,
    f: &StorageFacts,
    allocation: crate::p2::OpId,
    init: crate::p2::OpId,
    interior_edges: &[crate::p2::EdgeId],
) -> Admission<()> {
    for plan in &f.copies {
        if matches!(plan.location,CopyLocation::Transfer{site:TransferSite::NormalEdge(e),..}if interior_edges.contains(&e))
            && !plan.assignments.is_empty()
        {
            return Err(Separation::TransferCopy(plan.id).into());
        }
    }
    if protection(typed, allocation)? != protection(typed, init)? {
        return Err(Separation::DifferentProtection {
            allocation,
            initialization: init,
        }
        .into());
    }
    Ok(())
}
