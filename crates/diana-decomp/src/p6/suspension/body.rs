//! Closes a source-body alternative over one already established suspension.
//! All ABI entry/reentry and invalid-state code remains an owned, explicit
//! protocol attachment; it is not assumed pure or validated against a runtime.
use super::super::build::filled;
use super::*;

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    graph: &Graph,
) -> Result<Option<CoroutineBodyPlan>, RegionError> {
    if f.suspensions.len() != 1 {
        return Ok(None);
    }
    match candidate(storage, f, graph)? {
        Ok(body) => {
            if cfg!(feature = "trace-passes") {
                let flow = storage.execution();
                let key = flow.owner_facts().key();
                eprintln!(
                    "[coroutine-body] dex={} class={} method={} dispatch={} layout=suspend_body bodies={}",
                    key.dex.0,
                    key.class_def.0,
                    flow.method_id().0,
                    body.dispatch.0,
                    body.sequence.len()
                );
            }
            return Ok(Some(body));
        }
        Err(reason) => {
            if cfg!(feature = "trace-passes") {
                let flow = storage.execution();
                let key = flow.owner_facts().key();
                eprintln!(
                    "[coroutine-body] dex={} class={} method={} dispatch={} layout=explicit reason={}",
                    key.dex.0,
                    key.class_def.0,
                    flow.method_id().0,
                    f.suspensions[0].dispatch.0,
                    reason
                );
            }
        }
    }
    Ok(None)
}

