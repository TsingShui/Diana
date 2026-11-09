//! Surface facts over the selected regions. These consume the established lock,
//! handler, storage and transfer products without another graph or lock analysis.
use super::build::push;
use super::graph::Graph;
use super::*;
use crate::p2::{OpId, OpKind, Terminal};
use crate::p5::{
    CopyLocation, Materialization, StorageMethod, StorageRead, ThrowSite, TransferSite,
};

fn node(g: &Graph, site: crate::p3::UseSite) -> NodeId {
    match site {
        crate::p3::UseSite::Operation { operation, .. } => g.op_node[operation.0 as usize],
        crate::p3::UseSite::Terminal { block, .. } => g.terminal_node[block.0 as usize],
    }
}
fn private_capture(
    storage: &StorageMethod<'_>,
    g: &Graph,
    capture: OpId,
    inside: &[NodeId],
) -> bool {
    let Some([definition]) = storage.facts().operation_definitions(capture) else {
        return false;
    };
    let Materialization::ExceptionCapture(local) = definition.materialization() else {
        return false;
    };
    for usage in storage.facts().uses() {
        if (usage.state() == definition.state()
            || usage.original_source().ok().and_then(StorageRead::local) == Some(local))
            && (!inside.contains(&node(g, usage.site()))
                || !matches!(usage.read(), Ok(StorageRead::Value { local: l, state, .. }) if l == local && state == definition.state()))
        {
            return false;
        }
    }
    if storage
        .facts()
        .transfers()
        .iter()
        .any(|t| t.source() == definition.state() && t.destination_state() != definition.state())
    {
        return false;
    }
    !storage.facts().copy_plans().iter().any(|p| {
        let at_capture = matches!(p.location(),
            CopyLocation::Transfer { site: TransferSite::BeforeOperation(op) | TransferSite::AfterCommit(op), .. }
                | CopyLocation::BeforeThrow(ThrowSite::Operation(op)) if op == capture);
        at_capture || p.assignments().iter().any(|a| a.destination().ok() == Some(local)
            || a.source().ok().and_then(StorageRead::local) == Some(local))
    })
}
fn ancestor(f: &RegionFacts, mut child: RegionId, parent: RegionId) -> bool {
    while let Some(at) = f.parents[child.0 as usize] {
        if at == parent {
            return true;
        }
        child = at;
    }
    false
}
fn propagation(f: &RegionFacts, region: RegionId, terminal: NodeId) -> bool {
    let Some(site) = f
        .throws
        .iter()
        .find(|t| t.node == terminal && matches!(t.site, ThrowSite::Terminal(_)))
    else {
        return false;
    };
    site.edges
        .iter()
        .zip(&f.chains[site.chain.0 as usize].clauses)
        .all(|(edge, clause)| {
            if clause.target == CatchTarget::Uncaught {
                return true;
            }
            let EdgeRoute::Exception {
                delivery: ExceptionDelivery::Catch(id),
                ..
            } = f.edges[edge.0 as usize].route
            else {
                return false;
            };
            f.regions
                .iter()
                .find(|r| matches!(r.kind, RegionKind::Try(t) if t == id))
                .is_some_and(|parent| ancestor(f, region, parent.id))
        })
}
fn implicit(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    scope: &SyncPlan,
    region: RegionId,
) -> Result<Option<ImplicitMonitor>, RegionError> {
    if scope.separation.is_some()
        || scope
            .exits
            .iter()
            .filter(|e| matches!(e.kind, SyncExitKind::Follow(_)))
            .count()
            != 1
    {
        return Ok(None);
    }
    let mut cleanups = Vec::new();
    let mut captures = Vec::new();
    let mut rethrows = Vec::new();
    for exit in &scope.exits {
        if exit.before.iter().chain(&exit.after).any(|id| {
            let p = &storage.facts().copy_plans()[id.0 as usize];
            !p.is_complete() || !p.steps().is_empty()
        }) {
            return Ok(None);
        }
        let SyncExitKind::Rethrow {
            terminal,
            capture,
            exception,
        } = exit.kind
        else {
            if !matches!(exit.kind, SyncExitKind::Follow(_)) || !exit.routing.is_empty() {
                return Ok(None);
            }
            continue;
        };
        let Some([definition]) = storage.facts().operation_definitions(capture) else {
            return Ok(None);
        };
        if !matches!(exception, StorageRead::Value { state, view: crate::p4::Interpretation::Direct(crate::p4::Family::Reference), .. } if state == definition.state())
            || !private_capture(storage, g, capture, &[terminal])
            || !propagation(f, region, terminal)
        {
            return Ok(None);
        }
        let handler = g.op_node[capture.0 as usize];
        let Some(plan) = f.tries.iter().find(|p| {
            p.handlers.last() == Some(&handler)
                && p.finally.is_none()
                && p.members.iter().all(|n| scope.body.contains(n))
                && f.regions.iter().any(|r| {
                    matches!(r.kind, RegionKind::Try(id) if id == p.id) && ancestor(f, r.id, region)
                })
        }) else {
            return Ok(None);
        };
        // Each executable entrance to the hidden cleanup must already have
        // natural delivery from the shared selected protection producer.
        if f.throws.iter().any(|site| site.edges.iter().zip(&f.chains[site.chain.0 as usize].clauses).any(|(edge, clause)| {
            clause.target == CatchTarget::Handler(handler) && Graph::exception_possible(f, *edge)
                && !matches!(f.edges[edge.0 as usize].route, EdgeRoute::Exception { delivery: ExceptionDelivery::Catch(id), .. } if id == plan.id)
        })) { return Ok(None); }
        let index = plan.handlers.len() - 1;
        let range = &plan.handler_clauses[index];
        let clauses =
            &f.chains[plan.chain.0 as usize].clauses[range.start as usize..range.end as usize];
        if !matches!(clauses, [clause] if clause.catch.is_none() && clause.target == CatchTarget::Handler(handler))
        {
            return Ok(None);
        }
        for &n in &plan.handler_bodies[index] {
            let part = &f.nodes[n.0 as usize];
            if part.operations.clone().any(|id| {
                let op = OpId(id);
                op != capture
                    && op != exit.release
                    && !matches!(
                        storage.execution().operations()[id as usize].kind(),
                        OpKind::Nop
                    )
            }) {
                return Ok(None);
            }
            if part.terminal {
                let terminal_kind = storage.execution().blocks()[part.block.0 as usize].terminal();
                if if n == terminal {
                    !matches!(terminal_kind, Some(Terminal::Throw { .. }))
                } else {
                    !matches!(terminal_kind, Some(Terminal::Goto { .. }))
                } {
                    return Ok(None);
                }
            }
        }
        // Only the capture and exact pending rethrow are protocol. Any original
        // handler copy or extra entry would require executable handler work.
        if storage.facts().copy_plans().iter().any(|p| {
            let touched = match p.location() {
                CopyLocation::HandlerEntry { block, .. } => {
                    plan.handler_bodies[index].contains(&Graph::entry(f, block))
                }
                CopyLocation::BeforeThrow(ThrowSite::Operation(op))
                | CopyLocation::Transfer {
                    site: TransferSite::BeforeOperation(op) | TransferSite::AfterCommit(op),
                    ..
                } => plan.handler_bodies[index].contains(&g.op_node[op.0 as usize]),
                CopyLocation::BeforeThrow(ThrowSite::Terminal(block))
                | CopyLocation::Transfer {
                    site: TransferSite::BeforeTerminal(block),
                    ..
                } => plan.handler_bodies[index].contains(&g.terminal_node[block.0 as usize]),
                CopyLocation::Transfer {
                    site: TransferSite::NormalEdge(edge),
                    ..
                } => f
                    .edges
                    .get(edge.0 as usize)
                    .is_some_and(|edge| match edge.route {
                        EdgeRoute::Normal(link) => {
                            plan.handler_bodies[index].contains(&f.links[link.0 as usize].from)
                        }
                        _ => false,
                    }),
                _ => false,
            };
            touched && (!p.is_complete() || !p.steps().is_empty())
        }) {
            return Ok(None);
        }
        let cleanup = (plan.id, super::build::id(index)?);
        if !cleanups.contains(&cleanup) {
            push(&mut cleanups, cleanup)?;
        }
        push(&mut captures, capture)?;
        push(&mut rethrows, terminal)?;
    }
    if cleanups.is_empty() {
        return Ok(None);
    }
    Ok(Some(ImplicitMonitor {
        cleanups,
        captures,
        rethrows,
    }))
}
pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
) -> Result<(), RegionError> {
    for i in 0..f.tries.len() {
        let p = &f.tries[i];
        let mut bindings = Vec::new();
        let selected = f
            .regions
            .iter()
            .any(|r| matches!(r.kind, RegionKind::Try(id) if id == p.id));
        for (index, handler) in p.handlers.iter().enumerate() {
            let capture = f
                .handlers
                .iter()
                .find(|h| h.node == *handler)
                .and_then(|h| h.capture)
                .filter(|&op| {
                    selected
                        && p.finally.is_none()
                        && private_capture(storage, g, op, &p.handler_bodies[index])
                });
            push(&mut bindings, capture)?;
        }
        f.tries[i].bindings = bindings;
    }
    for i in 0..f.synchronized.len() {
        let scope = &f.synchronized[i];
        let Some(region) = f
            .regions
            .iter()
            .find(|r| matches!(r.kind, RegionKind::Synchronized(id) if id == scope.id))
            .map(|r| r.id)
        else {
            continue;
        };
        let surface = implicit(storage, f, g, scope, region)?;
        f.synchronized[i].implicit = surface;
    }
    Ok(())
}
