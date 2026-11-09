//! Selects a closed normal-run loop over the already built P6 node graph.
//! The shared SCC product nominates the cycle; no graph/dominance pass is rerun.
use super::*;
use crate::p2::{EdgeRole, FieldAccess, NormalEdge};

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
    s: &super::super::dominance::Structure,
) -> Result<(), RegionError> {
    let prototype = storage.execution().declaration().prototype_units();
    let suffix = b"Lkotlin/coroutines/Continuation;)Ljava/lang/Object;";
    if prototype.len() < suffix.len()
        || !units(&prototype[prototype.len() - suffix.len()..], suffix)
        || storage.execution().access_flags() & crate::substrate::flags::SYNCHRONIZED != 0
    {
        return Ok(());
    }
    for index in 0..f.nodes.len() {
        let node = NodeId(super::super::build::id(index)?);
        if !matches!(
            storage.execution().blocks()[f.nodes[index].block.0 as usize].terminal(),
            Some(Terminal::Switch { .. })
        ) || !f.nodes[index].terminal
        {
            continue;
        }
        match candidate(storage, f, g, s, node)? {
            Ok(plan) => {
                f.coroutine_loop = Some(plan);
                break;
            }
            Err(reason) => reject(storage, RegionId(index as u32), reason),
        }
    }
    Ok(())
}

