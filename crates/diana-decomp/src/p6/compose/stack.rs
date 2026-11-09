//! Turns one part into a region: give its nodes a single owner. A part whose
//! nodes already have one contiguous owner keeps that owner; anything else
//! becomes a sequence when the owners chain linearly and an explicit graph
//! otherwise. Only a graph keeps a labelled block, because only a graph needs a
//! lexical target for a transfer that leaves it.

use super::super::build::*;
use super::super::graph::Graph;
use super::super::*;
use super::parts::*;
use crate::p5::StorageMethod;

/// A printed order for the owners of one part: every original edge between two
/// owners points forward. Ties break on the owner's first node, so the order
/// follows the original block layout. `None` when an owner cycle remains.
fn topological(
    f: &RegionFacts,
    set: &[bool],
    owners: &[RegionId],
    roots: &[RegionId],
    entry: Option<NodeId>,
    loop_continue: Option<NodeId>,
) -> Result<Option<Vec<RegionId>>, RegionError> {
    let mut index = filled(f.regions.len(), usize::MAX)?;
    for (slot, &root) in roots.iter().enumerate() {
        index[root.0 as usize] = slot;
    }
    let mut representatives = filled(f.nodes.len(), usize::MAX)?;
    for (node, &inside) in set.iter().enumerate() {
        if inside {
            representatives[node] = index[owners[node].0 as usize];
        }
    }
    let start = entry.map(|node| index[owners[node.0 as usize].0 as usize]);
    let Some(order) = super::order::order(
        f,
        &representatives,
        roots.len(),
        start,
        loop_continue,
        |_| true,
        |slot| f.regions[roots[slot].0 as usize].nodes[0],
    )?
    else {
        return Ok(None);
    };
    let mut selected = Vec::new();
    for slot in order {
        push(&mut selected, roots[slot])?;
    }
    Ok(Some(selected))
}

pub(super) fn compose(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
    owners: &mut [RegionId],
    part: Part,
) -> Result<RegionId, RegionError> {
    let loop_continue = match part.completion {
        PartCompletion::Ordinary => None,
        PartCompletion::LoopBody(id) => {
            let target = f.loops[id.0 as usize].continue_target();
            if part.follow != Some(target) {
                return Err(bug("loop body lost its continue boundary"));
            }
            Some(target)
        }
    };
    let set = mask(f.nodes.len(), &part.nodes)?;
    let mut roots = Vec::new();
    for &p in &part.nodes {
        let r = owners[p.0 as usize];
        if !roots.contains(&r) {
            push(&mut roots, r)?;
        }
    }
    if roots.len() == 1 {
        return Ok(roots[0]);
    }
    let interface =
        super::super::interface::CompletionInterface::from_membership(storage, f, g, &set)?;
    let exported = interface.entry_nodes(storage, f, part.entry)?;
    // Linear chains are the single-successor case of this same contraction DAG.
    // Build and order it once instead of walking every member again on fallback.
    let ordered = if exported.len() <= 1 {
        topological(f, &set, owners, &roots, part.entry, loop_continue)?
    } else {
        None
    };
    let (kind, order) = if let Some(order) = ordered {
        (RegionKind::Sequence, order)
    } else {
        let reason = if f.issues.iter().any(|issue| {
            matches!(issue, PlanningIssue::Graph { entry, reason: RecoveryReason::Irreducible }
                if set[entry.0 as usize])
        }) {
            RecoveryReason::Irreducible
        } else if exported.len() > 1 {
            RecoveryReason::MultipleEntry
        } else {
            RecoveryReason::Unstructured
        };
        super::trace::graph(
            part.entry,
            &reason,
            part.nodes.len(),
            exported.len(),
            roots.len(),
        );
        if let Some(entry) = part.entry {
            push(&mut f.issues, PlanningIssue::Graph { entry, reason })?;
        }
        (
            RegionKind::Graph {
                reason,
                entries: exported,
                exits: interface.outlets,
            },
            roots,
        )
    };
    let labelled = matches!(kind, RegionKind::Graph { .. })
        .then(|| part.follow.filter(|n| !set[n.0 as usize]))
        .flatten();
    let mut nodes = part.nodes;
    let members = if labelled.is_some() {
        copy(&nodes)?
    } else {
        std::mem::take(&mut nodes)
    };
    let mut region = contract(f, owners, kind, part.entry, part.follow, order, members)?;
    if let Some(target) = labelled {
        let mut children = Vec::new();
        push(&mut children, region)?;
        region = contract(
            f,
            owners,
            RegionKind::LabelledBlock { target },
            part.entry,
            part.follow,
            children,
            nodes,
        )?;
    }
    Ok(region)
}