fn candidate(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    graph: &Graph,
) -> Result<Result<CoroutineBodyPlan, &'static str>, RegionError> {
    macro_rules! require {
        ($test:expr,$why:literal) => {
            if !$test {
                return Ok(Err($why));
            }
        };
    }
    let plan = f.suspensions[0];
    let flow = storage.execution();
    let Some(root) = f.root else {
        return Ok(Err("body_root_unavailable"));
    };
    let source = &f.regions[root.0 as usize];
    require!(
        matches!(source.kind, RegionKind::Sequence)
            && source.children.len() >= 3
            && source.continuation == Continuation::None,
        "body_requires_one_selected_root_sequence"
    );
    let Some(dispatch_index) = source
        .children
        .iter()
        .position(|&child| child == plan.dispatch)
    else {
        return Ok(Err("body_dispatch_is_not_a_root_child"));
    };
    require!(
        dispatch_index > 0 && dispatch_index + 1 < source.children.len(),
        "body_requires_entry_prefix_and_tail"
    );
    let dispatch = &f.regions[plan.dispatch.0 as usize];
    let decision = dispatch
        .decision
        .as_ref()
        .ok_or(bug("suspension lost its decision product"))?;
    let initial = decision.tests[0].body;
    let invalid = decision.default;
    let Some(follow) = dispatch.follow else {
        return Ok(Err("body_follow_unavailable"));
    };
    require!(
        f.regions[source.children[dispatch_index + 1].0 as usize].entry == Some(follow),
        "body_tail_does_not_start_at_original_merge"
    );
    let prefix = &source.children[..dispatch_index];
    let mut prefix_nodes = Vec::new();
    for &control in prefix {
        for &node in f.members(control)? {
            push(&mut prefix_nodes, node)?;
        }
    }
    let Some(&first) = prefix_nodes.first() else {
        return Ok(Err("body_entry_prefix_is_empty"));
    };
    require!(
        first == NodeId(0) && graph.incoming[0].is_empty(),
        "body_entry_prefix_has_other_arrivals"
    );
    let block = f.nodes[first.0 as usize].block;
    let mut previous = None;
    for (index, &node) in prefix_nodes.iter().enumerate() {
        require!(
            !f.nodes[node.0 as usize].terminal && f.nodes[node.0 as usize].block == block,
            "body_entry_prefix_is_not_a_straight_original_block"
        );
        if let Some(previous) = previous {
            require!(
                graph.incoming[node.0 as usize].as_slice() == [previous],
                "body_entry_prefix_has_other_arrivals"
            );
        }
        let [link] = graph.outgoing[node.0 as usize].as_slice() else {
            return Ok(Err("body_entry_prefix_has_multiple_exits"));
        };
        let target = prefix_nodes
            .get(index + 1)
            .copied()
            .unwrap_or(decision.tests[0].predicate);
        require!(
            f.links[link.0 as usize].to == target
                && f.link_plans[link.0 as usize].copies.is_empty(),
            "body_entry_prefix_does_not_enter_dispatch"
        );
        previous = Some(*link);
    }
    require!(
        decision
            .tests
            .iter()
            .all(|test| [test.selected, test.remaining]
                .into_iter()
                .all(|link| f.link_plans[link.0 as usize].copies.is_empty())),
        "body_dispatch_has_edge_copy"
    );
    let Some(initial_node) = f.regions[initial.0 as usize].entry else {
        return Ok(Err("body_initial_entry_unavailable"));
    };
    require!(
        graph.incoming[initial_node.0 as usize].as_slice() == [decision.tests[0].selected],
        "body_initial_entry_is_shared"
    );
    let Some(parameter) = storage
        .facts()
        .locals()
        .iter()
        .find_map(|local| match local.kind() {
            LocalKind::Parameter {
                role: EntryRole::Parameter(0),
                ..
            } => Some(local.id()),
            _ => None,
        })
    else {
        return Ok(Err("body_resume_parameter_unavailable"));
    };
    let entry_range = f.nodes[initial_node.0 as usize].operations.clone();
    require!(
        !entry_range.is_empty()
            && failure_check(storage, OpId(entry_range.start), parameter)?
            && plain_operation(storage, f, OpId(entry_range.start)),
        "body_initial_failure_check_unavailable"
    );
    let marker =
        definition_op(storage, plan.marker).ok_or(bug("suspension marker lost its definition"))?;
    let selector = storage
        .facts()
        .terminal_uses(f.nodes[decision.tests[0].predicate.0 as usize].block)
        .and_then(|uses| uses.get(decision.tests[0].argument as usize))
        .and_then(|use_| use_.read().ok())
        .and_then(StorageRead::local)
        .ok_or(bug("suspension dispatch lost its selector"))?;
    let label_read =
        definition_op(storage, selector).ok_or(bug("suspension label lost its definition"))?;
    let last = *prefix_nodes
        .last()
        .ok_or(bug("established prefix lost its last portion"))?;
    let prefix_range =
        f.nodes[first.0 as usize].operations.start..f.nodes[last.0 as usize].operations.end;
    require!(
        prefix_range.contains(&marker.0) && prefix_range.contains(&label_read.0),
        "body_marker_or_label_is_outside_entry_prefix"
    );
    let mut operations = Vec::new();
    for index in prefix_range {
        let operation = OpId(index);
        let role = if operation == marker {
            ProtocolRole::Marker
        } else if operation == label_read {
            ProtocolRole::LabelRead
        } else {
            require!(
                matches!(
                    flow.operations()[index as usize].kind(),
                    OpKind::Const { .. }
                ) && storage
                    .facts()
                    .operation_definitions(operation)
                    .is_some_and(|defs| !defs.is_empty()
                        && defs
                            .iter()
                            .all(|d| matches!(d.materialization(), Materialization::Literal(_)))),
                "body_entry_prefix_contains_nonprotocol_work"
            );
            ProtocolRole::Literal
        };
        require!(
            plain_operation(storage, f, operation),
            "body_entry_prefix_has_reference_issue_phase_or_handler"
        );
        push(&mut operations, ProtocolEvent { operation, role })?;
    }
    require!(
        plain_operation(storage, f, plan.state_store),
        "body_state_publication_has_reference_issue_phase_or_handler"
    );
    push(
        &mut operations,
        ProtocolEvent {
            operation: plan.state_store,
            role: ProtocolRole::PublishState,
        },
    )?;
    push(
        &mut operations,
        ProtocolEvent {
            operation: plan.resume_check,
            role: ProtocolRole::ResumeCheck,
        },
    )?;
    let invalid_operations = invalid_arm(storage, f, graph, invalid, decision.tests[1].remaining)?;
    let Ok(invalid_operations) = invalid_operations else {
        return Ok(Err(
            "body_invalid_state_arm_is_not_closed_exception_construction",
        ));
    };
    for operation in invalid_operations {
        push(
            &mut operations,
            ProtocolEvent {
                operation,
                role: ProtocolRole::InvalidState,
            },
        )?;
    }
    let mut terminals = Vec::new();
    for test in &decision.tests {
        push(&mut terminals, f.nodes[test.predicate.0 as usize].block)?;
    }
    push(&mut terminals, plan.predicate)?;
    push(&mut terminals, plan.marker_return)?;
    for node in f.members(invalid)? {
        if f.nodes[node.0 as usize].terminal {
            push(&mut terminals, f.nodes[node.0 as usize].block)?;
        }
    }
    let mut implicit_locals = Vec::new();
    for operation in &operations {
        for definition in storage
            .facts()
            .operation_definitions(operation.operation)
            .ok_or(bug("protocol operation lost its definitions"))?
        {
            let local = match definition.materialization() {
                Materialization::Store(local)
                | Materialization::AllocationCarrier(local)
                | Materialization::Deferred { local, .. } => Some(local),
                _ => None,
            };
            if let Some(local) = local
                && !implicit_locals.contains(&local)
            {
                push(&mut implicit_locals, local)?;
            }
        }
    }
    if !implicit_locals.contains(&plan.result) {
        push(&mut implicit_locals, plan.result)?;
    }
    require!(
        closed_values(
            storage,
            &operations,
            &terminals,
            [plan.normal_copy, plan.resume_copy],
            &implicit_locals
        ),
        "body_protocol_value_escapes_into_readable_code"
    );
    let mut protocol_nodes = filled(f.nodes.len(), false)?;
    for &node in &prefix_nodes {
        protocol_nodes[node.0 as usize] = true;
    }
    for test in &decision.tests {
        protocol_nodes[test.predicate.0 as usize] = true;
    }
    for &node in f.members(plan.resume)?.iter().chain(f.members(invalid)?) {
        protocol_nodes[node.0 as usize] = true;
    }
    protocol_nodes[f.regions[plan.header.0 as usize]
        .entry
        .ok_or(bug("protocol guard lost its header"))?
        .0 as usize] = true;
    for &node in f.members(plan.suspended_body)? {
        protocol_nodes[node.0 as usize] = true;
    }
    for (node, is_protocol) in protocol_nodes.iter().copied().enumerate() {
        if is_protocol {
            let id = NodeId(super::super::build::id(node)?);
            require!(
                f.nodes[node].reachable && f.lock_entries.empty_at(id),
                "body_protocol_lock_emptiness_unavailable"
            );
            require!(
                graph
                    .exceptions
                    .iter()
                    .flatten()
                    .all(|&edge| !Graph::exception_possible(f, edge)
                        || Graph::exception_target(f, storage, edge) != Some(id)),
                "body_protocol_has_executable_exceptional_arrival"
            );
        }
    }
    // Only the already established call-to-marker link enters protocol content
    // from a readable node. All other normal protocol links stay internal or
    // select the initial body / resume merge; no hidden jump becomes fallthrough.
    // One mask answers both membership questions for every link.
    let mut in_initial = filled(f.nodes.len(), false)?;
    for &node in f.members(initial)? {
        in_initial[node.0 as usize] = true;
    }
    for link in &f.links {
        let from_initial = in_initial[link.from.0 as usize];
        let to_initial = in_initial[link.to.0 as usize];
        require!(
            !from_initial || to_initial || link.id == plan.normal,
            "body_initial_exit_bypasses_suspend_result"
        );
        require!(
            from_initial || !to_initial || link.id == decision.tests[0].selected,
            "body_initial_run_has_foreign_arrival"
        );
        if !protocol_nodes[link.from.0 as usize] && protocol_nodes[link.to.0 as usize] {
            require!(
                link.to
                    == f.regions[plan.header.0 as usize]
                        .entry
                        .ok_or(bug("protocol guard lost its entry"))?
                    && link.from == graph.op_node[plan.call.0 as usize]
                    && matches!(link.origin, LinkOrigin::WithinBlock(_)),
                "body_has_foreign_normal_protocol_arrival"
            );
        }
    }
    let mut sequence = Vec::new();
    push(&mut sequence, initial)?;
    for &body in &source.children[dispatch_index + 1..] {
        push(&mut sequence, body)?;
    }
    Ok(Ok(CoroutineBodyPlan {
        root,
        prefix: super::super::build::copy(prefix)?,
        dispatch: plan.dispatch,
        initial,
        invalid,
        resume: plan.resume,
        entry_check: OpId(entry_range.start),
        sequence,
        operations,
        implicit_locals,
    }))
}

