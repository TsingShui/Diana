//! Normalizes supplied state dispatch and resume-input associations. This owns
//! no control layout and supplies the same site/copy matcher for both entries.
use super::facts::*;
use super::*;
use crate::p2::{EdgeRole, NormalEdge};

#[derive(Clone, Copy)]
pub(super) enum Dispatch {
    Switch(NodeId),
    Comparisons(NodeId),
}
#[derive(Clone, Copy)]
pub(super) enum Entry {
    Parameter,
    Field,
}
#[derive(Clone, Copy)]
pub(super) enum ResumeInput {
    Parameter,
    Field(OpId),
}
pub(super) struct Input {
    pub(super) entry: NodeId,
    pub(super) dispatch: Vec<NodeId>,
    pub(super) initial: LinkId,
    pub(super) invalid: LinkId,
    pub(super) states: Vec<(i32, LinkId)>,
    pub(super) label: crate::FieldId,
    pub(super) continuation: LocalId,
    pub(super) result: LocalId,
    pub(super) resume: ResumeInput,
    pub(super) marker: LocalId,
    pub(super) marker_op: OpId,
    pub(super) selector_op: OpId,
}

fn lambda(storage: &StorageMethod<'_>) -> bool {
    let flow = storage.execution();
    let d = flow.declaration();
    units(d.name_units(), b"invokeSuspend")
        && units(
            d.prototype_units(),
            b"(Ljava/lang/Object;)Ljava/lang/Object;",
        )
        && flow.access_flags() & crate::substrate::flags::STATIC == 0
        && matches!(flow.owner_facts().superclass(), crate::p0::Fact::Present(name)
            if units(name.units(), b"Lkotlin/coroutines/jvm/internal/SuspendLambda;"))
}

