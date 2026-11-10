//! Original invalid-state construction, retained as an ABI attachment.
use super::super::build::filled;
use super::*;

pub(super) fn invalid_nodes(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    graph: &Graph,
    nodes: &[NodeId],
    entry: Option<NodeId>,
    entrance: LinkId,
) -> Result<Result<Vec<OpId>, ()>, RegionError> {
    let flow = storage.execution();
    let Some(mut node) = entry else {
        return Ok(Err(()));
    };
    if graph.incoming[node.0 as usize].as_slice() != [entrance] {
        return Ok(Err(()));
    }
    let mut operations = Vec::new();
    let mut terminal = None;
    let mut in_region = filled(f.nodes.len(), false)?;
    for &inside in nodes {
        in_region[inside.0 as usize] = true;
    }
    for _ in 0..nodes.len() {
        if !in_region[node.0 as usize] {
            return Ok(Err(()));
        }
        for index in f.nodes[node.0 as usize].operations.clone() {
            let op = OpId(index);
            if !plain_operation(storage, f, op) {
                return Ok(Err(()));
            }
            push(&mut operations, op)?;
        }
        if f.nodes[node.0 as usize].terminal {
            terminal = Some(f.nodes[node.0 as usize].block);
            if !graph.outgoing[node.0 as usize].is_empty() {
                return Ok(Err(()));
            }
            break;
        }
        let [link] = graph.outgoing[node.0 as usize].as_slice() else {
            return Ok(Err(()));
        };
        let target = f.links[link.0 as usize].to;
        if !f.link_plans[link.0 as usize].copies.is_empty()
            || graph.incoming[target.0 as usize].as_slice() != [*link]
        {
            return Ok(Err(()));
        }
        node = target;
    }
    let Some(terminal) = terminal else {
        return Ok(Err(()));
    };
    if !matches!(
        flow.blocks()[terminal.0 as usize].terminal(),
        Some(Terminal::Throw { .. })
    ) {
        return Ok(Err(()));
    }
    if f.throws
        .iter()
        .filter(|site| site.site == crate::p5::ThrowSite::Terminal(terminal))
        .any(|site| {
            !site.snapshots.is_empty()
                || f.chains[site.chain.0 as usize]
                    .clauses
                    .iter()
                    .any(|c| c.target != CatchTarget::Uncaught)
        })
    {
        return Ok(Err(()));
    }
    let [allocation, message, init] = operations.as_slice() else {
        return Ok(Err(()));
    };
    let (
        OpKind::NewInstance { type_id },
        OpKind::ConstString { .. },
        OpKind::Invoke {
            kind: InvokeKind::Direct,
            result: None,
            ..
        },
    ) = (
        flow.operations()[allocation.0 as usize].kind(),
        flow.operations()[message.0 as usize].kind(),
        flow.operations()[init.0 as usize].kind(),
    )
    else {
        return Ok(Err(()));
    };
    let Some(Ok(CheckedReferenceFact::Type { descriptor, .. })) = flow.reference_fact(
        flow.operations()[allocation.0 as usize].pc(),
        ReferenceSlot::First,
    ) else {
        return Ok(Err(()));
    };
    let Some(Ok(CheckedReferenceFact::Method {
        owner_type,
        name,
        prototype,
        ..
    })) = flow.reference_fact(
        flow.operations()[init.0 as usize].pc(),
        ReferenceSlot::First,
    )
    else {
        return Ok(Err(()));
    };
    if *owner_type != *type_id
        || !ascii(*descriptor, b"Ljava/lang/IllegalStateException;")?
        || !ascii(*name, b"<init>")?
        || prototype.parameters().len() != 1
        || !ascii(
            prototype.parameters()[0].descriptor(),
            b"Ljava/lang/String;",
        )?
        || !ascii(prototype.return_type(), b"V")?
    {
        return Ok(Err(()));
    }
    let Some(group) = storage
        .facts()
        .construction_groups()
        .iter()
        .find(|group| group.allocation() == *allocation && group.initialization() == *init)
    else {
        return Ok(Err(()));
    };
    let Some([returned]) = storage.facts().terminal_uses(terminal) else {
        return Ok(Err(()));
    };
    if group.publications().len() != 1
        || returned.read().ok().and_then(StorageRead::local) != Some(group.publications()[0].local)
    {
        return Ok(Err(()));
    }
    let Some([_, argument]) = inputs(storage, *init) else {
        return Ok(Err(()));
    };
    let Some(message_definition) = storage
        .facts()
        .operation_definitions(*message)
        .and_then(|defs| defs.first())
    else {
        return Ok(Err(()));
    };
    if !argument
        .read()
        .is_ok_and(|read| same_definition(read, message_definition))
    {
        return Ok(Err(()));
    }
    if f.copies.iter().any(|copy|matches!(copy.location,crate::p5::CopyLocation::BeforeThrow(crate::p5::ThrowSite::Terminal(at))|crate::p5::CopyLocation::Transfer{site:TransferSite::BeforeTerminal(at),..} if at==terminal)){return Ok(Err(()))}
    // Preserve the original exception/message/initialization operations as an
    // explicit ABI-only attachment, including their exact original order.
    Ok(Ok(operations))
}
