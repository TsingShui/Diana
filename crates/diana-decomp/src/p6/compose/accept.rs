//! Decides whether one proposal may become a region. Acceptance is the P6
//! contract: entry closure, construction-group atomicity, no split owner,
//! lock-scope boundary, loop-plan laminarity and the successor-boundary
//! acyclicity of a partial arm. A rejected proposal keeps its reason and the
//! nodes keep their previous owner.

use super::super::build::*;
use super::super::graph::Graph;
use super::super::*;
use super::parts::*;
use crate::p5::StorageMethod;

pub(super) fn accept(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    owners: &[RegionId],
    c: &Candidate,
    interface: &super::super::interface::CompletionInterface,
    set: &[bool],
) -> Result<Result<(), RecoveryReason>, RegionError> {
    if f.synchronized.iter().any(|scope| {
        scope.separation.is_none()
            && scope.body.contains(&c.entry)
            && c.members.iter().any(|n| !scope.body.contains(n))
    }) {
        return Ok(Err(RecoveryReason::LockBoundary));
    }
    if (interface.method_entry && c.entry != NodeId(0))
        || interface
            .entrances
            .iter()
            .any(|link| f.links[link.0 as usize].to != c.entry)
        || interface
            .exceptional_entrances
            .iter()
            .any(|&edge| Graph::exception_target(f, storage, edge) != Some(c.entry))
    {
        return Ok(Err(RecoveryReason::MultipleEntry));
    }
    let mut partition = filled(f.nodes.len(), usize::MAX)?;
    for (i, p) in c.parts.iter().enumerate() {
        for &n in &p.nodes {
            if !set[n.0 as usize] || partition[n.0 as usize] != usize::MAX {
                return Err(bug("region candidate has duplicate or foreign content"));
            }
            partition[n.0 as usize] = i;
        }
    }
    for group in storage.facts().construction_groups() {
        let mut part = None;
        let mut inside = 0;
        for op in group.members() {
            let node = g.op_node[op.0 as usize];
            if !set[node.0 as usize] {
                continue;
            }
            inside += 1;
            let current = partition[node.0 as usize];
            if part.is_some_and(|old| old != current) {
                return Ok(Err(RecoveryReason::ConstructionSplit));
            }
            part = Some(current);
        }
        if inside != 0 && inside != group.members().len() {
            return Ok(Err(RecoveryReason::ConstructionSplit));
        }
    }
    // Enclosing interfaces are selected before child owners exist. Preserve
    // the same whole-loop/lock obligation at that earlier selection point.
    for plan in &f.loops {
        if matches!(c.kind, RegionKind::Loop(id) if id == plan.id) {
            continue;
        }
        if plan.lexical_members.iter().all(|n| set[n.0 as usize])
            && let Some(first) = plan.lexical_members.first()
            && plan
                .lexical_members
                .iter()
                .any(|n| partition[n.0 as usize] != partition[first.0 as usize])
        {
            return Ok(Err(RecoveryReason::Overlap));
        }
    }
    for scope in &f.synchronized {
        if scope.separation.is_some()
            || matches!(c.kind, RegionKind::Synchronized(id) if id == scope.id)
        {
            continue;
        }
        let inside = scope.members.iter().filter(|n| set[n.0 as usize]).count();
        if inside == scope.members.len() {
            if let Some(first) = scope.members.first()
                && scope
                    .members
                    .iter()
                    .any(|n| partition[n.0 as usize] != partition[first.0 as usize])
            {
                return Ok(Err(RecoveryReason::LockBoundary));
            }
        } else if inside != 0 && !c.members.iter().all(|n| scope.body.contains(n)) {
            return Ok(Err(RecoveryReason::LockBoundary));
        }
    }
    let mut owners_in_parts = filled(f.regions.len(), None)?;
    for &n in &c.members {
        let part = partition[n.0 as usize];
        if part == usize::MAX {
            return Err(bug("region candidate omitted execution content"));
        }
        let root = owners[n.0 as usize];
        let (assigned, count) = owners_in_parts[root.0 as usize].get_or_insert((part, 0usize));
        if *assigned != part {
            return Ok(Err(RecoveryReason::Overlap));
        }
        *count += 1;
    }
    for (root, assignment) in owners_in_parts.into_iter().enumerate() {
        if assignment.is_some_and(|(_, count)| count != f.regions[root].member_count) {
            return Ok(Err(RecoveryReason::Overlap));
        }
    }
    Ok(Ok(()))
}
pub(super) fn match_branch_loops(
    f: &RegionFacts,
    c: &Candidate,
    set: &[bool],
) -> Result<Result<(), RecoveryReason>, RegionError> {
    if !matches!(c.kind, RegionKind::If { .. } | RegionKind::Condition(_)) {
        return Ok(Ok(()));
    }
    for plan in &f.loops {
        let inside = plan
            .lexical_members
            .iter()
            .filter(|n| set[n.0 as usize])
            .count();
        let owns_loop = inside == plan.lexical_members.len();
        // All established plans constrain branches, before any loop is selected.
        if inside != 0 && !owns_loop && inside != c.members.len() {
            return Ok(Err(RecoveryReason::Overlap));
        }
        let owns_private_exit = c.follow == Some(plan.continue_target())
            && plan.private_exits.iter().any(|exit| {
                let link = f.links[exit.0 as usize];
                set[link.from.0 as usize] && set[link.to.0 as usize]
            });
        if !owns_loop
            && (set[plan.continue_target().0 as usize]
                || (!owns_private_exit
                    && plan.latches.iter().any(|&latch| {
                        let source = f.links[latch.0 as usize].from;
                        set[source.0 as usize] && !owns_nested_exit_latch(f, c, set, plan, source)
                    })))
        {
            return Ok(Err(RecoveryReason::Overlap));
        }
    }
    Ok(Ok(()))
}
// A nested loop can already own a private corridor that continues an enclosing
// loop. Retain that producer's exact proof: the branch follows the INNER source
// continuation, owns the entire corridor and its entrance, and the particular
// enclosing latch lies there. Existence of some unrelated private exit cannot
// license another latch or acquiring either loop's continuation target.
fn owns_nested_exit_latch(
    f: &RegionFacts,
    c: &Candidate,
    set: &[bool],
    enclosing: &LoopPlan,
    source: NodeId,
) -> bool {
    f.loops.iter().any(|inner| {
        c.follow == Some(inner.continue_target())
            && inner.members.len() < enclosing.members.len()
            && inner.members.iter().all(|n| enclosing.members.contains(n))
            && inner.private_exit_corridors.iter().any(|corridor| {
                let entrance = f.links[corridor.exit.0 as usize];
                corridor.follow == Some(enclosing.continue_target())
                    && corridor.nodes.contains(&source)
                    && set[entrance.from.0 as usize]
                    && set[entrance.to.0 as usize]
                    && corridor.nodes.iter().all(|n| set[n.0 as usize])
            })
    })
}
pub(super) fn successor_arm_acyclic(
    f: &RegionFacts,
    owners: &[RegionId],
    arm: &[bool],
) -> Result<bool, RegionError> {
    let mut representative = filled(f.nodes.len(), usize::MAX)?;
    for (node, &inside) in arm.iter().enumerate() {
        if inside {
            representative[node] = node;
        }
    }
    let mut contracted = filled(f.nodes.len(), false)?;
    let mut seen = filled(f.regions.len(), false)?;
    let mut stack = Vec::new();
    for (n, &inside) in arm.iter().enumerate() {
        if inside {
            let owner = owners[n];
            if !seen[owner.0 as usize] {
                seen[owner.0 as usize] = true;
                push(&mut stack, owner)?;
            }
        }
    }
    while let Some(id) = stack.pop() {
        let region = &f.regions[id.0 as usize];
        if let RegionKind::Loop(loop_id) = region.kind
            && f.loops[loop_id.0 as usize]
                .lexical_members
                .iter()
                .all(|n| arm[n.0 as usize])
        {
            let entry = region.entry.ok_or(bug("selected loop has no entry"))?;
            for &n in &f.loops[loop_id.0 as usize].lexical_members {
                representative[n.0 as usize] = entry.0 as usize;
                contracted[n.0 as usize] = true;
            }
        } else {
            for &child in &region.children {
                push(&mut stack, child)?;
            }
        }
    }
    Ok(super::order::order(
        f,
        &representative,
        f.nodes.len(),
        None,
        None,
        |node| contracted[node.0 as usize],
        |vertex| f.nodes[vertex].id,
    )?
    .is_some())
}
pub(super) fn match_successor_boundary(
    f: &RegionFacts,
    owners: &[RegionId],
    c: &Candidate,
    interface: &super::super::interface::CompletionInterface,
) -> Result<Result<(), RecoveryReason>, RegionError> {
    if c.successor_boundary.is_none() {
        return Ok(Ok(()));
    }
    let follow = c.follow.ok_or(bug("successor boundary has no follow"))?;
    let mut arm = mask(f.nodes.len(), &c.members)?;
    // The exact header/arm partition was established before child construction.
    if arm[follow.0 as usize] {
        return Ok(Err(RecoveryReason::Unstructured));
    }
    if interface.method_entry && c.entry != NodeId(0) {
        return Ok(Err(RecoveryReason::MultipleEntry));
    }
    if interface
        .outlets
        .iter()
        .any(|&link| f.links[link.0 as usize].to != follow)
    {
        return Ok(Err(RecoveryReason::Unstructured));
    }
    arm[c.entry.0 as usize] = false;
    if !successor_arm_acyclic(f, owners, &arm)? {
        return Ok(Err(RecoveryReason::Unstructured));
    }
    Ok(Ok(()))
}