pub(super) fn entry(storage: &StorageMethod<'_>) -> Option<Entry> {
    let prototype = storage.execution().declaration().prototype_units();
    let suffix = b"Lkotlin/coroutines/Continuation;)Ljava/lang/Object;";
    if prototype.len() >= suffix.len()
        && units(&prototype[prototype.len() - suffix.len()..], suffix)
    {
        Some(Entry::Field)
    } else {
        lambda(storage).then_some(Entry::Parameter)
    }
}

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    dispatch: Dispatch,
    entry_kind: Entry,
) -> Result<Result<Input, &'static str>, RegionError> {
    let flow = storage.execution();
    let mut nodes = Vec::new();
    let mut states = Vec::new();
    let (entry, initial, invalid, selector) = match dispatch {
        Dispatch::Switch(node) => {
            push(&mut nodes, node)?;
            let mut initial = None;
            let mut invalid = None;
            for &link in &g.outgoing[node.0 as usize] {
                let LinkOrigin::Edge(edge) = f.links[link.0 as usize].origin else {
                    return Ok(Err("dispatch_origin"));
                };
                match flow.edges()[edge.0 as usize].role() {
                    EdgeRole::Normal(NormalEdge::SwitchCase { key: 0, .. }) => initial = Some(link),
                    EdgeRole::Normal(NormalEdge::SwitchCase { key, .. }) if key > 0 => {
                        let Ok(state) = i32::try_from(key) else {
                            return Ok(Err("dispatch_state_range"));
                        };
                        push(&mut states, (state, link))?;
                    }
                    EdgeRole::Normal(NormalEdge::SwitchDefault) => invalid = Some(link),
                    _ => return Ok(Err("dispatch_state")),
                }
            }
            let (Some(initial), Some(invalid)) = (initial, invalid) else {
                return Ok(Err("dispatch_roles"));
            };
            let Some([selector]) = storage
                .facts()
                .terminal_uses(f.nodes[node.0 as usize].block)
            else {
                return Ok(Err("dispatch_selector"));
            };
            let Ok(selector) = selector.read() else {
                return Ok(Err("dispatch_selector_gap"));
            };
            (node, initial, invalid, selector)
        }
        Dispatch::Comparisons(entry) => {
            let mut at = entry;
            let mut binding = None;
            let mut initial = None;
            let mut selector = None;
            let invalid;
            loop {
                let node = &f.nodes[at.0 as usize];
                let Some((identity, argument, key, equal)) =
                    super::super::decision_layout::comparison(storage, node.block)
                else {
                    return Ok(Err("dispatch_comparison"));
                };
                if nodes.len() == f.nodes.len()
                    || nodes.contains(&at)
                    || !node.operations.is_empty()
                    || !node.terminal
                    || !terminal_plain(f, node.block)
                    || binding.is_some_and(|b| b != identity)
                    || (nodes.is_empty() && key != 0)
                    || (!nodes.is_empty() && (key <= 0 || states.iter().any(|&(s, _)| s == key)))
                {
                    return Ok(Err("dispatch_comparison_binding_or_phase"));
                }
                binding = Some(identity);
                selector = selector.or_else(|| {
                    storage
                        .facts()
                        .terminal_uses(node.block)
                        .and_then(|us| us.get(argument as usize))
                        .and_then(|u| u.read().ok())
                });
                let mut selected = None;
                let mut remaining = None;
                for &link in &g.outgoing[at.0 as usize] {
                    let LinkOrigin::Edge(edge) = f.links[link.0 as usize].origin else {
                        return Ok(Err("dispatch_comparison_edge"));
                    };
                    let yes = match flow.edges()[edge.0 as usize].role() {
                        EdgeRole::Normal(NormalEdge::ConditionalTrue) => equal,
                        EdgeRole::Normal(NormalEdge::ConditionalFalse) => !equal,
                        _ => return Ok(Err("dispatch_comparison_edge_role")),
                    };
                    if !f.link_plans[link.0 as usize].copies.is_empty() {
                        return Ok(Err("dispatch_copy"));
                    }
                    if yes {
                        selected = Some(link);
                    } else {
                        remaining = Some(link);
                    }
                }
                let (Some(selected), Some(remaining)) = (selected, remaining) else {
                    return Ok(Err("dispatch_comparison_links"));
                };
                if nodes.is_empty() {
                    initial = Some(selected);
                } else {
                    push(&mut states, (key, selected))?;
                }
                push(&mut nodes, at)?;
                at = f.links[remaining.0 as usize].to;
                if super::super::decision_layout::comparison(storage, f.nodes[at.0 as usize].block)
                    .is_some()
                {
                    if g.incoming[at.0 as usize].as_slice() != [remaining] {
                        return Ok(Err("dispatch_comparison_arrival"));
                    }
                } else {
                    invalid = remaining;
                    break;
                }
            }
            let Some(selector) = selector else {
                return Ok(Err("dispatch_selector"));
            };
            (
                entry,
                initial.ok_or(bug("dispatch lost state zero"))?,
                invalid,
                selector,
            )
        }
    };
    if states.is_empty()
        || g.outgoing[entry.0 as usize]
            .iter()
            .any(|l| !f.link_plans[l.0 as usize].copies.is_empty())
    {
        return Ok(Err("dispatch_copy_or_states"));
    }
    let Some(selector_op) = definition(storage, selector) else {
        return Ok(Err("selector_definition"));
    };
    let Some((label, continuation)) = field_get(storage, selector_op)? else {
        return Ok(Err("selector_field"));
    };
    if !field_named(storage, selector_op, b"label", b"I")? {
        return Ok(Err("selector_field_identity"));
    }
    let prefix_end = f.nodes[entry.0 as usize].operations.start;
    let mut marker = None;
    let mut result_op = None;
    for op in flow.operations().iter().take(prefix_end as usize) {
        if field_get(storage, op.id())?.is_some_and(|(_, r)| r == continuation)
            && field_named(storage, op.id(), b"result", b"Ljava/lang/Object;")?
        {
            result_op = Some(op.id());
        }
        for d in storage
            .facts()
            .operation_definitions(op.id())
            .into_iter()
            .flatten()
        {
            if let Materialization::Store(local) = d.materialization()
                && marker_source(storage, local)?
            {
                marker = Some((local, op.id()));
            }
        }
    }
    let Some((marker, marker_op)) = marker else {
        return Ok(Err("marker_definition"));
    };
    let (result, resume) = match entry_kind {
        Entry::Field => {
            let Some(op) = result_op else {
                return Ok(Err("resume_field"));
            };
            let Some(local) = storage
                .facts()
                .operation_definitions(op)
                .and_then(|ds| ds.first())
                .and_then(|d| match d.materialization() {
                    Materialization::Store(l) => Some(l),
                    _ => None,
                })
            else {
                return Ok(Err("resume_field_storage"));
            };
            if !prefix_protocol(
                storage,
                f,
                entry,
                continuation,
                label,
                op,
                selector_op,
                marker,
            )? {
                return Ok(Err("entry_protocol"));
            }
            (local, ResumeInput::Field(op))
        }
        Entry::Parameter => {
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
            let result = storage.facts().locals().iter().find_map(|l| {
                matches!(
                    l.kind(),
                    LocalKind::Parameter {
                        role: EntryRole::Parameter(0),
                        ..
                    }
                )
                .then_some(l.id())
            });
            let Some(result) = result.filter(|_| this == Some(continuation)) else {
                return Ok(Err("resume_parameters"));
            };
            if !matches!(flow.reference_fact(flow.operations()[selector_op.0 as usize].pc(), ReferenceSlot::First), Some(Ok(CheckedReferenceFact::Field { owner_type, .. })) if *owner_type == flow.owner_facts().type_id())
            {
                return Ok(Err("selector_owner"));
            }
            let block = f.nodes[entry.0 as usize].block;
            if entry.0 == 0 || !g.incoming[0].is_empty() {
                return Ok(Err("entry_prefix"));
            }
            for node in &f.nodes[..entry.0 as usize] {
                let [link] = g.outgoing[node.id.0 as usize].as_slice() else {
                    return Ok(Err("entry_prefix_outlet"));
                };
                let target = f.links[link.0 as usize].to;
                if node.terminal
                    || node.block != block
                    || node.id.0.checked_add(1) != Some(target.0)
                    || g.incoming[target.0 as usize].as_slice() != [*link]
                    || !f.link_plans[link.0 as usize].copies.is_empty()
                {
                    return Ok(Err("entry_prefix_arrival_or_copy"));
                }
                for index in node.operations.clone() {
                    if OpId(index) != selector_op
                        && OpId(index) != marker_op
                        && !(matches!(
                            flow.operations()[index as usize].kind(),
                            OpKind::Const { .. }
                        ) && storage
                            .facts()
                            .operation_definitions(OpId(index))
                            .is_some_and(|ds| {
                                !ds.is_empty()
                                    && ds.iter().all(|d| {
                                        matches!(d.materialization(), Materialization::Literal(_))
                                    })
                            }))
                    {
                        return Ok(Err("entry_prefix_work"));
                    }
                }
            }
            (result, ResumeInput::Parameter)
        }
    };
    Ok(Ok(Input {
        entry,
        dispatch: nodes,
        initial,
        invalid,
        states,
        label,
        continuation,
        result,
        resume,
        marker,
        marker_op,
        selector_op,
    }))
}
