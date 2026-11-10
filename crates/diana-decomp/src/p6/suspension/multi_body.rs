//! Closed acyclic ordinary continuation bodies over existing P5/P6 products.
use super::facts::*;
use super::*;
use crate::p2::{EdgeRole, NormalEdge, ValueKind};

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
    s: &super::super::dominance::Structure,
) -> Result<Option<CoroutineFlowPlan>, RegionError> {
    let prototype = storage.execution().declaration().prototype_units();
    let suffix = b"Lkotlin/coroutines/Continuation;)Ljava/lang/Object;";
    if prototype.len() < suffix.len()
        || !units(&prototype[prototype.len() - suffix.len()..], suffix)
    {
        return Ok(None);
    }
    for index in 0..f.nodes.len() {
        let node = NodeId(super::super::build::id(index)?);
        if !f.nodes[index].terminal
            || g.outgoing[index].len() < 4
            || !matches!(
                storage.execution().blocks()[f.nodes[index].block.0 as usize].terminal(),
                Some(Terminal::Switch { .. })
            )
        {
            continue;
        }
        match candidate(storage, f, g, s, node)? {
            Ok(plan) => {
                return Ok(Some(plan));
            }
            Err(reason) => reject(storage, RegionId(node.0), reason),
        }
    }
    Ok(None)
}

