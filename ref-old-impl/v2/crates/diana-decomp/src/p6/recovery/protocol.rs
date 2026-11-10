//! Projects established protocol associations into the ordinary control grammar.
//! Recognition stays in P6 suspension; this constructor runs no new graph solver.

use super::super::suspension::ProtocolNomination;
use super::*;
use crate::recovery::{BodyEnd, ControlRecord, SourceMode};

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

fn private_marker(f: &RegionFacts, calls: &[ProtocolCall], site: &ProtocolCall) -> bool {
    site.publication.is_some()
        && calls
            .iter()
            .filter(|call| {
                f.links[call.suspended.0 as usize].to == f.links[site.suspended.0 as usize].to
            })
            .count()
            == 1
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

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
    nomination: Option<ProtocolNomination>,
) -> Result<(), RegionError> {
    let Some(p) = nomination else {
        return Ok(());
    };
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
                publication: s.publication,
                resume: s.resume,
                spills: s.spills,
            },
        )?;
    }
    for guard_plan in &p.guards {
        guard(f, guard_plan.control, guard_plan.header, guard_plan.normal)?;
    }
    for s in &calls {
        if let Some((copy, _)) = s.publication {
            f.copies[copy.0 as usize].owner = CopyOwner::Publication(s.call);
        }
        if private_marker(f, &calls, s) {
            let marker = f.labels[f.links[s.suspended.0 as usize].to.0 as usize].owner;
            let header = f.labels[s.guard.0 as usize].owner;
            f.controls[header.0 as usize].source_only = vec_one(marker)?.into();
        }
        leaf_end(
            f,
            s.guard,
            BodyEnd::Annotated {
                terminal: s.predicate,
                mode: if private_marker(f, &calls, s) {
                    SourceMode::Origins
                } else {
                    SourceMode::Abi
                },
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
        f.controls[id.0 as usize].continuation = Some(target.map_or(Continuation::None, |node| {
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
        let (source, control) = cycle(storage, f, g, c.initial, c.body, c.tail, c.condition)?;
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
    let mut abi = Vec::new();
    for &node in &p.abi {
        if !calls
            .iter()
            .any(|s| private_marker(f, &calls, s) && f.links[s.suspended.0 as usize].to == node)
        {
            push(&mut abi, f.labels[node.0 as usize].owner)?;
        }
    }
    let abi = sequence(f, abi)?;
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
        operations: p.operations,
        source_nodes,
        abi_entries: p.abi_entries,
        hidden_locals: p.hidden_locals,
        iteration,
        forwarding: p.forwarding,
        entry_check: p.entry_check,
    });
    Ok(())
}
