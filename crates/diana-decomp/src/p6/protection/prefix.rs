//! Optional nonthrowing prefixes cannot destroy an established guarded entry.
//!
//! The base body retains every guarded seed and its original forward content.
//! Grow the predecessor closure once, recording complete single-entry frontiers.
//! A later optional predecessor can introduce another entrance; selecting the
//! last established checkpoint drops only that optional suffix, never a seed or
//! a successor path. No rejected owner or boundary is reconstructed.

use super::super::build::{bug, filled, id, push};
use super::super::graph::Graph;
use super::super::{EdgeRoute, ExceptionDelivery, NodeId, RegionError, RegionFacts};
use crate::p5::StorageMethod;

struct Frontier {
    outside: Vec<usize>,
    entries: usize,
    // XOR identifies the entry exactly when cardinality is one; it is not a
    // hash/equality proof for larger sets or independent graph verification.
    entry_xor: usize,
}
impl Frontier {
    fn new(
        storage: &StorageMethod<'_>,
        f: &RegionFacts,
        g: &Graph,
        body: &[bool],
    ) -> Result<Self, RegionError> {
        let mut result = Self {
            outside: filled(f.nodes.len(), 0usize)?,
            entries: 0,
            entry_xor: 0,
        };
        for link in &f.links {
            if !body[link.from.0 as usize] {
                result.arrival(link.to)?;
            }
        }
        for (from, edges) in g.exceptions.iter().enumerate() {
            if body[from] {
                continue;
            }
            for &edge in edges {
                if Graph::exception_possible(f, edge)
                    && let Some(target) = Graph::exception_target(f, storage, edge)
                {
                    result.arrival(target)?;
                }
            }
        }
        for (node, &inside) in body.iter().enumerate() {
            if inside && (node == 0 || result.outside[node] != 0) {
                result.add_entry(node)?;
            }
        }
        Ok(result)
    }
    fn arrival(&mut self, node: NodeId) -> Result<(), RegionError> {
        let count = &mut self.outside[node.0 as usize];
        *count = count
            .checked_add(1)
            .ok_or(bug("protected entry arrival count overflow"))?;
        Ok(())
    }
    fn add_entry(&mut self, node: usize) -> Result<(), RegionError> {
        self.entries = self
            .entries
            .checked_add(1)
            .ok_or(bug("protected entry count overflow"))?;
        self.entry_xor ^= node;
        Ok(())
    }
    fn internalize(&mut self, node: NodeId, body: &[bool]) -> Result<(), RegionError> {
        let n = node.0 as usize;
        let count = &mut self.outside[n];
        *count = count
            .checked_sub(1)
            .ok_or(bug("protected entry arrival count underflow"))?;
        if *count == 0 && body[n] && n != 0 {
            self.entries = self
                .entries
                .checked_sub(1)
                .ok_or(bug("protected entry count underflow"))?;
            self.entry_xor ^= n;
        }
        Ok(())
    }
    fn include(
        &mut self,
        storage: &StorageMethod<'_>,
        f: &RegionFacts,
        g: &Graph,
        body: &[bool],
        node: NodeId,
    ) -> Result<(), RegionError> {
        let n = node.0 as usize;
        if n == 0 || self.outside[n] != 0 {
            self.add_entry(n)?;
        }
        for &link in &g.outgoing[n] {
            self.internalize(f.links[link.0 as usize].to, body)?;
        }
        for &edge in &g.exceptions[n] {
            if Graph::exception_possible(f, edge)
                && let Some(target) = Graph::exception_target(f, storage, edge)
            {
                self.internalize(target, body)?;
            }
        }
        Ok(())
    }
    fn entry(&self) -> Result<Option<NodeId>, RegionError> {
        if self.entries == 1 {
            Ok(Some(NodeId(id(self.entry_xor)?)))
        } else {
            Ok(None)
        }
    }
}

pub(super) fn complete(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    body: &mut [bool],
    stop: Option<NodeId>,
    excluded: &[bool],
) -> Result<Option<NodeId>, RegionError> {
    let mut frontier = Frontier::new(storage, f, g, body)?;
    let mut choice = frontier.entry()?.map(|entry| (0, entry));
    let mut added = Vec::new();
    let mut eligible = filled(f.nodes.len(), true)?;
    let mut pending = filled(f.nodes.len(), 0usize)?;
    let mut queue = Vec::new();
    for site in &f.throws {
        if site.edges.iter().any(|edge| {
            !matches!(
                f.edges[edge.0 as usize].route,
                EdgeRoute::Exception {
                    delivery: ExceptionDelivery::ImpossibleRelease(_),
                    ..
                }
            )
        }) {
            eligible[site.node.0 as usize] = false;
        }
    }
    for node in &f.nodes {
        let n = node.id.0 as usize;
        eligible[n] &=
            !body[n] && !excluded[n] && Some(node.id) != stop && !g.outgoing[n].is_empty();
        if eligible[n] {
            pending[n] = g.outgoing[n]
                .iter()
                .filter(|link| !body[f.links[link.0 as usize].to.0 as usize])
                .count();
            if pending[n] == 0 {
                push(&mut queue, node.id)?;
            }
        }
    }
    while let Some(node) = queue.pop() {
        let n = node.0 as usize;
        if body[n] {
            continue;
        }
        body[n] = true;
        frontier.include(storage, f, g, body, node)?;
        push(&mut added, node)?;
        if let Some(entry) = frontier.entry()? {
            choice = Some((added.len(), entry));
        }
        for &link in &g.incoming[n] {
            let from = f.links[link.0 as usize].from;
            let p = from.0 as usize;
            if eligible[p] && !body[p] {
                pending[p] = pending[p]
                    .checked_sub(1)
                    .ok_or(bug("protected prefix predecessor count underflow"))?;
                if pending[p] == 0 {
                    push(&mut queue, from)?;
                }
            }
        }
    }
    let Some((keep, entry)) = choice else {
        return Ok(None);
    };
    for node in added.iter().skip(keep) {
        body[node.0 as usize] = false;
    }
    Ok(Some(entry))
}