fn candidate(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    s: &super::super::dominance::Structure,
    dispatch: NodeId,
) -> Result<Result<CoroutineFlowPlan, &'static str>, RegionError> {
    macro_rules! need {
        ($test:expr,$why:literal) => {
            if !$test {
                return Ok(Err($why));
            }
        };
    }
    let flow = storage.execution();
    need!(
        f.nodes.iter().all(|n| f.lock_entries.empty_at(n.id))
            && flow.access_flags() & crate::substrate::flags::SYNCHRONIZED == 0,
        "flow_lock_boundary"
    );
    need!(
        g.exceptions
            .iter()
            .flatten()
            .all(|&e| !Graph::exception_possible(f, e)
                || Graph::exception_target(f, storage, e).is_none()),
        "flow_protection_not_established"
    );
    let mut initial = None;
    let mut invalid = None;
    let mut states = Vec::new();
    for &link in &g.outgoing[dispatch.0 as usize] {
        need!(
            f.link_plans[link.0 as usize].copies.is_empty(),
            "flow_dispatch_copy"
        );
        let LinkOrigin::Edge(edge) = f.links[link.0 as usize].origin else {
            return Ok(Err("flow_dispatch_origin"));
        };
        match flow.edges()[edge.0 as usize].role() {
            EdgeRole::Normal(NormalEdge::SwitchCase { key: 0, .. }) => initial = Some(link),
            EdgeRole::Normal(NormalEdge::SwitchCase { key, .. }) if key > 0 => {
                let Ok(state) = i32::try_from(key) else {
                    return Ok(Err("flow_state_range"));
                };
                push(&mut states, (state, link))?;
            }
            EdgeRole::Normal(NormalEdge::SwitchDefault) => invalid = Some(link),
            _ => return Ok(Err("flow_dispatch_state")),
        }
    }
    let (Some(initial), Some(invalid)) = (initial, invalid) else {
        return Ok(Err("flow_dispatch_roles"));
    };
    let Some([selector]) = storage
        .facts()
        .terminal_uses(f.nodes[dispatch.0 as usize].block)
    else {
        return Ok(Err("flow_selector"));
    };
    let Some(selector_op) = selector.read().ok().and_then(|r| definition(storage, r)) else {
        return Ok(Err("flow_selector_definition"));
    };
    let Some((label, continuation)) = field_get(storage, selector_op)? else {
        return Ok(Err("flow_label"));
    };
    need!(
        field_named(storage, selector_op, b"label", b"I")?,
        "flow_label_identity"
    );
    let mut result_op = None;
    let mut marker = None;
    for op in flow
        .operations()
        .iter()
        .take(f.nodes[dispatch.0 as usize].operations.start as usize)
    {
        if field_get(storage, op.id())?.is_some_and(|(_, r)| r == continuation)
            && field_named(storage, op.id(), b"result", b"Ljava/lang/Object;")?
        {
            result_op = Some(op.id());
        }
        if let Some(defs) = storage.facts().operation_definitions(op.id()) {
            for d in defs {
                if let Materialization::Store(local) = d.materialization()
                    && marker_source(storage, local)?
                {
                    marker = Some((local, op.id()));
                }
            }
        }
    }
    let (Some(result_op), Some((marker, marker_op))) = (result_op, marker) else {
        return Ok(Err("flow_prefix_values"));
    };
    let Some(result) = storage
        .facts()
        .operation_definitions(result_op)
        .and_then(|d| d.first())
        .and_then(|d| match d.materialization() {
            Materialization::Store(l) => Some(l),
            _ => None,
        })
    else {
        return Ok(Err("flow_result_storage"));
    };
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
        "flow_prefix_protocol"
    );
    need!(
        invariant_local(storage, marker, Some(marker_op), None)
            && invariant_local(storage, result, Some(result_op), None)
            && invariant_local(
                storage,
                continuation,
                None,
                Some((
                    f.nodes[dispatch.0 as usize].operations.start,
                    f.nodes[dispatch.0 as usize].block
                ))
            ),
        "flow_protocol_reassigned"
    );
    let invalid_start = f.links[invalid.0 as usize].to;
    let Some(invalid_nodes) = ending_corridor(f, g, invalid_start)? else {
        return Ok(Err("flow_invalid_corridor"));
    };
    need!(
        super::body::invalid_nodes(storage, f, g, &invalid_nodes, Some(invalid_start), invalid)?
            .is_ok(),
        "flow_invalid_arm"
    );
    let mut sites = Vec::new();
    let mut protocol = Vec::new();
    let mut suspended_nodes = Vec::new();
    for index in 0..f.nodes.len() {
        let node = NodeId(super::super::build::id(index)?);
        if !f.nodes[index].terminal || g.outgoing[index].len() != 2 {
            continue;
        }
        let block = f.nodes[index].block;
        let Some(Terminal::Branch {
            op: op @ (BranchOp::Eq | BranchOp::Ne),
            ..
        }) = flow.blocks()[block.0 as usize].terminal()
        else {
            continue;
        };
        let Some([a, b]) = storage.facts().terminal_uses(block) else {
            continue;
        };
        let (Ok(a), Ok(b)) = (a.read(), b.read()) else {
            continue;
        };
        let call_read = if a.local() == Some(marker) {
            b
        } else if b.local() == Some(marker) {
            a
        } else {
            continue;
        };
        need!(
            matches!(
                call_read,
                StorageRead::Value {
                    view: Interpretation::Direct(Family::Reference),
                    ..
                }
            ) && terminal_plain(f, block),
            "flow_guard_interpretation_or_phase"
        );
        let Some(call) = definition(storage, call_read) else {
            continue;
        };
        if !suspending_signature(storage, call, continuation)? {
            continue;
        }
        need!(
            g.incoming[index].len() == 1
                && f.links[g.incoming[index][0].0 as usize].from == g.op_node[call.0 as usize]
                && f.nodes[index].operations.is_empty(),
            "flow_call_guard_adjacency"
        );
        let Some(store) = call.0.checked_sub(1).map(OpId) else {
            return Ok(Err("flow_state_store"));
        };
        let Some([state_use, _]) = inputs(storage, store) else {
            return Ok(Err("flow_state_operands"));
        };
        let Ok(StorageRead::Literal { bits, .. }) = state_use.read() else {
            return Ok(Err("flow_state_not_literal"));
        };
        let Ok(state) = i32::try_from(bits.value) else {
            return Ok(Err("flow_state_not_positive"));
        };
        let Some(&(_, resume_link)) = states.iter().find(|&&(s, _)| s == state) else {
            return Ok(Err("flow_state_without_resume"));
        };
        need!(
            state_store(storage, store, label, continuation, state)?,
            "flow_publication_binding"
        );
        need!(
            !sites.iter().any(|p: &CoroutineSite| p.state == state),
            "flow_state_has_multiple_calls"
        );
        let mut normal = None;
        let mut suspended = None;
        for &link in &g.outgoing[index] {
            let LinkOrigin::Edge(edge) = f.links[link.0 as usize].origin else {
                return Ok(Err("flow_guard_edge"));
            };
            let equality = matches!(
                (op, flow.edges()[edge.0 as usize].role()),
                (BranchOp::Eq, EdgeRole::Normal(NormalEdge::ConditionalTrue))
                    | (BranchOp::Ne, EdgeRole::Normal(NormalEdge::ConditionalFalse))
            );
            if equality {
                suspended = Some(link);
            } else {
                normal = Some(link);
            }
        }
        let (Some(normal), Some(suspended)) = (normal, suspended) else {
            return Ok(Err("flow_guard_roles"));
        };
        let returned = f.links[suspended.0 as usize].to;
        need!(
            f.nodes[returned.0 as usize].operations.is_empty()
                && g.outgoing[returned.0 as usize].is_empty()
                && matches!(
                    flow.blocks()[f.nodes[returned.0 as usize].block.0 as usize].terminal(),
                    Some(Terminal::Return { .. })
                ),
            "flow_marker_return_shape"
        );
        let Some([value]) = storage
            .facts()
            .terminal_uses(f.nodes[returned.0 as usize].block)
        else {
            return Ok(Err("flow_marker_return_value"));
        };
        need!(
            value.read().ok().and_then(StorageRead::local) == Some(marker)
                && f.link_plans[suspended.0 as usize].copies.is_empty(),
            "flow_marker_return_value"
        );
        if !suspended_nodes.contains(&returned) {
            push(&mut suspended_nodes, returned)?;
        }
        push(&mut protocol, store)?;
        push(
            &mut sites,
            CoroutineSite {
                state,
                call,
                continuation_argument: super::super::build::id(
                    inputs(storage, call)
                        .ok_or(bug("matched call lost inputs"))?
                        .len()
                        .checked_sub(1)
                        .ok_or(bug("matched call lost continuation"))?,
                )?,
                guard: node,
                normal,
                suspended,
                resume: Vec::new(),
                spills: Vec::new(),
            },
        )?;
        let _ = resume_link;
    }
    need!(sites.len() == states.len(), "flow_missing_site");
    for &node in &suspended_nodes {
        need!(
            g.incoming[node.0 as usize]
                .iter()
                .all(|l| sites.iter().any(|p| p.suspended == *l)),
            "flow_marker_foreign_arrival"
        );
    }
    let mut source_mask = super::super::build::filled(f.nodes.len(), false)?;
    let mut queue = Vec::new();
    push(&mut queue, f.links[initial.0 as usize].to)?;
    while let Some(node) = queue.pop() {
        if source_mask[node.0 as usize] {
            continue;
        }
        source_mask[node.0 as usize] = true;
        for &link in &g.outgoing[node.0 as usize] {
            if !sites.iter().any(|p| p.suspended == link) {
                push(&mut queue, f.links[link.0 as usize].to)?;
            }
        }
    }
    let mut entry_check = None;
    for node in &f.nodes {
        if source_mask[node.id.0 as usize] {
            for index in node.operations.clone() {
                if failure_check(storage, OpId(index), result)? {
                    entry_check = Some(OpId(index));
                }
            }
        }
    }
    let Some(entry_check) = entry_check else {
        return Ok(Err("flow_entry_failure_check"));
    };
    let mut entry_at = f.links[initial.0 as usize].to;
    let mut found_entry = false;
    for _ in 0..f.nodes.len() {
        for index in f.nodes[entry_at.0 as usize].operations.clone() {
            if OpId(index) == entry_check {
                found_entry = true;
                break;
            }
            need!(
                matches!(
                    flow.operations()[index as usize].kind(),
                    OpKind::Move { .. } | OpKind::Const { .. }
                ) || field_put(storage, OpId(index))?.is_some_and(|(_, r, _)| r == continuation),
                "flow_work_before_entry_check"
            );
        }
        if found_entry {
            break;
        }
        let [link] = g.outgoing[entry_at.0 as usize].as_slice() else {
            return Ok(Err("flow_entry_check_not_private"));
        };
        entry_at = f.links[link.0 as usize].to;
    }
    need!(found_entry, "flow_entry_check_not_reached");
    push(&mut protocol, entry_check)?;
    for site in &mut sites {
        need!(source_mask[site.guard.0 as usize], "flow_unreachable_site");
        let resume_link = states
            .iter()
            .find(|&&(state, _)| state == site.state)
            .ok_or(bug("site lost resume state"))?
            .1;
        let mut at = f.links[resume_link.0 as usize].to;
        let mut arrival = resume_link;
        let resumed_edge;
        loop {
            need!(
                site.resume.len() < f.nodes.len()
                    && !site.resume.contains(&at)
                    && !source_mask[at.0 as usize],
                "flow_resume_corridor"
            );
            need!(
                g.incoming[at.0 as usize].as_slice() == [arrival],
                "flow_foreign_resume_arrival"
            );
            push(&mut site.resume, at)?;
            let [link] = g.outgoing[at.0 as usize].as_slice() else {
                return Ok(Err("flow_resume_branch"));
            };
            at = f.links[link.0 as usize].to;
            arrival = *link;
            if source_mask[at.0 as usize] {
                resumed_edge = *link;
                break;
            }
        }
        let merge = at;
        let Some(&normal_edge) = g.incoming[merge.0 as usize]
            .iter()
            .find(|&&l| source_mask[f.links[l.0 as usize].from.0 as usize])
        else {
            return Ok(Err("flow_normal_merge"));
        };
        need!(
            g.incoming[merge.0 as usize].len() == 2,
            "flow_merge_foreign_arrival"
        );
        let mut normal_at = f.links[site.normal.0 as usize].to;
        let mut normal_arrival = site.normal;
        for _ in 0..f.nodes.len() {
            if normal_at == merge {
                break;
            }
            need!(
                g.incoming[normal_at.0 as usize].as_slice() == [normal_arrival],
                "flow_normal_corridor_arrival"
            );
            need!(
                f.nodes[normal_at.0 as usize]
                    .operations
                    .clone()
                    .all(|i| matches!(flow.operations()[i as usize].kind(), OpKind::Move { .. })),
                "flow_normal_only_effect"
            );
            let [link] = g.outgoing[normal_at.0 as usize].as_slice() else {
                return Ok(Err("flow_normal_corridor_branch"));
            };
            normal_at = f.links[link.0 as usize].to;
            normal_arrival = *link;
        }
        need!(
            normal_at == merge && normal_arrival == normal_edge,
            "flow_resume_does_not_join_its_call"
        );
        let mut saved = Vec::new();
        let mut corridor = Vec::new();
        at = g.op_node[site.call.0 as usize];
        for _ in 0..f.nodes.len() {
            let [link] = g.incoming[at.0 as usize].as_slice() else {
                break;
            };
            let from = f.links[link.0 as usize].from;
            if !source_mask[from.0 as usize] || g.outgoing[from.0 as usize].len() != 1 {
                break;
            }
            push(&mut corridor, from)?;
            for index in f.nodes[from.0 as usize].operations.clone().rev() {
                let op = OpId(index);
                if let Some((field, receiver, value)) = field_put(storage, op)?
                    && receiver == continuation
                    && field != label
                    && !saved.iter().any(|&(id, _, _)| id == field)
                {
                    need!(
                        matches!(
                            flow.operations()[index as usize].kind(),
                            OpKind::Field {
                                kind: ValueKind::Word | ValueKind::Wide | ValueKind::Object,
                                ..
                            }
                        ),
                        "flow_narrow_spill"
                    );
                    push(&mut saved, (field, op, value))?;
                    push(&mut protocol, op)?;
                }
            }
            at = from;
        }
        let mut restored = Vec::new();
        let mut checked = false;
        for &node in &site.resume {
            for index in f.nodes[node.0 as usize].operations.clone() {
                let op = OpId(index);
                if let Some((field, receiver)) = field_get(storage, op)? {
                    need!(
                        !checked
                            && receiver == continuation
                            && saved.iter().any(|&(id, _, _)| id == field),
                        "flow_unmatched_restored_field"
                    );
                    push(&mut restored, (field, op))?;
                } else if failure_check(storage, op, result)? {
                    need!(!checked, "flow_duplicate_resume_check");
                    checked = true;
                } else {
                    need!(
                        matches!(
                            flow.operations()[index as usize].kind(),
                            OpKind::Move { .. }
                        ),
                        "flow_resume_effect"
                    );
                    let Some([input]) = inputs(storage, op) else {
                        return Ok(Err("flow_resume_move"));
                    };
                    let Ok(read) = input.read() else {
                        return Ok(Err("flow_resume_move_gap"));
                    };
                    let read = normalize(storage, read);
                    need!(
                        read.local() == Some(result)
                            || read.local() == Some(marker)
                            || definition(storage, read)
                                .is_some_and(|at| restored.iter().any(|&(_, r)| r == at))
                            || read.local().is_some_and(|local| storage
                                .facts()
                                .locals()
                                .iter()
                                .any(|l| l.id() == local
                                    && matches!(
                                        l.kind(),
                                        LocalKind::Parameter {
                                            role: EntryRole::This,
                                            ..
                                        }
                                    ))),
                        "flow_resume_move_not_protocol"
                    );
                }
            }
        }
        need!(checked, "flow_missing_resume_check");
        let Some(normal) = assignments(storage, f, normal_edge)? else {
            return Ok(Err("flow_normal_copy"));
        };
        let Some(resumed) = assignments(storage, f, resumed_edge)? else {
            return Ok(Err("flow_resume_copy"));
        };
        let mut matched_result = false;
        for &(destination, value) in &resumed {
            let value = normalize(storage, value);
            let expected = if value.local() == Some(result) {
                matched_result = true;
                normal
                    .iter()
                    .find(|&&(_, r)| definition(storage, normalize(storage, r)) == Some(site.call))
                    .map(|&(_, r)| normalize(storage, r))
            } else {
                let Some(get) = definition(storage, value) else {
                    return Ok(Err("flow_resume_copy_source"));
                };
                let Some(&(field, _)) = restored.iter().find(|&&(_, op)| op == get) else {
                    return Ok(Err("flow_copy_not_restored"));
                };
                let Some(&(_, put, r)) = saved.iter().find(|&&(id, _, _)| id == field) else {
                    return Ok(Err("flow_copy_not_saved"));
                };
                push(&mut site.spills, (put, get))?;
                Some(normalize(storage, r))
            };
            let Some(expected) = expected else {
                return Ok(Err("flow_result_not_copied"));
            };
            let actual = normal
                .iter()
                .find(|&&(d, _)| d == destination)
                .map(|&(_, r)| normalize(storage, r));
            need!(
                actual.is_some_and(|r| same_value(r, expected))
                    || (actual.is_none()
                        && expected.local() == Some(destination)
                        && !corridor.iter().any(|&node| f.nodes[node.0 as usize]
                            .operations
                            .clone()
                            .any(
                                |i| storage
                                    .facts()
                                    .operation_definitions(OpId(i))
                                    .is_some_and(|ds| ds.iter().any(|d| d.materialization()
                                        == Materialization::Store(destination)))
                            ))),
                "flow_resume_value_mismatch"
            );
        }
        need!(
            matched_result
                && normal
                    .iter()
                    .all(|&(d, _)| resumed.iter().any(|&(r, _)| r == d)),
            "flow_unmatched_merge_copy"
        );
    }
    let mut abi = Vec::new();
    let mut source_nodes = Vec::new();
    for node in &f.nodes {
        if source_mask[node.id.0 as usize] {
            push(&mut source_nodes, node.id)?;
        } else {
            need!(
                node.id.0 <= dispatch.0
                    || invalid_nodes.contains(&node.id)
                    || suspended_nodes.contains(&node.id)
                    || sites.iter().any(|p| p.resume.contains(&node.id)),
                "flow_unclassified_content"
            );
            push(&mut abi, node.id)?;
        }
    }
    for link in &f.links {
        if source_mask[link.to.0 as usize] && !source_mask[link.from.0 as usize] {
            need!(
                link.id == initial || sites.iter().any(|p| p.resume.last() == Some(&link.from)),
                "flow_foreign_source_arrival"
            );
        }
    }
    for op in flow.operations() {
        need!(
            plain_operation(storage, f, op.id()) && inputs(storage, op.id()).is_some(),
            "flow_operation_gap_or_phase"
        );
    }
    need!(
        closed_values(
            storage,
            f,
            &source_nodes,
            &protocol,
            &sites,
            dispatch,
            (continuation, marker, result)
        ),
        "flow_protocol_escape"
    );
    let Some(root) = f.root else {
        return Ok(Err("flow_root"));
    };
    let entry = f.links[initial.0 as usize].to;
    let has_cycle = source_nodes
        .iter()
        .any(|n| s.cyclic[s.component[n.0 as usize]]);
    let mut explicit_predicates = Vec::new();
    let (source, cycle) = if has_cycle {
        let mut initial_mask = super::super::build::filled(f.nodes.len(), false)?;
        let mut at = entry;
        for _ in 0..f.nodes.len() {
            if s.cyclic[s.component[at.0 as usize]] {
                break;
            }
            initial_mask[at.0 as usize] = true;
            let [link] = g.outgoing[at.0 as usize].as_slice() else {
                return Ok(Err("cycle_initial_branch"));
            };
            at = f.links[link.0 as usize].to;
        }
        let header = at;
        let Some(initial_header) = g.incoming[header.0 as usize]
            .iter()
            .find(|&&id| initial_mask[f.links[id.0 as usize].from.0 as usize])
            .copied()
        else {
            return Ok(Err("cycle_initial_header_edge"));
        };
        let component = s.component[header.0 as usize];
        need!(
            s.cyclic[component]
                && f.nodes[header.0 as usize].terminal
                && f.nodes[header.0 as usize].operations.is_empty()
                && matches!(
                    flow.blocks()[f.nodes[header.0 as usize].block.0 as usize].terminal(),
                    Some(Terminal::Branch { .. })
                ),
            "cycle_head_condition"
        );
        need!(
            source_nodes
                .iter()
                .all(|n| !s.cyclic[s.component[n.0 as usize]]
                    || s.component[n.0 as usize] == component),
            "cycle_multiple_components"
        );
        need!(
            s.retreating
                .iter()
                .filter(|&&l| source_mask[f.links[l.0 as usize].from.0 as usize]
                    && s.component[f.links[l.0 as usize].from.0 as usize] == component)
                .all(|l| f.links[l.0 as usize].to == header),
            "cycle_nested_header"
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
            return Ok(Err("cycle_condition_roles"));
        };
        let mut body_mask = super::super::build::filled(f.nodes.len(), false)?;
        let mut tail_mask = super::super::build::filled(f.nodes.len(), false)?;
        for &node in &source_nodes {
            if node != header {
                if s.component[node.0 as usize] == component {
                    body_mask[node.0 as usize] = true;
                } else if !initial_mask[node.0 as usize] {
                    tail_mask[node.0 as usize] = true;
                }
            }
        }
        let Ok((mut initial_roots, mut e0)) =
            select(f, root, &initial_mask, &source_mask, &sites, dispatch, true)?
        else {
            return Ok(Err("cycle_initial_owner"));
        };
        explicit_predicates.append(&mut e0);
        let Ok((mut body_roots, mut e1)) =
            select(f, root, &body_mask, &source_mask, &sites, dispatch, true)?
        else {
            return Ok(Err("cycle_body_owner"));
        };
        explicit_predicates.append(&mut e1);
        let Ok((mut tail_roots, mut e2)) =
            select(f, root, &tail_mask, &source_mask, &sites, dispatch, true)?
        else {
            return Ok(Err("cycle_tail_owner"));
        };
        explicit_predicates.append(&mut e2);
        need!(
            rotate(f, &mut initial_roots, entry)
                && rotate(f, &mut body_roots, f.links[repeat.0 as usize].to)
                && rotate(f, &mut tail_roots, f.links[exit.0 as usize].to),
            "cycle_entry_order"
        );
        let Some(&last) = body_roots.last() else {
            return Ok(Err("cycle_empty_body"));
        };
        let contains = f.membership(last)?;
        let Some(implicit_latch) = f
            .links
            .iter()
            .find(|l| {
                l.to == header && contains(l.from) && g.outgoing[l.from.0 as usize].len() == 1
            })
            .map(|l| l.id)
        else {
            return Ok(Err("cycle_latch_owner"));
        };
        need!(
            f.links
                .iter()
                .filter(|l| l.to == header
                    && source_mask[l.from.0 as usize]
                    && s.component[l.from.0 as usize] == component)
                .count()
                == 1,
            "cycle_multiple_latches"
        );
        need!(
            last_source_node(f, last) == Some(f.links[implicit_latch.0 as usize].from),
            "cycle_latch_not_lexical_end"
        );
        let mut roots = Vec::new();
        for &id in initial_roots.iter().chain(&body_roots).chain(&tail_roots) {
            push(&mut roots, id)?;
        }
        (
            roots,
            Some(CoroutineCyclePlan {
                condition: LoopCondition {
                    node: header,
                    repeat,
                    exit,
                },
                initial: initial_roots,
                body: body_roots,
                tail: tail_roots,
                implicit_latch,
                initial_header,
            }),
        )
    } else {
        let Ok((mut roots, explicit)) =
            select(f, root, &source_mask, &source_mask, &sites, dispatch, false)?
        else {
            return Ok(Err("flow_mixed_control_owner"));
        };
        explicit_predicates = explicit;
        need!(rotate(f, &mut roots, entry), "flow_initial_source_owner");
        (roots, None)
    };
    let mut covered = super::super::build::filled(f.nodes.len(), false)?;
    if let Some(cycle) = &cycle {
        covered[cycle.condition.node.0 as usize] = true;
    }
    for &id in &source {
        for &node in f.members(id)? {
            need!(!covered[node.0 as usize], "flow_duplicate_source_owner");
            covered[node.0 as usize] = true;
        }
    }
    need!(covered == source_mask, "flow_source_coverage");
    let mut owner = super::super::build::filled(f.nodes.len(), None)?;
    for (index, &id) in source.iter().enumerate() {
        for &node in f.members(id)? {
            owner[node.0 as usize] = Some(index);
        }
    }
    let mut gotos = Vec::new();
    for link in &f.links {
        if cycle.as_ref().is_some_and(|p| {
            link.from == p.condition.node
                || link.id == p.implicit_latch
                || link.id == p.initial_header
        }) {
            continue;
        }
        if source_mask[link.from.0 as usize]
            && source_mask[link.to.0 as usize]
            && owner[link.from.0 as usize] != owner[link.to.0 as usize]
        {
            let from = owner[link.from.0 as usize].ok_or(bug("source lost owner"))?;
            if source
                .get(from + 1)
                .is_none_or(|id| f.regions[id.0 as usize].entry != Some(link.to))
            {
                push(&mut gotos, link.id)?;
            }
        }
    }
    let mut abi_entries = Vec::new();
    for link in &f.links {
        if !source_mask[link.from.0 as usize]
            && source_mask[link.to.0 as usize]
            && !abi_entries.contains(&link.to)
        {
            push(&mut abi_entries, link.to)?;
        }
    }
    for site in &sites {
        let target = f.links[site.normal.0 as usize].to;
        if !abi_entries.contains(&target) {
            push(&mut abi_entries, target)?;
        }
    }
    let mut guards = Vec::new();
    for region in &f.regions {
        if let RegionKind::If {
            true_link,
            false_link,
        } = region.kind
            && let Some(site) = sites.iter().find(|site| region.entry == Some(site.guard))
        {
            let [header, yes, no] = region.children.as_slice() else {
                return Ok(Err("flow_guard_children"));
            };
            let normal = if site.normal == true_link {
                *yes
            } else if site.normal == false_link {
                *no
            } else {
                return Ok(Err("flow_guard_link_owner"));
            };
            need!(
                f.members(normal)?.iter().all(|n| source_mask[n.0 as usize]),
                "flow_guard_normal_coverage"
            );
            push(
                &mut guards,
                CoroutineGuardControl {
                    control: region.id,
                    header: *header,
                    normal,
                },
            )?;
        }
    }
    let mut active = super::super::build::filled(f.regions.len(), false)?;
    let mut work = Vec::new();
    for &id in &source {
        push(&mut work, id)?;
    }
    while let Some(id) = work.pop() {
        if active[id.0 as usize] {
            continue;
        }
        active[id.0 as usize] = true;
        if let Some(guard) = guards.iter().find(|g| g.control == id) {
            push(&mut work, guard.header)?;
            push(&mut work, guard.normal)?;
        } else {
            for &child in &f.regions[id.0 as usize].children {
                push(&mut work, child)?;
            }
        }
    }
    if let Some(cycle) = &cycle {
        active[cycle.condition.node.0 as usize] = true;
    }
    let mut completions = Vec::new();
    for (index, &used) in active.iter().enumerate() {
        if !used {
            continue;
        }
        let id = RegionId(super::super::build::id(index)?);
        let region = &f.regions[index];
        if cycle.as_ref().is_some_and(|p| {
            matches!(region.continuation,Continuation::Goto(label)if label.0==p.condition.node.0)
                && (p.body.last() == Some(&id) || p.initial.last() == Some(&id))
        }) {
            push(&mut completions, (id, None))?;
            continue;
        }
        if guards.iter().any(|g| g.control == id) {
            push(&mut completions, (id, None))?;
            continue;
        }
        if let Continuation::Break(scope) = region.continuation
            && !active[scope.0 as usize]
        {
            let Some(follow) = region.follow.filter(|n| source_mask[n.0 as usize]) else {
                return Ok(Err("flow_retired_scope_follow"));
            };
            let next = source
                .iter()
                .position(|&r| r == id)
                .and_then(|i| source.get(i + 1))
                .and_then(|r| f.regions[r.0 as usize].entry);
            push(
                &mut completions,
                (id, (next != Some(follow)).then_some(follow)),
            )?;
        }
    }
    let mut retired_transfers = Vec::new();
    let mut abi_gotos = Vec::new();
    for link in &f.links {
        if !source_mask[link.from.0 as usize] {
            if !matches!(link.origin, LinkOrigin::WithinBlock(_))
                || link.from.0.checked_add(1) != Some(link.to.0)
            {
                push(&mut abi_gotos, link.id)?;
            }
        } else if source_mask[link.to.0 as usize]
            && matches!(f.link_plans[link.id.0 as usize].action,TransferAction::Break(scope) if !active[scope.0 as usize])
        {
            push(&mut retired_transfers, link.id)?;
        }
    }
    Ok(Ok(CoroutineFlowPlan {
        source,
        entry_check,
        abi,
        sites,
        protocol_operations: protocol,
        gotos,
        abi_entries,
        guards,
        completions,
        abi_gotos,
        retired_transfers,
        cycle,
        explicit_predicates,
    }))
}

