//! Private ordinary-continuation facts shared by the owning P6 alternatives.
use super::*;
use crate::p2::FieldAccess;

pub(super) fn invariant_local(
    storage: &StorageMethod<'_>,
    local: LocalId,
    source: Option<OpId>,
    prefix_end: Option<(u32, crate::p2::BlockId)>,
) -> bool {
    for d in storage.facts().definitions() {
        if d.materialization() != Materialization::Store(local)
            || Some(d.operation()) == source
            || prefix_end.is_some_and(|(end, _)| d.operation().0 < end)
        {
            continue;
        }
        if !matches!(
            storage.execution().operations()[d.operation().0 as usize].kind(),
            OpKind::Move { .. }
        ) || !inputs(storage, d.operation()).is_some_and(
            |uses| matches!(uses, [u] if u.read().ok().and_then(StorageRead::local) == Some(local)),
        ) {
            return false;
        }
    }
    for p in storage.facts().copy_plans() {
        if let Some((_, block)) = prefix_end
            && matches!(p.location(), crate::p5::CopyLocation::Transfer { site: TransferSite::NormalEdge(edge), .. } if storage.execution().edges()[edge.0 as usize].from().0 < block.0)
        {
            continue;
        }
        for step in p.steps() {
            if let CopyStep::Assign {
                destination,
                source,
                ..
            } = *step
                && destination == local
                && source.local() != Some(local)
            {
                return false;
            }
        }
    }
    true
}

pub(super) fn same_value(a: StorageRead, b: StorageRead) -> bool {
    matches!((a, b), (StorageRead::Value { local: a, value: av, state: as_, .. }, StorageRead::Value { local: b, value: bv, state: bs, .. }) if a == b && av == bv && as_ == bs)
}

pub(super) fn ending_corridor(
    f: &RegionFacts,
    g: &Graph,
    mut at: NodeId,
) -> Result<Option<Vec<NodeId>>, RegionError> {
    let mut nodes = Vec::new();
    for _ in 0..f.nodes.len() {
        if nodes.contains(&at) {
            return Ok(None);
        }
        push(&mut nodes, at)?;
        match g.outgoing[at.0 as usize].as_slice() {
            [] => return Ok(Some(nodes)),
            [link] => at = f.links[link.0 as usize].to,
            _ => return Ok(None),
        }
    }
    Ok(None)
}

pub(super) fn definition(storage: &StorageMethod<'_>, read: StorageRead) -> Option<OpId> {
    let StorageRead::Value { local, state, .. } = read else {
        return None;
    };
    storage.facts().definitions().iter().find(|d| d.state() == state && matches!(d.materialization(), Materialization::Store(l) | Materialization::Deferred { local: l, .. } if l == local)).map(|d| d.operation())
}

pub(super) fn normalize(storage: &StorageMethod<'_>, mut read: StorageRead) -> StorageRead {
    for _ in 0..storage.execution().operations().len() {
        let Some(op) = definition(storage, read) else {
            break;
        };
        if !matches!(
            storage.execution().operations()[op.0 as usize].kind(),
            OpKind::Move { .. }
        ) {
            break;
        }
        let Some([input]) = inputs(storage, op) else {
            break;
        };
        let Ok(source) = input.read() else {
            break;
        };
        read = source;
    }
    read
}

pub(super) fn field_get(
    storage: &StorageMethod<'_>,
    id: OpId,
) -> Result<Option<(crate::FieldId, LocalId)>, RegionError> {
    let OpKind::Field {
        access: FieldAccess::Get,
        static_field: false,
        field,
        ..
    } = storage.execution().operations()[id.0 as usize].kind()
    else {
        return Ok(None);
    };
    let Some([receiver]) = inputs(storage, id) else {
        return Ok(None);
    };
    Ok(receiver
        .read()
        .ok()
        .and_then(StorageRead::local)
        .map(|local| (*field, local)))
}

pub(super) fn field_put(
    storage: &StorageMethod<'_>,
    id: OpId,
) -> Result<Option<(crate::FieldId, LocalId, StorageRead)>, RegionError> {
    let OpKind::Field {
        access: FieldAccess::Put,
        static_field: false,
        field,
        ..
    } = storage.execution().operations()[id.0 as usize].kind()
    else {
        return Ok(None);
    };
    let Some([value, receiver]) = inputs(storage, id) else {
        return Ok(None);
    };
    Ok(value
        .read()
        .ok()
        .zip(receiver.read().ok().and_then(StorageRead::local))
        .map(|(value, receiver)| (*field, receiver, value)))
}

