//! Original-target interfaces used while selecting a region and its children.
//! Storage availability is not a control-edge predicate: links and terminals
//! retain their original copy, operand, completion and site products.

use super::super::build::*;
use super::super::graph::Graph;
use super::super::*;

pub(super) fn reachable(
    f: &RegionFacts,
    g: &Graph,
    starts: &[NodeId],
    boundary: &[bool],
    follow: Option<NodeId>,
) -> Result<Vec<bool>, RegionError> {
    let mut reached = filled(f.nodes.len(), false)?;
    let mut stack = Vec::new();
    for &start in starts {
        push(&mut stack, start)?;
    }
    while let Some(node) = stack.pop() {
        let index = node.0 as usize;
        if reached[index] || boundary[index] || follow == Some(node) {
            continue;
        }
        reached[index] = true;
        for &link in &g.outgoing[index] {
            push(&mut stack, f.links[link.0 as usize].to)?;
        }
    }
    Ok(reached)
}

/// A branch arm or loop body containing a complete established try body must
/// leave room for its handlers in the same child. Normal reachability alone
/// strands abrupt catch bodies before the try can be selected.
/// Shared handlers and foreign entrances still pass ordinary admission.
pub(super) fn include_try_handlers(
    f: &RegionFacts,
    boundary: &[bool],
    follow: Option<NodeId>,
    reached: &mut [bool],
) {
    loop {
        let mut changed = false;
        for plan in &f.tries {
            if !plan.body.iter().all(|n| reached[n.0 as usize])
                || plan
                    .members
                    .iter()
                    .any(|n| boundary[n.0 as usize] || Some(*n) == follow)
            {
                continue;
            }
            for &node in &plan.members {
                changed |= !reached[node.0 as usize];
                reached[node.0 as usize] = true;
            }
        }
        if !changed {
            break;
        }
    }
}

/// A unique first shared actual target, not a categorical return vertex. More
/// than one shared frontier stays explicit; no returned operands are compared.
pub(super) fn shared_target(
    f: &RegionFacts,
    g: &Graph,
    starts: &[NodeId],
    boundary: &[bool],
) -> Result<Option<NodeId>, RegionError> {
    let mut counts = filled(f.nodes.len(), 0usize)?;
    for &start in starts {
        let reached = reachable(f, g, &[start], boundary, None)?;
        for (index, inside) in reached.into_iter().enumerate() {
            if inside {
                counts[index] = counts[index]
                    .checked_add(1)
                    .ok_or(crate::RequestError::PlatformCapacity)?;
            }
        }
    }
    let mut target = None;
    for index in 0..counts.len() {
        if counts[index] < 2
            || g.incoming[index]
                .iter()
                .any(|link| counts[f.links[link.0 as usize].from.0 as usize] >= 2)
        {
            continue;
        }
        if target.is_some() {
            return Ok(None);
        }
        target = Some(NodeId(id(index)?));
    }
    Ok(target)
}