fn rotate(f: &RegionFacts, roots: &mut [RegionId], entry: NodeId) -> bool {
    if roots.is_empty() {
        return false;
    }
    let Some(at) = roots
        .iter()
        .position(|id| f.regions[id.0 as usize].entry == Some(entry))
    else {
        return false;
    };
    roots.rotate_left(at);
    true
}
fn last_source_node(f: &RegionFacts, mut id: RegionId) -> Option<NodeId> {
    for _ in 0..f.regions.len() {
        let region = &f.regions[id.0 as usize];
        match region.kind {
            RegionKind::Block(node) => return Some(node),
            RegionKind::Sequence | RegionKind::Graph { .. } => id = *region.children.last()?,
            _ => return None,
        }
    }
    None
}
type SelectedControls = (Vec<RegionId>, Vec<NodeId>);
fn select(
    f: &RegionFacts,
    root: RegionId,
    mask: &[bool],
    source: &[bool],
    sites: &[CoroutineSite],
    dispatch: NodeId,
    partition: bool,
) -> Result<Result<SelectedControls, ()>, RegionError> {
    let mut work = Vec::new();
    push(&mut work, root)?;
    let mut result = Vec::new();
    let mut explicit = Vec::new();
    while let Some(id) = work.pop() {
        let region = &f.regions[id.0 as usize];
        if region.member_count == 0 || f.members(region.id)?.iter().all(|n| !mask[n.0 as usize]) {
            continue;
        }
        if f.members(region.id)?.iter().all(|n| mask[n.0 as usize]) {
            if matches!(region.continuation,Continuation::Goto(label)if !source[label.0 as usize]) {
                return Ok(Err(()));
            }
            push(&mut result, id)?;
            continue;
        }
        let guard = matches!(region.kind, RegionKind::If { .. })
            && region
                .entry
                .is_some_and(|n| sites.iter().any(|s| s.guard == n));
        let partial_if = partition && matches!(region.kind, RegionKind::If { .. });
        if !(matches!(region.kind, RegionKind::Sequence | RegionKind::Graph { .. })
            || (matches!(region.kind, RegionKind::Switch { .. }) && region.entry == Some(dispatch))
            || guard
            || partial_if)
        {
            return Ok(Err(()));
        }
        if partial_if && !guard && region.entry.is_some_and(|n| mask[n.0 as usize]) {
            push(
                &mut explicit,
                region.entry.ok_or(bug("partition If lost predicate"))?,
            )?;
        }
        for &child in region.children.iter().rev() {
            push(&mut work, child)?;
        }
    }
    Ok(Ok((result, explicit)))
}

