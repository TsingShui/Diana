//! Establish lexical loop content without changing natural backedge membership.

use super::super::build::*;
use super::super::graph::Graph;
use super::super::*;
use super::parts::{copy, mask};
use crate::p2::{OpKind, Terminal};
use crate::p5::StorageMethod;

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
) -> Result<(), RegionError> {
    for index in 0..f.loops.len() {
        let plan = &f.loops[index];
        let mut members = copy(&plan.members)?;
        let mut private_exits = Vec::new();
        let mut private_exit_corridors = Vec::new();
        for scope in &f.synchronized {
            if scope.separation.is_none()
                && plan.members.contains(&g.op_node[scope.enter.0 as usize])
                && !scope.members.contains(&plan.header)
                && plan.follow.is_none_or(|n| !scope.members.contains(&n))
            {
                for &node in &scope.members {
                    if !members.contains(&node) {
                        push(&mut members, node)?;
                    }
                }
            }
        }
        // The predicate's own exit remains the real normal continuation. Only
        // other exits can contribute closed private corridors to the body.
        if let LoopForm::While(condition) | LoopForm::DoWhile(condition) = plan.form {
            for &exit in &plan.exits {
                if exit == condition.exit {
                    continue;
                }
                if let Some(corridor) = corridor(storage, f, g, index, exit)? {
                    push(&mut private_exits, exit)?;
                    for &node in &corridor.nodes {
                        if !members.contains(&node) {
                            push(&mut members, node)?;
                        }
                    }
                    push(&mut private_exit_corridors, corridor)?;
                }
            }
        }
        let mut boundary = mask(f.nodes.len(), &plan.evaluation)?;
        boundary[plan.header.0 as usize] = true;
        let mut reached = mask(f.nodes.len(), &members)?;
        // A private catch is lexical body content even when it cannot reach a
        // backedge. Predicate protection and shared follows stay outside it.
        super::completion::include_try_handlers(f, &boundary, plan.follow, &mut reached);
        f.loops[index].lexical_members = super::parts::list(&reached)?;
        f.loops[index].private_exits = private_exits;
        f.loops[index].private_exit_corridors = private_exit_corridors;
    }
    Ok(())
}

fn corridor(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    index: usize,
    exit: LinkId,
) -> Result<Option<super::super::product::PrivateLoopExit>, RegionError> {
    let plan = &f.loops[index];
    let mut enclosing = Vec::new();
    for other in &f.loops {
        if other.members.len() > plan.members.len()
            && plan.members.iter().all(|node| other.members.contains(node))
        {
            push(&mut enclosing, other.id)?;
        }
    }
    let mut nodes = Vec::new();
    // Membership of the walked corridor grows with the walk itself, so it is a
    // mask rather than a scan of `nodes` at every step.
    let mut walked = mask(f.nodes.len(), &[])?;
    let mut at = f.links[exit.0 as usize].to;
    let follow;
    loop {
        // An inner exit can finish its private work inside an enclosing core,
        // or leave through that loop's original break/continue destination.
        // Those destinations stay outside the corridor; only a selected scope
        // can later spell the outgoing original link as break or continue.
        if plan.follow == Some(at)
            || enclosing.iter().any(|id| {
                let outer = &f.loops[id.0 as usize];
                outer.follow == Some(at) || outer.continue_target() == at
            })
        {
            follow = Some(at);
            break;
        }
        if plan.members.contains(&at)
            || walked[at.0 as usize]
            || f.loops
                .iter()
                .any(|other| other.members.contains(&at) && !enclosing.contains(&other.id))
            || f.tries.iter().any(|scope| scope.members.contains(&at))
            || f.synchronized
                .iter()
                .any(|scope| scope.members.contains(&at))
        {
            return Ok(None);
        }
        let node = &f.nodes[at.0 as usize];
        if node.operations.clone().any(|op| {
            matches!(
                storage.execution().operations()[op as usize].kind(),
                OpKind::Monitor { .. }
            )
        }) || f.throws.iter().any(|site| {
            site.node == at
                && f.chains[site.chain.0 as usize]
                    .clauses
                    .iter()
                    .any(|clause| matches!(clause.target, CatchTarget::Handler(_)))
        }) {
            return Ok(None);
        }
        walked[at.0 as usize] = true;
        push(&mut nodes, at)?;
        match g.outgoing[at.0 as usize].as_slice() {
            [] if node.terminal
                && matches!(
                    storage.execution().blocks()[node.block.0 as usize].terminal(),
                    Some(Terminal::Return { .. } | Terminal::Throw { .. })
                ) =>
            {
                follow = None;
                break;
            }
            [link] => at = f.links[link.0 as usize].to,
            _ => return Ok(None),
        }
    }
    let set = walked;
    for &node in &nodes {
        if node == NodeId(0)
            || g.incoming[node.0 as usize].iter().any(|link| {
                let link = f.links[link.0 as usize];
                !set[link.from.0 as usize] && link.id != exit
            })
        {
            return Ok(None);
        }
    }
    // A normal body arm cannot own an exception-arrival path. Keep handler
    // sharing explicit even when its destination happens to be an abrupt node.
    for &edge in g.exceptions.iter().flatten() {
        if Graph::exception_possible(f, edge)
            && Graph::exception_target(f, storage, edge).is_some_and(|n| set[n.0 as usize])
        {
            return Ok(None);
        }
    }
    for group in storage.facts().construction_groups() {
        if group
            .members()
            .iter()
            .any(|op| set[g.op_node[op.0 as usize].0 as usize])
            && group.members().iter().any(|op| {
                let node = g.op_node[op.0 as usize];
                !set[node.0 as usize] && !plan.members.contains(&node)
            })
        {
            return Ok(None);
        }
    }
    Ok(Some(super::super::product::PrivateLoopExit {
        exit,
        nodes,
        follow,
    }))
}
