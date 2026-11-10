//! Establishes pure normal body completion before the handler's FIRST cleanup
//! event. The paired matcher owns that boundary; a failed match never searches
//! for a later event or discards preparation to make the cleanup align.

use super::super::build::{bug, filled, id, push};
use super::super::graph::Graph;
use super::super::{NodeId, RegionError, RegionFacts, TryPlan};
use crate::p2::{OpKind, Terminal};
use crate::p5::{OperationEmission, StorageMethod};

pub(super) struct Preparation {
    pub entry: NodeId,
    pub nodes: Vec<NodeId>,
}

fn pure(storage: &StorageMethod<'_>, f: &RegionFacts, g: &Graph, node: NodeId) -> bool {
    if node.0 == 0 || g.outgoing[node.0 as usize].is_empty() {
        return false;
    }
    let n = &f.nodes[node.0 as usize];
    for operation in n.operations.clone() {
        let op = &storage.execution().operations()[operation as usize];
        if !matches!(
            op.kind(),
            OpKind::Nop
                | OpKind::Move { .. }
                | OpKind::Const { .. }
                | OpKind::Unary { .. }
                | OpKind::Binary { .. }
                | OpKind::Compare { .. }
        ) || op.effects().may_throw()
            || op.exceptional().is_some()
            || !matches!(
                storage.facts().operation_emission(op.id()),
                Some(OperationEmission::Ordinary)
            )
            || storage.facts().operation_uses(op.id()).is_none_or(|uses| {
                uses.is_err() || uses.is_ok_and(|uses| uses.iter().any(|u| u.read().is_err()))
            })
        {
            return false;
        }
    }
    if n.terminal {
        match storage.execution().blocks()[n.block.0 as usize].terminal() {
            Some(Terminal::Goto { .. }) => {}
            Some(Terminal::Branch { .. } | Terminal::Switch { .. }) => {
                if storage
                    .facts()
                    .terminal_uses(n.block)
                    .is_none_or(|uses| uses.iter().any(|u| u.read().is_err()))
                {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    plan: &TryPlan,
    mut first_event: impl FnMut(NodeId) -> bool,
) -> Result<Option<Preparation>, RegionError> {
    let Some(start) = plan.follow else {
        return Ok(None);
    };
    let mut body = filled(f.nodes.len(), false)?;
    for node in &plan.body {
        body[node.0 as usize] = true;
    }
    if body[start.0 as usize]
        || f.links
            .iter()
            .any(|link| body[link.from.0 as usize] && !body[link.to.0 as usize] && link.to != start)
    {
        return Ok(None);
    }
    let mut blocked = filled(f.nodes.len(), false)?;
    for node in &plan.members {
        blocked[node.0 as usize] = !body[node.0 as usize];
    }
    for &edge in g.exceptions.iter().flatten() {
        if Graph::exception_possible(f, edge)
            && let Some(target) = Graph::exception_target(f, storage, edge)
        {
            blocked[target.0 as usize] = true;
        }
    }
    for group in storage.facts().construction_groups() {
        for op in group.members() {
            blocked[g.op_node[op.0 as usize].0 as usize] = true;
        }
    }
    for scope in &f.synchronized {
        for node in &scope.members {
            blocked[node.0 as usize] = true;
        }
    }
    let mut pending = filled(f.nodes.len(), 0usize)?;
    let mut outside = filled(f.nodes.len(), 0usize)?;
    for link in &f.links {
        let from = link.from.0 as usize;
        let to = link.to.0 as usize;
        if !body[from] {
            pending[to] = pending[to]
                .checked_add(1)
                .ok_or(bug("pure preparation predecessor overflow"))?;
        } else if !body[to] {
            outside[to] = outside[to]
                .checked_add(1)
                .ok_or(bug("pure preparation frontier overflow"))?;
        }
    }
    if outside[start.0 as usize] == 0 {
        return Ok(None);
    }
    let mut entries = 1usize;
    let mut entry_xor = start.0 as usize;
    let mut queue = Vec::new();
    let mut nodes = Vec::new();
    if pending[start.0 as usize] == 0 {
        push(&mut queue, start)?;
    }
    loop {
        // Cardinality one identifies this exact frontier, not a hash proof.
        // The first admitted event is the ONLY cleanup candidate.
        if entries == 1 && entry_xor != 0 && first_event(NodeId(id(entry_xor)?)) {
            return Ok(Some(Preparation {
                entry: NodeId(id(entry_xor)?),
                nodes,
            }));
        }
        let Some(node) = queue.pop() else {
            return Ok(None);
        };
        let n = node.0 as usize;
        if body[n] || blocked[n] || !pure(storage, f, g, node) {
            continue;
        }
        body[n] = true;
        if outside[n] != 0 {
            entries = entries
                .checked_sub(1)
                .ok_or(bug("pure preparation entry underflow"))?;
            entry_xor ^= n;
            outside[n] = 0;
        }
        push(&mut nodes, node)?;
        for &edge in &g.outgoing[n] {
            let to = f.links[edge.0 as usize].to;
            let t = to.0 as usize;
            if body[t] {
                continue;
            }
            if outside[t] == 0 {
                entries = entries
                    .checked_add(1)
                    .ok_or(bug("pure preparation entry overflow"))?;
                entry_xor ^= t;
            }
            outside[t] = outside[t]
                .checked_add(1)
                .ok_or(bug("pure preparation frontier overflow"))?;
            pending[t] = pending[t]
                .checked_sub(1)
                .ok_or(bug("pure preparation predecessor underflow"))?;
            if pending[t] == 0 {
                push(&mut queue, to)?;
            }
        }
    }
}
