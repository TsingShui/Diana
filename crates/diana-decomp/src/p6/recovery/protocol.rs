//! Projects established protocol associations into the ordinary control grammar.
//! Recognition stays in P6 suspension; this constructor runs no new graph solver.

use super::*;
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
) -> Result<RegionId, RegionError> {
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
    sequence(f, children)
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

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
) -> Result<(), RegionError> {
    let flow = f.coroutine_flow.take();
    let repeated = f.coroutine_loop.take();
    let single = f.coroutine_body.take();
    let built = (|| {
        let (source, abi, mut plan) = if let Some(p) = &flow {
            let mut calls = Vec::new();
            for s in &p.sites {
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
            let source = if let Some(c) = &p.cycle {
                next(f, c.implicit_latch);
                next(f, c.initial_header);
                cycle(
                    storage,
                    f,
                    g,
                    super::super::build::copy(&c.initial)?,
                    super::super::build::copy(&c.body)?,
                    super::super::build::copy(&c.tail)?,
                    c.condition,
                )?
            } else {
                sequence(f, super::super::build::copy(&p.source)?)?
            };
            (
                source,
                sequence(f, nodes(f, &p.abi)?)?,
                ProtocolPlan {
                    root: source,
                    calls,
                    operations: super::super::build::copy(&p.protocol_operations)?,
                    source_nodes: super::super::build::copy(&p.source_nodes)?,
                    abi_entries: super::super::build::copy(&p.abi_entries)?,
                    hidden_locals: Vec::new(),
                },
            )
        } else if let Some(p) = &repeated {
            let guard_node = p.guard;
            let terminal = f.nodes[guard_node.0 as usize].block;
            let normal = p.normal;
            let suspended = p.suspended;
            leaf_end(
                f,
                guard_node,
                BodyEnd::Annotated {
                    terminal,
                    mode: SourceMode::Origins,
                    normal,
                    suspended,
                },
            );
            for &node in p.initial.iter().chain(&p.body).chain(&p.tail) {
                f.controls[f.labels[node.0 as usize].owner.0 as usize].continuation =
                    Some(Continuation::None);
            }
            for &node in &p.abi {
                if f.nodes[node.0 as usize].terminal {
                    leaf_end(f, node, BodyEnd::Terminal(f.nodes[node.0 as usize].block));
                }
            }
            for &link in &p.normal_links {
                if !p.abi.contains(&f.links[link.0 as usize].from) {
                    next(f, link);
                }
            }
            let source = cycle(
                storage,
                f,
                g,
                nodes(f, &p.initial)?,
                nodes(f, &p.body)?,
                nodes(f, &p.tail)?,
                p.condition,
            )?;
            let mut source_nodes = super::super::build::copy(&p.initial)?;
            for &node in p
                .body
                .iter()
                .chain(&p.tail)
                .chain(std::iter::once(&p.condition.node))
            {
                push(&mut source_nodes, node)?;
            }
            (
                source,
                sequence(f, nodes(f, &p.abi)?)?,
                ProtocolPlan {
                    root: source,
                    calls: vec_one(ProtocolCall {
                        call: p.call,
                        continuation_argument: p.continuation_argument,
                        state: p.state,
                        guard: guard_node,
                        predicate: terminal,
                        normal,
                        suspended,
                        publication: None,
                    })?,
                    operations: super::super::build::copy(&p.protocol_operations)?,
                    source_nodes,
                    abi_entries: super::super::build::copy(&p.abi_entries)?,
                    hidden_locals: Vec::new(),
                },
            )
        } else if let Some(p) = &single {
            let s = *f
                .suspensions
                .first()
                .ok_or(bug("closed protocol lost its call association"))?;
            guard(f, s.control, s.header, s.normal_body)?;
            let mut source_only = vec_one(s.suspended_body)?;
            f.controls[s.control.0 as usize].source_only = std::mem::take(&mut source_only).into();
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
            f.controls[p.root.0 as usize].children = super::super::build::copy(&p.sequence)?.into();
            let mut abi_children = super::super::build::copy(&p.prefix)?;
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
            let mut operations = Vec::new();
            for op in &p.operations {
                push(&mut operations, op.operation)?;
            }
            let source_nodes = selected_nodes(f, p.root)?;
            (
                p.root,
                sequence(f, abi_children)?,
                ProtocolPlan {
                    root: p.root,
                    calls: vec_one(ProtocolCall {
                        call: s.call,
                        continuation_argument: s.continuation_argument,
                        state: s.state,
                        guard: guard_node,
                        predicate: s.predicate,
                        normal: s.normal,
                        suspended: s.suspended,
                        publication: Some((s.normal_copy, s.merged)),
                    })?,
                    operations,
                    source_nodes,
                    abi_entries: vec_one(
                        f.regions[p.initial.0 as usize]
                            .entry
                            .ok_or(bug("protocol initial body lost its entry"))?,
                    )?,
                    hidden_locals: super::super::build::copy(&p.implicit_locals)?,
                },
            )
        } else {
            return Ok(());
        };
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
        plan.root = sequence(f, children)?;
        f.protocol = Some(plan);
        Ok(())
    })();
    f.coroutine_flow = flow;
    f.coroutine_loop = repeated;
    f.coroutine_body = single;
    built
}
