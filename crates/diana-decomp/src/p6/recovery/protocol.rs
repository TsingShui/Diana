//! Projects established protocol associations into the ordinary control grammar.
//! Recognition stays in P6 suspension; this constructor runs no new graph solver.

use super::super::suspension::Nomination;
use super::*;
use crate::p2::OpId;
use crate::recovery::{BodyEnd, ControlRecord, SourceMode};

fn nodes(f: &RegionFacts, source: &[NodeId]) -> Result<Vec<RegionId>, RegionError> {
    let mut result = Vec::new();
    for &node in source {
        push(&mut result, f.labels[node.0 as usize].owner)?;
    }
    Ok(result)
}
fn sequence(f: &mut RegionFacts, children: Vec<RegionId>) -> Result<RegionId, RegionError> {
    append(f, ControlLayout::Sequence, children)
}
fn append(
    f: &mut RegionFacts,
    layout: ControlLayout,
    children: Vec<RegionId>,
) -> Result<RegionId, RegionError> {
    let id = RegionId(super::super::build::id(f.controls.len())?);
    push(
        &mut f.controls,
        ControlRecord {
            layout,
            children: children.into(),
            source: None,
            source_only: Vec::new().into(),
            end: None,
            continuation: Some(Continuation::None),
            entry: None,
            follow: None,
        },
    )?;
    Ok(id)
}
fn scope(
    f: &mut RegionFacts,
    body: RegionId,
    mode: SourceMode,
    heading: &'static str,
) -> Result<RegionId, RegionError> {
    append(
        f,
        ControlLayout::Scope {
            body,
            mode,
            heading: Some(heading),
        },
        vec_one(body)?,
    )
}
fn vec_one<T>(value: T) -> Result<Vec<T>, RegionError> {
    let mut result = Vec::new();
    push(&mut result, value)?;
    Ok(result)
}
fn cycle(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
    initial: Vec<RegionId>,
    body: Vec<RegionId>,
    tail: Vec<RegionId>,
    test: LoopCondition,
) -> Result<(RegionId, RegionId), RegionError> {
    let predicate_control = f.labels[test.node.0 as usize].owner;
    f.controls[predicate_control.0 as usize].end = Some(BodyEnd::None);
    f.controls[predicate_control.0 as usize].continuation = Some(Continuation::None);
    let body = sequence(f, body)?;
    let placement = LoopPlacement::While {
        predicate: predicate_control,
        test: predicate(storage, f, g, test.node, Some(test.repeat))?,
        compact: true,
    };
    let mut children = vec_one(body)?;
    push(&mut children, predicate_control)?;
    let loop_control = append(
        f,
        ControlLayout::Loop {
            continue_target: test.node,
            body,
            placement,
        },
        children,
    )?;
    let mut children = initial;
    push(&mut children, loop_control)?;
    for child in tail {
        push(&mut children, child)?;
    }
    Ok((sequence(f, children)?, loop_control))
}
fn leaf_end(f: &mut RegionFacts, node: NodeId, end: BodyEnd) {
    let id = f.labels[node.0 as usize].owner;
    f.controls[id.0 as usize].end = Some(end);
    f.controls[id.0 as usize].continuation = Some(Continuation::None);
}
fn next(f: &mut RegionFacts, link: LinkId) {
    f.link_plans[link.0 as usize].action = TransferAction::Next(f.links[link.0 as usize].to);
}
fn goto(f: &mut RegionFacts, link: LinkId) {
    f.link_plans[link.0 as usize].action =
        TransferAction::Goto(LabelId(f.links[link.0 as usize].to.0));
}
fn guard(
    f: &mut RegionFacts,
    control: RegionId,
    header: RegionId,
    normal: RegionId,
) -> Result<(), RegionError> {
    let record = &mut f.controls[control.0 as usize];
    record.layout = ControlLayout::Sequence;
    let mut children = vec_one(header)?;
    push(&mut children, normal)?;
    record.children = children.into();
    record.continuation = Some(Continuation::None);
    Ok(())
}

/// Leaves follow the selected static ownership, never the original root's ABI
/// membership. The installed grammar is finite; records name earlier children.
fn selected_nodes(f: &RegionFacts, root: RegionId) -> Result<Vec<NodeId>, RegionError> {
    let mut result = Vec::new();
    let mut work = vec_one(root)?;
    while let Some(id) = work.pop() {
        let record = &f.controls[id.0 as usize];
        if let ControlLayout::Block(node) = record.layout {
            push(&mut result, node)?;
        } else {
            for &child in record.children.iter().rev() {
                push(&mut work, child)?;
            }
        }
    }
    Ok(result)
}

