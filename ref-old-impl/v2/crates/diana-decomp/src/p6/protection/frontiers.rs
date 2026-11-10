//! One guarded catch-all footprint when distinct unguarded completion suffixes
//! prevent a common normal follow. Admission remains in the owning parent pass.

use super::*;

pub(super) struct Completion {
    pub body: Vec<bool>,
    pub follow: Option<NodeId>,
}

pub(super) fn complete(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    seeds: &[NodeId],
    wanted: &[HandlerClause],
    excluded: &[bool],
    handler_frontiers: &[NodeId],
) -> Result<Option<Completion>, RegionError> {
    let reach = reachable(storage, f, g, seeds, None, excluded, true)?;
    let mut outside = filled(f.nodes.len(), false)?;
    let mut guarded_seeds = filled(f.nodes.len(), false)?;
    for &seed in seeds {
        guarded_seeds[seed.0 as usize] = true;
    }
    let mut queue = Vec::new();
    for site in &f.throws {
        if reach[site.node.0 as usize]
            && !guarded_site(f, site, wanted, |node| reach[node.0 as usize])
            && !outside[site.node.0 as usize]
        {
            outside[site.node.0 as usize] = true;
            push(&mut queue, site.node)?;
        }
    }
    // A continuation shared by the selected capture and a body-private
    // capture is outside BOTH. Cutting only its first throwing effect would
    // leave its pure guard inside the body and create a false handler reentry.
    for &node in handler_frontiers {
        // An established loop reentry reached by recovery is not a cleanup
        // outlet. It stays in the protected body; the handler's own walk still
        // stops there. No loop membership or continuation is reconstructed.
        let reentry = f.loops.iter().any(|plan| {
            (node == plan.header || node == plan.continue_target())
                && seeds.iter().any(|seed| plan.members.contains(seed))
        });
        if !reentry && reach[node.0 as usize] && !outside[node.0 as usize] {
            outside[node.0 as usize] = true;
            push(&mut queue, node)?;
        }
    }
    if queue.is_empty() {
        return Ok(Some(Completion {
            body: reach,
            follow: None,
        }));
    }
    // Cut the entire normal suffix, including joins reached by a path that
    // skips its first effect. A return after close cannot reenter the try.
    while let Some(node) = queue.pop() {
        if guarded_seeds[node.0 as usize] {
            return Ok(None);
        }
        for &link in &g.outgoing[node.0 as usize] {
            let to = f.links[link.0 as usize].to;
            if reach[to.0 as usize] && !outside[to.0 as usize] {
                outside[to.0 as usize] = true;
                push(&mut queue, to)?;
            }
        }
    }
    for (slot, &blocked) in outside.iter_mut().zip(excluded) {
        *slot |= blocked;
    }
    // Materialize this ONE nominated boundary. Exception fragments whose only
    // incoming effect was cut are not body content merely because the initial
    // discovery walk saw them. This is not a retry after admission/refusal.
    let body = reachable(storage, f, g, seeds, None, &outside, true)?;
    if seeds.iter().any(|seed| !body[seed.0 as usize]) {
        return Ok(None);
    }
    // This is an actual lexical completion target, not a new runtime join.
    // Every different outlet stays explicit through the existing interface.
    let follow = f
        .links
        .iter()
        .filter(|link| body[link.from.0 as usize] && !body[link.to.0 as usize])
        .map(|link| link.to)
        .min();
    Ok(follow.map(|follow| Completion {
        body,
        follow: Some(follow),
    }))
}