fn definition_op(storage: &StorageMethod<'_>, local: LocalId) -> Option<OpId> {
    storage
        .facts()
        .definitions()
        .iter()
        .find(|d| d.materialization() == Materialization::Store(local))
        .map(|d| d.operation())
}
fn closed_values(
    storage: &StorageMethod<'_>,
    operations: &[ProtocolEvent],
    terminals: &[crate::p2::BlockId],
    copies: [crate::p5::PlanId; 2],
    locals: &[LocalId],
) -> bool {
    let flow = storage.execution();
    let touches = |use_: &crate::p5::UseAccess| {
        !use_
            .read()
            .is_ok_and(|read| read.local().is_none_or(|local| !locals.contains(&local)))
    };
    if flow
        .operations()
        .iter()
        .filter(|op| !operations.iter().any(|entry| entry.operation == op.id()))
        .any(|op| inputs(storage, op.id()).is_none_or(|uses| uses.iter().any(touches)))
    {
        return false;
    }
    if flow
        .blocks()
        .iter()
        .filter(|block| !terminals.contains(&block.id()) && !flow.terminal_reads(block).is_empty())
        .any(|block| {
            storage
                .facts()
                .terminal_uses(block.id())
                .is_none_or(|uses| uses.iter().any(touches))
        })
    {
        return false;
    }
    !storage
        .facts()
        .copy_plans()
        .iter()
        .filter(|copy| !copies.contains(&copy.id()))
        .any(|copy| {
            copy.assignments().iter().any(|assignment| {
                assignment.issue().is_some()
                    || assignment
                        .source()
                        .ok()
                        .and_then(StorageRead::local)
                        .is_some_and(|local| locals.contains(&local))
                    || assignment
                        .destination()
                        .ok()
                        .is_some_and(|local| locals.contains(&local))
            })
        })
}
fn invalid_arm(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    graph: &Graph,
    region: RegionId,
    entrance: LinkId,
) -> Result<Result<Vec<OpId>, ()>, RegionError> {
    let r = &f.regions[region.0 as usize];
    invalid_nodes(storage, f, graph, f.members(r.id)?, r.entry, entrance)
}
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