fn events(source: Vec<OpId>) -> Result<Vec<ProtocolEvent>, RegionError> {
    let mut result = Vec::new();
    for operation in source {
        push(
            &mut result,
            ProtocolEvent {
                operation,
                role: ProtocolRole::Abi,
            },
        )?;
    }
    Ok(result)
}

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
    nomination: Option<Nomination>,
) -> Result<(), RegionError> {
    let Some(nomination) = nomination else {
        return Ok(());
    };
    let (
        source,
        abi,
        calls,
        operations,
        abi_entries,
        hidden_locals,
        iteration,
        forwarding,
        entry_check,
    ) = match nomination {
        Nomination::Flow(p) => {
            let mut calls = Vec::new();
            for s in p.sites {
                push(
                    &mut calls,
                    ProtocolCall {
                        call: s.call,
                        continuation_argument: s.continuation_argument,
                        state: s.state,
                        guard: s.guard,
                        predicate: f.nodes[s.guard.0 as usize].block,
                        normal: s.normal,
                        suspended: s.suspended,
                        publication: None,
                        resume: s.resume,
                        spills: s.spills,
                    },
                )?;
            }
            for guard_plan in &p.guards {
                guard(f, guard_plan.control, guard_plan.header, guard_plan.normal)?;
            }
            for s in &calls {
                leaf_end(
                    f,
                    s.guard,
                    BodyEnd::Annotated {
                        terminal: s.predicate,
                        mode: SourceMode::Abi,
                        normal: s.normal,
                        suspended: s.suspended,
                    },
                );
                next(f, s.normal);
            }
            for &node in p.abi.iter().chain(&p.explicit_predicates) {
                if f.nodes[node.0 as usize].terminal {
                    leaf_end(f, node, BodyEnd::Terminal(f.nodes[node.0 as usize].block));
                }
            }
            for &(id, target) in &p.completions {
                f.controls[id.0 as usize].continuation =
                    Some(target.map_or(Continuation::None, |node| {
                        Continuation::Goto(LabelId(node.0))
                    }));
            }
            for &link in &p.retired_transfers {
                next(f, link);
            }
            for &node in &p.abi {
                for &link in &g.outgoing[node.0 as usize] {
                    next(f, link);
                }
            }
            for &link in p.gotos.iter().chain(&p.abi_gotos) {
                goto(f, link);
            }
            let (source, iteration) = if let Some(c) = p.cycle {
                next(f, c.implicit_latch);
                next(f, c.initial_header);
                let (source, control) =
                    cycle(storage, f, g, c.initial, c.body, c.tail, c.condition)?;
                (
                    source,
                    Some(ProtocolIteration {
                        control,
                        condition: c.condition,
                        implicit: Some((c.implicit_latch, c.initial_header)),
                    }),
                )
            } else {
                (sequence(f, p.source)?, None)
            };
            (
                source,
                sequence(f, nodes(f, &p.abi)?)?,
                calls,
                events(p.protocol_operations)?,
                p.abi_entries,
                Vec::new(),
                iteration,
                p.forwarding,
                p.entry_check,
            )
        }
        Nomination::Body(p) => {
            let s = p.site;
            guard(f, s.control, s.header, s.normal_body)?;
            f.controls[s.control.0 as usize].source_only = vec_one(s.suspended_body)?.into();
            let guard_node = g.terminal_node[s.predicate.0 as usize];
            leaf_end(
                f,
                guard_node,
                BodyEnd::Annotated {
                    terminal: s.predicate,
                    mode: SourceMode::Origins,
                    normal: s.normal,
                    suspended: s.suspended,
                },
            );
            f.copies[s.normal_copy.0 as usize].owner = CopyOwner::Publication(s.call);
            next(f, s.normal);
            f.controls[p.root.0 as usize].children = p.sequence.into();
            let mut abi_children = p.prefix;
            let decision = f.regions[p.dispatch.0 as usize]
                .decision
                .as_ref()
                .ok_or(bug("protocol dispatch lost its established tests"))?;
            let mut headers = Vec::new();
            for test in &decision.tests {
                push(&mut headers, (test.header, test.predicate))?;
            }
            for (header, node) in headers {
                push(&mut abi_children, header)?;
                leaf_end(f, node, BodyEnd::Terminal(f.nodes[node.0 as usize].block));
                f.controls[header.0 as usize].continuation = Some(Continuation::None);
            }
            push(&mut abi_children, p.resume)?;
            push(&mut abi_children, p.invalid)?;
            let calls = vec_one(ProtocolCall {
                call: s.call,
                continuation_argument: s.continuation_argument,
                state: s.state,
                guard: guard_node,
                predicate: s.predicate,
                normal: s.normal,
                suspended: s.suspended,
                publication: Some((s.normal_copy, s.merged)),
                resume: super::super::build::copy(f.members(p.resume)?)?,
                spills: Vec::new(),
            })?;
            let entries = vec_one(
                f.regions[p.initial.0 as usize]
                    .entry
                    .ok_or(bug("protocol initial body lost its entry"))?,
            )?;
            (
                p.root,
                sequence(f, abi_children)?,
                calls,
                p.operations,
                entries,
                p.implicit_locals,
                None,
                None,
                p.entry_check,
            )
        }
    };
    let source_nodes = selected_nodes(f, source)?;
    let abi = scope(
        f,
        abi,
        SourceMode::Abi,
        "// Diana: ABI entry/reentry, resume and invalid-state paths",
    )?;
    let source = scope(
        f,
        source,
        SourceMode::Code,
        "// Diana: inferred suspend body; original ABI facts retained above",
    )?;
    let mut children = vec_one(abi)?;
    push(&mut children, source)?;
    let root = sequence(f, children)?;
    f.protocol = Some(ProtocolPlan {
        root,
        calls,
        operations,
        source_nodes,
        abi_entries,
        hidden_locals,
        iteration,
        forwarding,
        entry_check,
    });
    Ok(())
}
