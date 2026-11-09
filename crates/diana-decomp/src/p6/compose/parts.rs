//! The vocabulary shared by region proposals: a candidate, its ordered parts and
//! the exported entries a part hands to its successors. A part is not a region
//! yet; it names original nodes whose already-selected owners a region will own.

use super::super::build::*;
use super::super::graph::Graph;
use super::super::*;
use crate::p5::StorageMethod;

pub(super) fn copy<T: Copy>(source: &[T]) -> Result<Vec<T>, RegionError> {
    let mut out = Vec::new();
    crate::substrate::owned::reserve(&mut out, source.len())?;
    out.extend_from_slice(source);
    Ok(out)
}
pub(super) fn mask(n: usize, nodes: &[NodeId]) -> Result<Vec<bool>, RegionError> {
    let mut out = filled(n, false)?;
    for p in nodes {
        out[p.0 as usize] = true;
    }
    Ok(out)
}
pub(super) fn list(set: &[bool]) -> Result<Vec<NodeId>, RegionError> {
    let mut out = Vec::new();
    for (i, &inside) in set.iter().enumerate() {
        if inside {
            push(&mut out, NodeId(id(i)?))?;
        }
    }
    Ok(out)
}
#[derive(Clone, Copy)]
pub(super) enum PartCompletion {
    Ordinary,
    LoopBody(LoopId),
}
pub(super) struct Part {
    pub(super) nodes: Vec<NodeId>,
    pub(super) entry: Option<NodeId>,
    pub(super) follow: Option<NodeId>,
    pub(super) completion: PartCompletion,
}
#[derive(Clone, Copy)]
pub(super) enum SuccessorBoundary {
    True(LinkId),
    False(LinkId),
}
pub(super) struct Candidate {
    pub(super) kind: RegionKind,
    pub(super) entry: NodeId,
    pub(super) follow: Option<NodeId>,
    pub(super) members: Vec<NodeId>,
    pub(super) parts: Vec<Part>,
    pub(super) successor_boundary: Option<SuccessorBoundary>,
}
/// Installs one admitted fragment after reserving both arenas. Child roles and
/// whole original membership come from the owning candidate/part constructors.
/// Updating parent and current-owner relations cannot allocate after reservation.
pub(super) fn contract(
    f: &mut RegionFacts,
    owners: &mut [RegionId],
    kind: RegionKind,
    entry: Option<NodeId>,
    follow: Option<NodeId>,
    children: Vec<RegionId>,
    nodes: &[NodeId],
) -> Result<RegionId, RegionError> {
    if f.parents.len() != f.regions.len() || owners.len() != f.nodes.len() {
        return Err(bug("fragment ownership has the wrong extent"));
    }
    let region = RegionId(id(f.regions.len())?);
    for &child in &children {
        if f.parents
            .get(child.0 as usize)
            .ok_or(bug("fragment names an unknown child"))?
            .is_some()
        {
            return Err(bug("region child has two execution owners"));
        }
    }
    crate::substrate::owned::reserve(&mut f.parents, 1)?;
    crate::substrate::owned::reserve(&mut f.regions, 1)?;
    for &node in nodes {
        if node.0 as usize >= owners.len() {
            return Err(bug("fragment membership names an unknown node"));
        }
    }
    f.parents.push(None);
    for &child in &children {
        f.parents[child.0 as usize] = Some(region);
    }
    for &node in nodes {
        owners[node.0 as usize] = region;
    }
    f.regions.push(Region {
        decision: None,
        id: region,
        kind,
        entry,
        follow,
        children,
        member_range: None,
        member_count: nodes.len(),
        continuation: Continuation::None,
    });
    Ok(region)
}
pub(in crate::p6) fn graph_kind(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    region: RegionId,
    entry: Option<NodeId>,
    reason: RecoveryReason,
) -> Result<RegionKind, RegionError> {
    let interface =
        super::super::interface::CompletionInterface::for_region(storage, f, g, region)?;
    let entries = interface.entry_nodes(storage, f, entry)?;
    Ok(RegionKind::Graph {
        reason,
        entries,
        exits: interface.outlets,
    })
}