fn candidate(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    s: &super::super::dominance::Structure,
    dispatch: NodeId,
) -> Result<Result<CoroutineLoopPlan, &'static str>, RegionError> {
    macro_rules! need {
        ($condition:expr, $reason:literal) => {
            if !$condition {
                return Ok(Err($reason));
            }
        };
    }
    let flow = storage.execution();
    need!(
        g.outgoing[dispatch.0 as usize].len() == 3,
        "loop_requires_two_states_and_default"
    );
    let mut initial_link = None;
    let mut resume_link = None;
    let mut state = None;
    let mut invalid_link = None;
    for &link in &g.outgoing[dispatch.0 as usize] {
        let LinkOrigin::Edge(edge) = f.links[link.0 as usize].origin else {
            return Ok(Err("loop_dispatch_origin"));
        };
        match flow.edges()[edge.0 as usize].role() {
            EdgeRole::Normal(NormalEdge::SwitchCase { key: 0, .. }) => initial_link = Some(link),
            EdgeRole::Normal(NormalEdge::SwitchCase { key, .. }) if key > 0 => {
                let Ok(value) = i32::try_from(key) else {
                    return Ok(Err("loop_state_not_integral"));
                };
                resume_link = Some(link);
                state = Some(value);
            }
            EdgeRole::Normal(NormalEdge::SwitchDefault) => invalid_link = Some(link),
            _ => return Ok(Err("loop_dispatch_states")),
        }
        need!(
            f.link_plans[link.0 as usize].copies.is_empty(),
            "loop_dispatch_copies"
        );
    }
    let (Some(initial_link), Some(resume_link), Some(invalid_link)) =
        (initial_link, resume_link, invalid_link)
    else {
        return Ok(Err("loop_dispatch_states"));
    };
    let Some(state) = state else {
        return Ok(Err("loop_dispatch_states"));
    };
    let Some([selector]) = storage
        .facts()
        .terminal_uses(f.nodes[dispatch.0 as usize].block)
    else {
        return Ok(Err("loop_selector"));
    };
    let Some(selector_op) = selector.read().ok().and_then(|r| definition(storage, r)) else {
        return Ok(Err("loop_selector_definition"));
    };
    let Some((label, continuation)) = field_get(storage, selector_op)? else {
        return Ok(Err("loop_label_read"));
    };
    need!(
        field_named(storage, selector_op, b"label", b"I")?,
        "loop_label_identity"
    );

    // The first cyclic node reached by the original initial corridor is the
    // only head nomination. A cycle is not assumed to be a natural loop.
    let mut initial = Vec::new();
    let mut normal_links = Vec::new();
    let mut at = f.links[initial_link.0 as usize].to;
    for _ in 0..f.nodes.len() {
        if s.cyclic[s.component[at.0 as usize]] {
            break;
        }
        need!(!initial.contains(&at), "loop_initial_cycle");
        push(&mut initial, at)?;
        let [link] = g.outgoing[at.0 as usize].as_slice() else {
            return Ok(Err("loop_initial_not_straight"));
        };
        push(&mut normal_links, *link)?;
        at = f.links[link.0 as usize].to;
    }
    let header = at;
    let component = s.component[header.0 as usize];
    need!(
        s.cyclic[component]
            && f.nodes[header.0 as usize].terminal
            && f.nodes[header.0 as usize].operations.is_empty(),
        "loop_head_not_terminal"
    );
    need!(
        matches!(
            flow.blocks()[f.nodes[header.0 as usize].block.0 as usize].terminal(),
            Some(Terminal::Branch { .. })
        ),
        "loop_head_not_branch"
    );
    let mut repeat = None;
    let mut exit = None;
    for &link in &g.outgoing[header.0 as usize] {
        if s.component[f.links[link.0 as usize].to.0 as usize] == component {
            repeat = Some(link);
        } else {
            exit = Some(link);
        }
    }
    let (Some(repeat), Some(exit)) = (repeat, exit) else {
        return Ok(Err("loop_head_exits"));
    };
    need!(g.outgoing[header.0 as usize].len() == 2, "loop_head_exits");
    need!(
        f.link_plans[repeat.0 as usize].copies.is_empty()
            && f.link_plans[exit.0 as usize].copies.is_empty(),
        "loop_predicate_edge_copies"
    );

    let mut body = Vec::new();
    let mut guard = None;
    let mut suspended = None;
    let mut normal = None;
    at = f.links[repeat.0 as usize].to;
    for _ in 0..f.nodes.len() {
        if at == header {
            break;
        }
        need!(
            s.component[at.0 as usize] == component && !body.contains(&at),
            "loop_body_not_one_cycle"
        );
        push(&mut body, at)?;
        let outgoing = &g.outgoing[at.0 as usize];
        let link = match outgoing.as_slice() {
            [link] => *link,
            [a, b] if guard.is_none() => {
                let (inside, outside) =
                    if s.component[f.links[a.0 as usize].to.0 as usize] == component {
                        (*a, *b)
                    } else {
                        (*b, *a)
                    };
                need!(
                    s.component[f.links[inside.0 as usize].to.0 as usize] == component
                        && s.component[f.links[outside.0 as usize].to.0 as usize] != component,
                    "loop_extra_branch"
                );
                guard = Some(at);
                suspended = Some(outside);
                normal = Some(inside);
                inside
            }
            _ => return Ok(Err("loop_body_branching_unavailable")),
        };
        push(&mut normal_links, link)?;
        at = f.links[link.0 as usize].to;
    }
    need!(
        at == header && body.len() + 1 == s.component.iter().filter(|&&c| c == component).count(),
        "loop_cycle_coverage"
    );
    let (Some(guard), Some(suspended), Some(_normal)) = (guard, suspended, normal) else {
        return Ok(Err("loop_suspension_guard_unavailable"));
    };
    let guard_block = f.nodes[guard.0 as usize].block;
    let Some(Terminal::Branch { op, .. }) = flow.blocks()[guard_block.0 as usize].terminal() else {
        return Ok(Err("loop_marker_comparison"));
    };
    need!(
        matches!(op, BranchOp::Eq | BranchOp::Ne),
        "loop_marker_comparison"
    );
    let LinkOrigin::Edge(marker_edge) = f.links[suspended.0 as usize].origin else {
        return Ok(Err("loop_marker_edge"));
    };
    need!(
        matches!(
            (op, flow.edges()[marker_edge.0 as usize].role()),
            (BranchOp::Eq, EdgeRole::Normal(NormalEdge::ConditionalTrue))
                | (BranchOp::Ne, EdgeRole::Normal(NormalEdge::ConditionalFalse))
        ),
        "loop_marker_polarity"
    );
    let Some([left, right]) = storage.facts().terminal_uses(guard_block) else {
        return Ok(Err("loop_marker_operands"));
    };
    let (Some(left), Some(right)) = (left.read().ok(), right.read().ok()) else {
        return Ok(Err("loop_marker_operand_gap"));
    };
    let (call_result, marker_read) = if let Some(local) = right.local()
        && marker_source(storage, local)?
    {
        (left, right)
    } else if let Some(local) = left.local()
        && marker_source(storage, local)?
    {
        (right, left)
    } else {
        return Ok(Err("loop_marker_binding"));
    };
    need!(
        matches!(
            call_result,
            StorageRead::Value {
                view: Interpretation::Direct(Family::Reference),
                ..
            }
        ) && matches!(
            marker_read,
            StorageRead::Value {
                view: Interpretation::Direct(Family::Reference),
                ..
            }
        ),
        "loop_marker_interpretation"
    );
    let Some(call) = definition(storage, call_result) else {
        return Ok(Err("loop_call_definition"));
    };
    let Some(marker) = marker_read.local() else {
        return Ok(Err("loop_marker_definition"));
    };
    need!(marker_source(storage, marker)?, "loop_marker_binding");
    need!(
        suspending_signature(storage, call, continuation)?,
        "loop_call_binding"
    );
    need!(
        g.incoming[guard.0 as usize].len() == 1
            && g.op_node[call.0 as usize]
                == f.links[g.incoming[guard.0 as usize][0].0 as usize].from,
        "loop_call_guard_adjacency"
    );
    let returned = f.links[suspended.0 as usize].to;
    need!(
        f.nodes[returned.0 as usize].operations.is_empty()
            && f.nodes[returned.0 as usize].terminal
            && g.outgoing[returned.0 as usize].is_empty()
            && g.incoming[returned.0 as usize].as_slice() == [suspended],
        "loop_marker_return_shape"
    );
    let Some([returned_value]) = storage
        .facts()
        .terminal_uses(f.nodes[returned.0 as usize].block)
    else {
        return Ok(Err("loop_marker_return"));
    };
    need!(
        matches!(
            flow.blocks()[f.nodes[returned.0 as usize].block.0 as usize].terminal(),
            Some(Terminal::Return { .. })
        ) && same_read(returned_value.read().ok(), Some(marker_read)),
        "loop_marker_return"
    );
    need!(
        f.link_plans[suspended.0 as usize].copies.is_empty(),
        "loop_suspended_copy"
    );
    let Some(state_store) = call.0.checked_sub(1).map(OpId) else {
        return Ok(Err("loop_state_store"));
    };
    need!(
        super::state_store(storage, state_store, label, continuation, state)?,
        "loop_state_store"
    );

    let mut tail = Vec::new();
    at = f.links[exit.0 as usize].to;
    for _ in 0..f.nodes.len() {
        need!(
            s.component[at.0 as usize] != component && !tail.contains(&at),
            "loop_exit_cycle"
        );
        push(&mut tail, at)?;
        if g.outgoing[at.0 as usize].is_empty() {
            break;
        }
        let [link] = g.outgoing[at.0 as usize].as_slice() else {
            return Ok(Err("loop_exit_not_straight"));
        };
        push(&mut normal_links, *link)?;
        at = f.links[link.0 as usize].to;
    }
    need!(
        matches!(
            flow.blocks()[f.nodes[at.0 as usize].block.0 as usize].terminal(),
            Some(Terminal::Return { .. })
        ),
        "loop_exit_not_return"
    );

    let mut resume = Vec::new();
    at = f.links[resume_link.0 as usize].to;
    let merge;
    let resumed_edge;
    loop {
        need!(
            resume.len() < f.nodes.len()
                && !resume.contains(&at)
                && s.component[at.0 as usize] != component,
            "loop_resume_not_private_corridor"
        );
        push(&mut resume, at)?;
        let [link] = g.outgoing[at.0 as usize].as_slice() else {
            return Ok(Err("loop_resume_not_straight"));
        };
        at = f.links[link.0 as usize].to;
        if s.component[at.0 as usize] == component {
            merge = at;
            resumed_edge = *link;
            break;
        }
    }
    need!(body.contains(&merge), "loop_resume_merge");
    let Some(&normal_merge_edge) = normal_links
        .iter()
        .find(|&&l| f.links[l.0 as usize].to == merge && f.links[l.0 as usize].from != header)
    else {
        return Ok(Err("loop_normal_merge"));
    };
    need!(
        g.incoming[merge.0 as usize].as_slice().len() == 2
            && g.incoming[merge.0 as usize].contains(&resumed_edge),
        "loop_merge_ingress"
    );

    let Some(&entry) = initial.first() else {
        return Ok(Err("loop_initial_check"));
    };
    let Some(entry_check) = f.nodes[entry.0 as usize]
        .operations
        .clone()
        .next()
        .map(OpId)
    else {
        return Ok(Err("loop_initial_check"));
    };
    let Some([result_use]) = inputs(storage, entry_check) else {
        return Ok(Err("loop_initial_result"));
    };
    let Some(result_read) = result_use.read().ok() else {
        return Ok(Err("loop_initial_result"));
    };
    let Some(result_local) = result_read.local() else {
        return Ok(Err("loop_initial_result"));
    };
    let Some(result_op) = definition(storage, result_read) else {
        return Ok(Err("loop_result_definition"));
    };
    need!(
        field_get(storage, result_op)?.is_some_and(|(_, receiver)| receiver == continuation)
            && field_named(storage, result_op, b"result", b"Ljava/lang/Object;")?
            && failure_check(storage, entry_check, result_local)?,
        "loop_result_protocol"
    );
    let marker_op = storage
        .facts()
        .definitions()
        .iter()
        .find(|d| d.materialization() == Materialization::Store(marker))
        .map(|d| d.operation())
        .ok_or(bug("matched marker lost its definition"))?;
    need!(
        invariant_local(storage, marker, Some(marker_op), None)
            && invariant_local(storage, result_local, Some(result_op), None)
            && invariant_local(
                storage,
                continuation,
                None,
                Some((
                    f.nodes[dispatch.0 as usize].operations.start,
                    f.nodes[dispatch.0 as usize].block
                ))
            ),
        "loop_protocol_storage_reassigned"
    );
    let mut protocol_operations = Vec::new();
    push(&mut protocol_operations, entry_check)?;
    push(&mut protocol_operations, state_store)?;
    let mut saved = Vec::new();
    for &node in &body {
        if node == g.op_node[call.0 as usize] {
            break;
        }
        for index in f.nodes[node.0 as usize].operations.clone() {
            let id = OpId(index);
            if id == state_store {
                continue;
            }
            if let Some((field, receiver, value)) = field_put(storage, id)?
                && receiver == continuation
            {
                need!(
                    field != label && !saved.iter().any(|&(f, _, _)| f == field),
                    "loop_spill_written_twice"
                );
                push(&mut saved, (field, id, value))?;
                push(&mut protocol_operations, id)?;
            }
        }
    }
    let mut restored = Vec::new();
    let mut checked = false;
    for &node in &resume {
        for index in f.nodes[node.0 as usize].operations.clone() {
            let id = OpId(index);
            if let Some((field, receiver)) = field_get(storage, id)? {
                need!(
                    !checked
                        && receiver == continuation
                        && saved.iter().any(|&(f, _, _)| f == field),
                    "loop_resume_foreign_field"
                );
                push(&mut restored, (field, id))?;
            } else if failure_check(storage, id, result_local)? {
                need!(!checked, "loop_resume_duplicate_check");
                checked = true;
            } else {
                need!(
                    matches!(
                        flow.operations()[index as usize].kind(),
                        OpKind::Move { .. }
                    ),
                    "loop_resume_nonprotocol_effect"
                );
            }
        }
    }
    need!(checked, "loop_resume_check_unavailable");
    let Some(normal_assignments) = assignments(storage, f, normal_merge_edge)? else {
        return Ok(Err("loop_normal_copy_gap"));
    };
    let Some(resume_assignments) = assignments(storage, f, resumed_edge)? else {
        return Ok(Err("loop_resume_copy_gap"));
    };
    let mut spills = Vec::new();
    let mut result_matched = false;
    for &(destination, value) in &resume_assignments {
        let normalized = normalize(storage, value);
        let expected = if normalized.local() == Some(result_local) {
            result_matched = true;
            normalize(storage, call_result)
        } else {
            let Some(get) = definition(storage, normalized) else {
                return Ok(Err("loop_resume_value_not_saved"));
            };
            let Some(&(field, _)) = restored.iter().find(|&&(_, op)| op == get) else {
                return Ok(Err("loop_resume_value_not_spill"));
            };
            let Some(&(_, put, saved_value)) = saved.iter().find(|&&(f, _, _)| f == field) else {
                return Ok(Err("loop_spill_pair"));
            };
            push(&mut spills, (put, get))?;
            normalize(storage, saved_value)
        };
        let actual = normal_assignments
            .iter()
            .find(|&&(d, _)| d == destination)
            .map(|&(_, r)| normalize(storage, r));
        // Field-use type views can differ from a copy's integral/reference view.
        // The exact available value/state and storage identity must still agree.
        need!(
            actual.is_some_and(|a| same_value(a, expected))
                || (actual.is_none()
                    && expected.local() == Some(destination)
                    && spills.last().is_some_and(|&(put, _)| unchanged_after(
                        storage,
                        f,
                        &body,
                        put,
                        merge,
                        destination
                    ))),
            "loop_resume_copy_disagrees_with_normal_run"
        );
    }
    need!(result_matched, "loop_result_copy_unavailable");
    need!(
        normal_assignments
            .iter()
            .all(|&(destination, _)| resume_assignments.iter().any(|&(d, _)| d == destination)),
        "loop_normal_copy_without_resume_value"
    );
    // Initial/core/tail have no other external normal arrivals. The resume
    // corridor alone enters the established merge, never the loop head.
    let mut source = super::super::build::filled(f.nodes.len(), false)?;
    for &node in initial
        .iter()
        .chain(&body)
        .chain(&tail)
        .chain(std::iter::once(&header))
    {
        source[node.0 as usize] = true;
    }
    for link in &f.links {
        if source[link.to.0 as usize] && !source[link.from.0 as usize] {
            need!(
                link.id == initial_link || link.id == resumed_edge,
                "loop_foreign_normal_entry"
            );
        }
    }
    need!(
        g.exceptions
            .iter()
            .flatten()
            .all(|&e| !Graph::exception_possible(f, e)
                || Graph::exception_target(f, storage, e).is_none()),
        "loop_executable_handler_unavailable"
    );
    need!(
        f.nodes.iter().all(|n| f.lock_entries.empty_at(n.id)),
        "loop_lock_boundary"
    );
    for op in flow.operations() {
        need!(
            plain_operation(storage, f, op.id()) && inputs(storage, op.id()).is_some(),
            "loop_phase_or_local_gap"
        );
    }
    for &node in initial
        .iter()
        .chain(&body)
        .chain(&tail)
        .chain(std::iter::once(&header))
    {
        if f.nodes[node.0 as usize].terminal {
            need!(
                terminal_plain(f, f.nodes[node.0 as usize].block),
                "loop_terminal_phase"
            );
        }
    }
    let invalid_start = f.links[invalid_link.0 as usize].to;
    let Some(invalid) = ending_corridor(f, g, invalid_start)? else {
        return Ok(Err("loop_invalid_state_not_straight"));
    };
    need!(
        super::body::invalid_nodes(storage, f, g, &invalid, Some(invalid_start), invalid_link)?
            .is_ok(),
        "loop_invalid_state_not_protocol"
    );
    let mut abi = Vec::new();
    for (index, &in_source) in source.iter().enumerate() {
        let node = NodeId(super::super::build::id(index)?);
        if !in_source || node == guard {
            push(&mut abi, node)?;
        }
    }
    // The prefix has only continuation plumbing. Other user effects remain
    // explicit unless their owning producer establishes a separate protocol.
    need!(
        abi.iter().all(|&n| n.0 < dispatch.0
            || n == dispatch
            || resume.contains(&n)
            || invalid.contains(&n)
            || n == guard
            || n == returned),
        "loop_unclassified_original_content"
    );
    need!(
        prefix_protocol(
            storage,
            f,
            dispatch,
            continuation,
            label,
            result_op,
            selector_op,
            marker
        )?,
        "loop_entry_prefix_not_protocol"
    );
    body.retain(|&node| node != guard);
    need!(
        closed_prefix(
            storage,
            f,
            &initial,
            &body,
            &tail,
            &normal_links,
            &protocol_operations,
            dispatch,
            call,
            continuation,
            marker,
            result_local
        ),
        "loop_protocol_value_escapes"
    );
    let mut abi_entries = Vec::new();
    for link in &f.links {
        if abi.contains(&link.from)
            && source[link.to.0 as usize]
            && !abi.contains(&link.to)
            && !abi_entries.contains(&link.to)
        {
            push(&mut abi_entries, link.to)?;
        }
    }
    Ok(Ok(CoroutineLoopPlan {
        initial,
        body,
        tail,
        abi,
        abi_entries,
        condition: LoopCondition {
            node: header,
            repeat,
            exit,
        },
        call,
        continuation_argument: u32::try_from(
            inputs(storage, call)
                .ok_or(bug("matched call lost inputs"))?
                .len()
                - 1,
        )
        .map_err(|_| crate::RequestError::PlatformCapacity)?,
        state,
        protocol_operations,
        normal_links,
        spills,
    }))
}

