//! Matches a catch-all cleanup against the original normal exit before region
//! selection. Exact P5 reads, copy-free scopes and equal propagation are required;
//! shared tails and value joins remain explicit rather than inventing bindings.

use super::build::*;
use super::graph::Graph;
use super::*;
use crate::p2::{InvokeKind, OpId, OpKind, Terminal};
use crate::p5::{CopyLocation, Materialization, StorageMethod, StorageRead, TransferSite};

struct Match {
    plan: FinallyPlan,
    follow: NodeId,
}

fn plain_call(storage: &StorageMethod<'_>, f: &RegionFacts, op: OpId) -> bool {
    let operation = &storage.execution().operations()[op.0 as usize];
    if operation.normal().commit() != crate::p2::CommitRule::None
        || operation.result_pc().is_some()
        || operation.exceptional().is_none()
    {
        return false;
    }
    matches!(
        storage.execution().operations()[op.0 as usize].kind(),
        OpKind::Invoke {
            kind: InvokeKind::Static | InvokeKind::Virtual | InvokeKind::Interface,
            ..
        }
    ) && storage
        .facts()
        .operation_definitions(op)
        .is_some_and(|d| d.is_empty())
        && storage
            .facts()
            .operation_uses(op)
            .is_some_and(|u| u.is_ok_and(|u| u.iter().all(|u| u.read().is_ok())))
        && f.throws
            .iter()
            .find(|t| t.site == crate::p5::ThrowSite::Operation(op))
            .is_some_and(|t| {
                f.chains[t.chain.0 as usize]
                    .clauses
                    .iter()
                    .all(|c| c.target == CatchTarget::Uncaught)
            })
}

fn same_call(storage: &StorageMethod<'_>, f: &RegionFacts, a: OpId, b: OpId) -> bool {
    let flow = storage.execution();
    if flow.operations()[a.0 as usize].kind() != flow.operations()[b.0 as usize].kind()
        || flow.operations()[a.0 as usize].effects() != flow.operations()[b.0 as usize].effects()
        || flow.operations()[a.0 as usize].normal() != flow.operations()[b.0 as usize].normal()
        || !plain_call(storage, f, a)
        || !plain_call(storage, f, b)
    {
        return false;
    }
    let (Some(Ok(a)), Some(Ok(b))) = (
        storage.facts().operation_uses(a),
        storage.facts().operation_uses(b),
    ) else {
        return false;
    };
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.read() == b.read() && a.initialization() == b.initialization())
}

fn copies_touch(storage: &StorageMethod<'_>, f: &RegionFacts, g: &Graph, nodes: &[NodeId]) -> bool {
    storage
        .facts()
        .copy_plans()
        .iter()
        .any(|p| match p.location() {
            CopyLocation::Transfer { site, .. } => match site {
                TransferSite::InitialEntry => nodes.contains(&NodeId(0)),
                TransferSite::NormalEdge(e) => {
                    if let EdgeRoute::Normal(l) = f.edges[e.0 as usize].route {
                        let l = &f.links[l.0 as usize];
                        nodes.contains(&l.from) || nodes.contains(&l.to)
                    } else {
                        false
                    }
                }
                TransferSite::AfterCommit(op) | TransferSite::BeforeOperation(op) => {
                    nodes.contains(&g.op_node[op.0 as usize])
                }
                TransferSite::BeforeTerminal(b) => nodes.contains(&g.terminal_node[b.0 as usize]),
            },
            CopyLocation::BeforeThrow(crate::p5::ThrowSite::Operation(op)) => {
                nodes.contains(&g.op_node[op.0 as usize])
            }
            CopyLocation::BeforeThrow(crate::p5::ThrowSite::Terminal(b)) => {
                nodes.contains(&g.terminal_node[b.0 as usize])
            }
            CopyLocation::HandlerEntry { block, .. } => nodes.contains(&Graph::entry(f, block)),
        })
}