pub(super) fn field_named(
    storage: &StorageMethod<'_>,
    id: OpId,
    expected: &[u8],
    type_: &[u8],
) -> Result<bool, RegionError> {
    let op = &storage.execution().operations()[id.0 as usize];
    let Some(Ok(CheckedReferenceFact::Field {
        name, field_type, ..
    })) = storage
        .execution()
        .reference_fact(op.pc(), ReferenceSlot::First)
    else {
        return Ok(false);
    };
    Ok(ascii(*name, expected)? && ascii(*field_type, type_)?)
}

pub(super) fn assignments(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    link: LinkId,
) -> Result<Option<Vec<(LocalId, StorageRead)>>, RegionError> {
    let [copy] = f.link_plans[link.0 as usize].copies.as_slice() else {
        return Ok(f.link_plans[link.0 as usize]
            .copies
            .is_empty()
            .then(Vec::new));
    };
    let Some(plan) = storage.facts().copy_plans().get(copy.0 as usize) else {
        return Ok(None);
    };
    if !plan.is_complete() {
        return Ok(None);
    }
    let mut result = Vec::new();
    for step in plan.steps() {
        let CopyStep::Assign {
            destination,
            source,
            ..
        } = *step
        else {
            return Ok(None);
        };
        push(&mut result, (destination, source))?;
    }
    Ok(Some(result))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prefix_protocol(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    dispatch: NodeId,
    continuation: LocalId,
    label: crate::FieldId,
    result: OpId,
    selector: OpId,
    marker: LocalId,
) -> Result<bool, RegionError> {
    let flow = storage.execution();
    let Some(Ok(CheckedReferenceFact::Field { owner_type, .. })) = flow.reference_fact(
        flow.operations()[selector.0 as usize].pc(),
        ReferenceSlot::First,
    ) else {
        return Ok(false);
    };
    let owner_type = *owner_type;
    let last_parameter = storage
        .facts()
        .locals()
        .iter()
        .filter_map(|l| match l.kind() {
            LocalKind::Parameter {
                role: EntryRole::Parameter(index),
                ..
            } => Some((index, l.id())),
            _ => None,
        })
        .max_by_key(|&(index, _)| index)
        .map(|(_, id)| id);
    let this = storage.facts().locals().iter().find_map(|l| {
        matches!(
            l.kind(),
            LocalKind::Parameter {
                role: EntryRole::This,
                ..
            }
        )
        .then_some(l.id())
    });
    let mut allocations = 0;
    let mut initializations = 0;
    for node in &f.nodes[..dispatch.0 as usize] {
        for index in node.operations.clone() {
            let id = OpId(index);
            let op = &flow.operations()[index as usize];
            match op.kind() {
                OpKind::Const { .. } | OpKind::Move { .. } => {},
                OpKind::InstanceOf { type_id } | OpKind::Cast { type_id } if *type_id == owner_type => {},
                OpKind::NewInstance { type_id } if *type_id == owner_type => { allocations += 1; },
                OpKind::Binary { op: crate::p2::BinaryOp::AndInt | crate::p2::BinaryOp::SubInt, .. } => {
                    if !inputs(storage, id).is_some_and(|uses| uses.iter().any(|u| matches!(u.read(), Ok(StorageRead::Literal { bits, .. }) if bits.value == 0x8000_0000))) { return Ok(false); }
                }
                OpKind::Field { field, access, .. } if id == result || id == selector || *field == label => {
                    if matches!(access, FieldAccess::Put) && field_put(storage, id)?.is_none() { return Ok(false); }
                }
                OpKind::Invoke { kind: InvokeKind::Direct, .. } => {
                    let Some(Ok(CheckedReferenceFact::Method { name, owner_type: constructor_owner, .. })) = flow.reference_fact(op.pc(), ReferenceSlot::First) else { return Ok(false); };
                    let Some(uses) = inputs(storage, id) else { return Ok(false); };
                    if !ascii(*name, b"<init>")? || *constructor_owner != owner_type || uses.last().and_then(|u| u.read().ok()).and_then(StorageRead::local) != last_parameter || (uses.len() != 2 && !(uses.len() == 3 && uses[1].read().ok().and_then(StorageRead::local) == this)) || !storage.facts().construction_groups().iter().any(|g| g.initialization() == id && matches!(flow.operations()[g.allocation().0 as usize].kind(), OpKind::NewInstance { type_id } if *type_id == owner_type)) { return Ok(false); }
                    initializations += 1;
                }
                OpKind::Invoke { kind: InvokeKind::Static, .. } if storage.facts().operation_definitions(id).is_some_and(|ds| ds.iter().any(|d| d.materialization() == Materialization::Store(marker))) => {},
                _ => return Ok(false),
            }
        }
    }
    // The receiver is an available original storage identity, never a guessed
    // superclass/callee declaration or an external symbol lookup.
    Ok(allocations == 1
        && initializations == 1
        && storage
            .facts()
            .locals()
            .iter()
            .any(|l| l.id() == continuation))
}