fn invariant_local(
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

fn same_value(a: StorageRead, b: StorageRead) -> bool {
    matches!((a, b), (StorageRead::Value { local: a, value: av, state: as_, .. }, StorageRead::Value { local: b, value: bv, state: bs, .. }) if a == b && av == bv && as_ == bs)
}
fn unchanged_after(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    body: &[NodeId],
    put: OpId,
    merge: NodeId,
    local: LocalId,
) -> bool {
    let mut after = false;
    for &node in body {
        if node == merge {
            return after;
        }
        for index in f.nodes[node.0 as usize].operations.clone() {
            let op = OpId(index);
            if op == put {
                after = true;
                continue;
            }
            if after
                && storage
                    .facts()
                    .operation_definitions(op)
                    .is_none_or(|defs| {
                        defs.iter()
                            .any(|d| d.materialization() == Materialization::Store(local))
                    })
            {
                return false;
            }
        }
    }
    false
}
fn ending_corridor(
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

#[allow(clippy::too_many_arguments)]
fn closed_prefix(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    initial: &[NodeId],
    body: &[NodeId],
    tail: &[NodeId],
    links: &[LinkId],
    protocol: &[OpId],
    dispatch: NodeId,
    call: OpId,
    continuation: LocalId,
    marker: LocalId,
    result: LocalId,
) -> bool {
    let private = |local| {
        local == continuation || storage.facts().definitions().iter().any(|d| d.operation().0 < f.nodes[dispatch.0 as usize].operations.start && matches!(d.materialization(), Materialization::Store(l) | Materialization::Deferred { local: l, .. } if l == local))
    };
    for &node in initial.iter().chain(body).chain(tail) {
        for index in f.nodes[node.0 as usize].operations.clone() {
            let id = OpId(index);
            if protocol.contains(&id) {
                continue;
            }
            let Some(uses) = inputs(storage, id) else {
                return false;
            };
            for (argument, use_) in uses.iter().enumerate() {
                let Ok(read) = use_.read() else {
                    return false;
                };
                if let Some(local) = read.local()
                    && private(local)
                {
                    let self_move = matches!(
                        storage.execution().operations()[index as usize].kind(),
                        OpKind::Move { .. }
                    ) && (local == marker || local == result)
                        && storage.facts().operation_definitions(id).is_some_and(|ds| {
                            ds.iter()
                                .all(|d| d.materialization() == Materialization::Store(local))
                        });
                    if !(self_move
                        || (id == call && argument + 1 == uses.len() && local == continuation))
                    {
                        return false;
                    }
                }
            }
        }
        if f.nodes[node.0 as usize].terminal
            && storage
                .facts()
                .terminal_uses(f.nodes[node.0 as usize].block)
                .is_none_or(|uses| {
                    uses.iter().any(|u| {
                        !u.read()
                            .is_ok_and(|r| r.local().is_none_or(|l| !private(l)))
                    })
                })
        {
            return false;
        }
    }
    for &link in links {
        for &copy in &f.link_plans[link.0 as usize].copies {
            for step in storage.facts().copy_plans()[copy.0 as usize].steps() {
                if let CopyStep::Assign {
                    source,
                    destination,
                    ..
                } = *step
                    && (private(destination) || source.local().is_some_and(private))
                {
                    return false;
                }
            }
        }
    }
    true
}

fn definition(storage: &StorageMethod<'_>, read: StorageRead) -> Option<OpId> {
    let StorageRead::Value { local, state, .. } = read else {
        return None;
    };
    storage.facts().definitions().iter().find(|d| d.state() == state && matches!(d.materialization(), Materialization::Store(l) | Materialization::Deferred { local: l, .. } if l == local)).map(|d| d.operation())
}
fn normalize(storage: &StorageMethod<'_>, mut read: StorageRead) -> StorageRead {
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
fn field_get(
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
fn field_put(
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
fn field_named(
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
fn assignments(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    link: LinkId,
) -> Result<Option<Vec<(LocalId, StorageRead)>>, RegionError> {
    let [copy] = f.link_plans[link.0 as usize].copies.as_slice() else {
        return Ok(None);
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
fn prefix_protocol(
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