fn candidate(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    index: usize,
) -> Result<Option<Match>, RegionError> {
    let p = &f.tries[index];
    let [handler] = p.handlers.as_slice() else {
        return Ok(None);
    };
    let clauses = &f.chains[p.chain.0 as usize].clauses;
    if p.outer_sync.is_some()
        || p.clause_range.start != 0
        || p.clause_range.end != 1
        || clauses.len() != 1
        || clauses[0].catch.is_some()
    {
        return Ok(None);
    }
    if p.body.iter().any(|node| {
        let n = &f.nodes[node.0 as usize];
        let terminal = storage.execution().blocks()[n.block.0 as usize].terminal();
        (n.terminal && matches!(terminal, Some(Terminal::Return { .. })))
            || (g.outgoing[node.0 as usize].is_empty()
                && !(n.terminal && matches!(terminal, Some(Terminal::Throw { .. }))))
    }) || f
        .throws
        .iter()
        .any(|t| p.body.contains(&t.node) && f.chains[t.chain.0 as usize].clauses != clauses[..1])
    {
        return Ok(None);
    }
    let Some(capture) = f
        .handlers
        .iter()
        .find(|h| h.node == *handler)
        .filter(|h| h.receives.is_empty())
        .and_then(|h| h.capture)
    else {
        return Ok(None);
    };
    let Some(def) = storage
        .facts()
        .operation_definitions(capture)
        .and_then(|d| if let [d] = d { Some(d) } else { None })
    else {
        return Ok(None);
    };
    let Materialization::ExceptionCapture(local) = def.materialization() else {
        return Ok(None);
    };
    let mut seen = filled(f.nodes.len(), false)?;
    let mut cleanup = Vec::new();
    let mut at = *handler;
    let rethrow = loop {
        if seen[at.0 as usize] || !p.handler_bodies[0].contains(&at) || !f.lock_entries.empty_at(at)
        {
            return Ok(None);
        }
        seen[at.0 as usize] = true;
        let n = &f.nodes[at.0 as usize];
        for op in n.operations.clone().map(OpId) {
            if op != capture {
                if !plain_call(storage, f, op) {
                    return Ok(None);
                }
                push(&mut cleanup, op)?;
            }
        }
        if n.terminal {
            match storage.execution().blocks()[n.block.0 as usize].terminal() {
                Some(Terminal::Throw { .. }) => {
                    let Some([u]) = storage.facts().terminal_uses(n.block) else {
                        return Ok(None);
                    };
                    if !matches!(u.read(), Ok(StorageRead::Value { local: l, state,
                        view: crate::p4::Interpretation::Direct(crate::p4::Family::Reference), .. })
                        if l == local && state == def.state())
                    {
                        return Ok(None);
                    }
                    break at;
                }
                Some(Terminal::Goto { .. }) => {}
                _ => return Ok(None),
            }
        }
        let [link] = g.outgoing[at.0 as usize].as_slice() else {
            return Ok(None);
        };
        at = f.links[link.0 as usize].to;
    };
    if cleanup.is_empty()
        || p.handler_bodies[0].iter().any(|n| !seen[n.0 as usize])
        || copies_touch(storage, f, g, &p.handler_bodies[0])
    {
        return Ok(None);
    }
    // The capture is private protocol storage, not a value consumed by cleanup
    // or by ordinary code after this handler.
    if storage.facts().uses().iter().any(|u| match u.read() {
        Err(_) => true,
        Ok(read) => {
            read.local() == Some(local)
                && !matches!(u.site(), crate::p3::UseSite::Terminal { block, .. }
                if block == f.nodes[rethrow.0 as usize].block)
        }
    }) {
        return Ok(None);
    }
    if storage.facts().copy_plans().iter().any(|p| {
        !p.is_complete()
            || p.assignments().iter().any(|a| {
                a.destination().ok() == Some(local)
                    || a.source().ok().and_then(|r| r.local()) == Some(local)
            })
    }) {
        return Ok(None);
    }
    let Some(mut at) = p.follow else {
        return Ok(None);
    };
    let mut normal = Vec::new();
    let mut pairs = Vec::new();
    let mut next_op = 0;
    while next_op < cleanup.len() {
        if seen[at.0 as usize]
            || p.members.contains(&at)
            || !f.lock_entries.empty_at(at)
            || f.handlers.iter().any(|h| h.node == at)
        {
            return Ok(None);
        }
        seen[at.0 as usize] = true;
        let n = &f.nodes[at.0 as usize];
        if n.terminal {
            return Ok(None);
        }
        for op in n.operations.clone().map(OpId) {
            let Some(&canonical) = cleanup.get(next_op) else {
                return Ok(None);
            };
            if !same_call(storage, f, op, canonical) {
                return Ok(None);
            }
            push(&mut pairs, (op, canonical))?;
            next_op += 1;
        }
        push(&mut normal, at)?;
        let [link] = g.outgoing[at.0 as usize].as_slice() else {
            return Ok(None);
        };
        at = f.links[link.0 as usize].to;
    }
    if normal.is_empty()
        || p.members.contains(&at)
        || normal.contains(&at)
        || copies_touch(storage, f, g, &normal)
        || f.tries
            .iter()
            .enumerate()
            .any(|(other, p)| other != index && normal.iter().any(|n| p.members.contains(n)))
    {
        return Ok(None);
    }
    for &node in &normal {
        if g.incoming[node.0 as usize].iter().any(|l| {
            let l = &f.links[l.0 as usize];
            !normal.contains(&l.from) && !p.body.contains(&l.from)
        }) || g.exceptions.iter().flatten().any(|&e| {
            Graph::exception_possible(f, e) && Graph::exception_target(f, storage, e) == Some(node)
        }) {
            return Ok(None);
        }
    }
    // No additional normal outlet can bypass or re-enter the matched cleanup.
    if f.links
        .iter()
        .any(|l| p.body.contains(&l.from) && !p.body.contains(&l.to) && Some(l.to) != p.follow)
    {
        return Ok(None);
    }
    Ok(Some(Match {
        plan: FinallyPlan {
            normal,
            operations: pairs,
            capture,
            rethrow,
        },
        follow: at,
    }))
}

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
) -> Result<(), RegionError> {
    for index in 0..f.tries.len() {
        let Some(matched) = candidate(storage, f, g, index)? else {
            continue;
        };
        let p = &mut f.tries[index];
        for &node in &matched.plan.normal {
            push(&mut p.members, node)?;
        }
        p.members.sort_unstable();
        p.follow = Some(matched.follow);
        #[cfg(feature = "trace-passes")]
        {
            let key = storage.execution().owner_facts().key();
            eprintln!(
                "[finally] dex={} class={} method={} candidate={} matched_pairs={} capture={} rethrow={}",
                key.dex.0,
                key.class_def.0,
                storage.execution().method_id().0,
                p.id.0,
                matched.plan.operations.len(),
                matched.plan.capture.0,
                matched.plan.rethrow.0
            );
        }
        p.finally = Some(matched.plan);
    }
    Ok(())
}
