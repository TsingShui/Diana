//! Matches a catch-all cleanup against the original normal exit before region
//! selection. Established P5 reads, copy-free cleanup and equal propagation are required;
//! branches and joins are matched as a graph without inventing storage bindings.

mod matching;
mod preparation;

use super::build::*;
use super::graph::Graph;
use super::*;
use crate::p2::{InvokeKind, OpId, OpKind, Terminal};
use crate::p5::{
    CopyLocation, Materialization, StorageMethod, StorageRead, ThrowSite, TransferSite,
};

struct Match {
    plan: FinallyPlan,
    preparation: Vec<NodeId>,
    follow: Option<NodeId>,
}

fn unhandled(f: &RegionFacts, site: ThrowSite) -> bool {
    f.throws.iter().find(|t| t.site == site).is_some_and(|t| {
        f.chains[t.chain.0 as usize]
            .clauses
            .iter()
            .all(|c| c.target == CatchTarget::Uncaught)
    })
}

fn operation_allowed(storage: &StorageMethod<'_>, f: &RegionFacts, op: OpId) -> bool {
    let operation = &storage.execution().operations()[op.0 as usize];
    let construction = match storage.facts().operation_emission(op) {
        Some(crate::p5::OperationEmission::Ordinary) => false,
        Some(crate::p5::OperationEmission::Construction { group, .. }) => {
            if storage
                .facts()
                .construction_groups()
                .get(group.0 as usize)
                .is_none_or(|group| !group.members().contains(&op))
            {
                return false;
            }
            true
        }
        None => return false,
    };
    if !(matches!(
        operation.kind(),
        OpKind::Move { .. }
            | OpKind::Const { .. }
            | OpKind::Unary { .. }
            | OpKind::Binary { .. }
            | OpKind::Compare { .. }
            | OpKind::ConstString { .. }
            | OpKind::Field { .. }
            | OpKind::Invoke {
                kind: InvokeKind::Static | InvokeKind::Virtual | InvokeKind::Interface,
                ..
            }
    ) || construction
        && matches!(
            operation.kind(),
            OpKind::NewInstance { .. }
                | OpKind::Invoke {
                    kind: InvokeKind::Direct,
                    ..
                }
        ))
    {
        return false;
    }
    if storage
        .facts()
        .operation_uses(op)
        .is_none_or(|u| u.is_err() || u.is_ok_and(|u| u.iter().any(|u| u.read().is_err())))
    {
        return false;
    }
    if operation.exceptional().is_none() {
        return !operation.effects().may_throw();
    }
    unhandled(f, ThrowSite::Operation(op))
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
                        // Selected-edge copies execute with the original
                        // source outlet, not with the target cleanup node.
                        // Body-to-cleanup copies remain before the finally;
                        // interior/exit copies would be folded and decline it.
                        nodes.contains(&l.from)
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
    let clauses = &f.chains[p.chain.0 as usize].clauses
        [p.clause_range.start as usize..p.clause_range.end as usize];
    let [clause] = clauses else {
        return Ok(None);
    };
    if p.outer_sync.is_some()
        || clause.catch.is_some()
        || clause.target != CatchTarget::Handler(*handler)
    {
        return Ok(None);
    }
    if p.body.iter().any(|node| {
        let n = &f.nodes[node.0 as usize];
        let terminal = storage.execution().blocks()[n.block.0 as usize].terminal();
        (n.terminal && matches!(terminal, Some(Terminal::Return { .. })))
            || (g.outgoing[node.0 as usize].is_empty()
                && !(n.terminal && matches!(terminal, Some(Terminal::Throw { .. }))))
    }) || f.throws.iter().any(|t| {
        p.body.contains(&t.node)
            && f.chains[t.chain.0 as usize].clauses.iter().any(|c| {
                match c.target {
                    // Nested catches stay in the body. Only this cleanup
                    // may receive an exceptional escape from it.
                    CatchTarget::Handler(n) => n != *handler && !p.body.contains(&n),
                    CatchTarget::Uncaught => true,
                }
            })
    }) {
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
    let Some(matching::Matched {
        preparation,
        entry,
        normal,
        operations: pairs,
        predicates,
        rethrows,
        terminals,
        follow: at,
    }) = matching::construct(storage, f, g, p, capture, def)?
    else {
        return Ok(None);
    };
    // An implicit pending rethrow cannot replace a site-specific outward
    // dispatch. Enclosing lexical delivery has not been established here.
    if rethrows
        .iter()
        .any(|rethrow| !unhandled(f, ThrowSite::Terminal(f.nodes[rethrow.0 as usize].block)))
    {
        return Ok(None);
    }
    if copies_touch(storage, f, g, &p.handler_bodies[0]) {
        return Ok(None);
    }
    // The capture is private protocol storage, not a value consumed by cleanup
    // or by ordinary code after this handler.
    if storage.facts().uses().iter().any(|u| match u.read() {
        Err(_) => true,
        Ok(read) => {
            read.local() == Some(local)
                && !matches!(u.site(), crate::p3::UseSite::Terminal { block, .. }
                if rethrows.iter().any(|rethrow| block == f.nodes[rethrow.0 as usize].block))
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
    if normal.is_empty()
        || at.is_some_and(|at| p.members.contains(&at) || normal.contains(&at))
        || copies_touch(storage, f, g, &normal)
        || f.tries
            .iter()
            .enumerate()
            .any(|(other, p)| other != index && normal.iter().any(|n| p.members.contains(n)))
    {
        return Ok(None);
    }
    let body_contains = |node| p.body.contains(&node) || preparation.contains(&node);
    for &node in &normal {
        if g.incoming[node.0 as usize].iter().any(|l| {
            let l = &f.links[l.0 as usize];
            !normal.contains(&l.from) && !body_contains(l.from)
        }) || g.exceptions.iter().flatten().any(|&e| {
            Graph::exception_possible(f, e) && Graph::exception_target(f, storage, e) == Some(node)
        }) {
            return Ok(None);
        }
    }
    // No additional normal outlet can bypass or re-enter the matched cleanup.
    if f.links
        .iter()
        .any(|l| body_contains(l.from) && !body_contains(l.to) && l.to != entry)
    {
        return Ok(None);
    }
    Ok(Some(Match {
        preparation,
        plan: FinallyPlan {
            normal,
            operations: pairs,
            predicates,
            capture,
            rethrows,
            terminals,
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
        for &node in &matched.preparation {
            push(&mut p.body, node)?;
            push(&mut p.members, node)?;
        }
        p.body.sort_unstable();
        for &node in &matched.plan.normal {
            push(&mut p.members, node)?;
        }
        p.members.sort_unstable();
        p.follow = matched.follow;
        #[cfg(feature = "trace-passes")]
        {
            let key = storage.execution().owner_facts().key();
            eprintln!(
                "[finally] dex={} class={} method={} candidate={} matched_pairs={} predicates={} capture={} rethrows={}",
                key.dex.0,
                key.class_def.0,
                storage.execution().method_id().0,
                p.id.0,
                matched.plan.operations.len(),
                matched.plan.predicates.len(),
                matched.plan.capture.0,
                matched.plan.rethrows.len()
            );
        }
        p.finally = Some(matched.plan);
    }
    Ok(())
}
