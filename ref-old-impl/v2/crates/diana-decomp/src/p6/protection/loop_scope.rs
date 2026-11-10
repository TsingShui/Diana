//! Nominate one protected loop frontier before ordinary try admission.

use super::*;

// Normal dominance is already established over the original normal roots. A
// recovering handler has an outgoing link into content not dominated by its
// capture root; a closed abrupt handler cannot resume that shared content.
pub(super) fn abrupt(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    s: &Structure,
    root: NodeId,
) -> bool {
    s.dom.dominates(root.0 as usize, root.0 as usize)
        && f.nodes
            .iter()
            .filter(|n| s.dom.dominates(root.0 as usize, n.id.0 as usize))
            .all(|n| {
                let outgoing = &g.outgoing[n.id.0 as usize];
                outgoing.iter().all(|l| {
                    s.dom
                        .dominates(root.0 as usize, f.links[l.0 as usize].to.0 as usize)
                }) && (!outgoing.is_empty()
                    || (n.terminal
                        && matches!(
                            storage.execution().blocks()[n.block.0 as usize].terminal(),
                            Some(
                                crate::p2::Terminal::Return { .. }
                                    | crate::p2::Terminal::Throw { .. }
                            )
                        )))
            })
}

pub(super) struct Completion {
    pub body: Vec<bool>,
    pub entry: NodeId,
    pub follow: NodeId,
}

pub(super) fn complete(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    seeds: &[NodeId],
    wanted: &[HandlerClause],
    excluded: &[bool],
    current: Option<NodeId>,
) -> Result<Option<Completion>, RegionError> {
    let [clause] = wanted else {
        return Ok(None);
    };
    if clause.catch.is_some() || !matches!(clause.target, CatchTarget::Handler(_)) {
        return Ok(None);
    }
    let mut selected = None;
    let mut size = 0;
    for plan in &f.loops {
        let Some(follow) = plan.follow else { continue };
        if plan.members.len() <= size
            || current.is_some_and(|n| !plan.members.contains(&n))
            || !seeds.iter().any(|n| plan.members.contains(n))
        {
            continue;
        }
        let body = reachable(storage, f, g, &[plan.header], Some(follow), excluded, true)?;
        if plan.members.iter().any(|n| !body[n.0 as usize])
            || seeds.iter().any(|n| !body[n.0 as usize])
            || !guarded(f, wanted, |n| body[n.0 as usize])
            || !closed_entry(storage, f, g, &body, plan.header)
        {
            continue;
        }
        size = plan.members.len();
        selected = Some(Completion {
            body,
            entry: plan.header,
            follow,
        });
    }
    Ok(selected)
}
