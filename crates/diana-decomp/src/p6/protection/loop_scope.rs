//! Completes an abrupt catch-all scope over an already established loop.
//! A first per-iteration normal join is not necessarily the protection boundary.
//! No throwing work or recovering handler is silently acquired by this closure.

use super::*;

pub(super) fn enclosing<'f>(
    storage: &StorageMethod<'_>,
    f: &'f RegionFacts,
    g: &Graph,
    body: &[NodeId],
    arms: &[Vec<NodeId>],
    wanted: &[HandlerClause],
) -> Option<&'f LoopPlan> {
    let [clause] = wanted else {
        return None;
    };
    if clause.catch.is_some() || !matches!(clause.target, CatchTarget::Handler(_)) {
        return None;
    }
    // Every normal completion of a handler is internal or an original method
    // exit. An ordinary recovery join must stay at its original iteration.
    for arm in arms {
        for &node in arm {
            let outgoing = &g.outgoing[node.0 as usize];
            if outgoing
                .iter()
                .any(|l| !arm.contains(&f.links[l.0 as usize].to))
            {
                return None;
            }
            if outgoing.is_empty() {
                let n = &f.nodes[node.0 as usize];
                if !n.terminal
                    || !matches!(
                        storage.execution().blocks()[n.block.0 as usize].terminal(),
                        Some(
                            crate::p2::Terminal::Return { .. } | crate::p2::Terminal::Throw { .. }
                        )
                    )
                {
                    return None;
                }
            }
        }
    }
    f.loops
        .iter()
        .filter(|p| {
            p.members.len() > body.len()
                && body.iter().all(|n| p.members.contains(n))
                && arms.iter().flatten().all(|n| !p.members.contains(n))
                && p.follow.is_some_and(|follow| {
                    p.exits.iter().all(|l| f.links[l.0 as usize].to == follow)
                })
                && super::guarded(f, wanted, |n| p.members.contains(&n))
        })
        .max_by_key(|p| p.members.len())
}