fn closed_values(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    nodes: &[NodeId],
    protocol: &[OpId],
    sites: &[CoroutineSite],
    dispatch: NodeId,
    values: (LocalId, LocalId, LocalId),
) -> bool {
    let (continuation, marker, result) = values;
    let private = |local| {
        local==continuation || storage.facts().definitions().iter().any(|d|d.operation().0<f.nodes[dispatch.0 as usize].operations.start && matches!(d.materialization(),Materialization::Store(l)|Materialization::Deferred{local:l,..} if l==local))
    };
    for &node in nodes {
        for index in f.nodes[node.0 as usize].operations.clone() {
            let op = OpId(index);
            if protocol.contains(&op) {
                continue;
            }
            let Some(uses) = inputs(storage, op) else {
                return false;
            };
            for (argument, u) in uses.iter().enumerate() {
                let Ok(read) = u.read() else {
                    return false;
                };
                if let Some(local) = read.local()
                    && private(local)
                {
                    let move_ = matches!(
                        storage.execution().operations()[index as usize].kind(),
                        OpKind::Move { .. }
                    ) && (local == marker || local == result)
                        && storage.facts().operation_definitions(op).is_some_and(|ds| {
                            ds.iter()
                                .all(|d| d.materialization() == Materialization::Store(local))
                        });
                    if !(move_
                        || (sites.iter().any(|p| p.call == op)
                            && argument + 1 == uses.len()
                            && local == continuation))
                    {
                        return false;
                    }
                }
            }
        }
        if f.nodes[node.0 as usize].terminal
            && !sites.iter().any(|p| p.guard == node)
            && storage
                .facts()
                .terminal_uses(f.nodes[node.0 as usize].block)
                .is_none_or(|us| {
                    us.iter().any(|u| {
                        !u.read()
                            .is_ok_and(|r| r.local().is_none_or(|l| !private(l)))
                    })
                })
        {
            return false;
        }
        for link in &f.links {
            if link.from == node {
                for &id in &f.link_plans[link.id.0 as usize].copies {
                    for step in storage.facts().copy_plans()[id.0 as usize].steps() {
                        if let CopyStep::Assign {
                            destination,
                            source,
                            ..
                        } = *step
                            && (private(destination) || source.local().is_some_and(private))
                        {
                            return false;
                        }
                    }
                }
            }
        }
    }
    true
}
