//! Proves single-entry loops with private handler rejoin corridors. Original
//! normal-DFS retreating links nominate headers, never establish dominance.
//! Admission requires closed real entrances and internal handler provenance;
//! normal LT forests and all original edges remain unchanged.

use super::super::build::*;
use super::super::dominance::Structure;
use super::super::graph::Graph;
use super::super::*;
use crate::p5::StorageMethod;

fn close(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    seeds: &[bool],
    header: NodeId,
) -> Result<Option<Vec<bool>>, RegionError> {
    if f.handlers.iter().any(|h| h.node == header) {
        return Ok(None);
    }
    let mut members = filled(seeds.len(), false)?;
    members.copy_from_slice(seeds);
    let mut queue = Vec::new();
    for node in &f.nodes {
        if members[node.id.0 as usize] && node.id != header {
            push(&mut queue, node.id)?;
        }
    }
    // A rejoining corridor is necessary only at an interior entrance. The
    // original header boundary stays open and is never pulled into its prefix.
    while let Some(node) = queue.pop() {
        for &link in &g.incoming[node.0 as usize] {
            let from = f.links[link.0 as usize].from;
            if !members[from.0 as usize] {
                members[from.0 as usize] = true;
                push(&mut queue, from)?;
            }
        }
    }
    if header != NodeId(0) && members[0] {
        return Ok(None);
    }
    let mut handler_root = false;
    for &root in &g.roots {
        if !members[root.0 as usize] || root == header {
            continue;
        }
        let Some(handler) = f.handlers.iter().find(|h| h.node == root) else {
            return Ok(None);
        };
        // An ignored exception can enter the same original update/goto as an
        // ordinary path. A capture still needs exceptional-only arrival. The
        // complete real-entrance checks below close every captureless arrival.
        if handler.capture.is_some() && !g.incoming[root.0 as usize].is_empty() {
            return Ok(None);
        }
        handler_root = true;
    }
    if !handler_root {
        return Ok(None);
    }
    // All genuine normal/exceptional arrivals must cross the original header or
    // originate inside the candidate. Analysis roots are not runtime entrances.
    if g.external_entry(f, &members, header) {
        return Ok(None);
    }
    for (from, edges) in g.exceptions.iter().enumerate() {
        if members[from] {
            continue;
        }
        for &edge in edges {
            if Graph::exception_possible(f, edge)
                && let Some(to) = Graph::exception_target(f, storage, edge)
                && members[to.0 as usize]
                && to != header
            {
                return Ok(None);
            }
        }
    }
    // Reject dead or mutually disconnected handler roots instead of treating
    // their normal return into the SCC as sufficient ownership evidence.
    let mut reached = filled(members.len(), false)?;
    reached[header.0 as usize] = true;
    push(&mut queue, header)?;
    while let Some(node) = queue.pop() {
        for &link in &g.outgoing[node.0 as usize] {
            let to = f.links[link.0 as usize].to;
            if members[to.0 as usize] && !reached[to.0 as usize] {
                reached[to.0 as usize] = true;
                push(&mut queue, to)?;
            }
        }
        for &edge in &g.exceptions[node.0 as usize] {
            if Graph::exception_possible(f, edge)
                && let Some(to) = Graph::exception_target(f, storage, edge)
                && members[to.0 as usize]
                && !reached[to.0 as usize]
            {
                reached[to.0 as usize] = true;
                push(&mut queue, to)?;
            }
        }
    }
    if members
        .iter()
        .zip(reached)
        .any(|(inside, reached)| *inside && !reached)
    {
        return Ok(None);
    }
    Ok(Some(members))
}

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    s: &Structure,
    heads: &mut [Vec<LinkId>],
) -> Result<Vec<Option<Vec<bool>>>, RegionError> {
    let mut closed = filled(f.nodes.len(), None)?;
    let mut candidates = Vec::new();
    for &link in &s.retreating {
        let header = f.links[link.0 as usize].to;
        if !candidates.contains(&header) {
            push(&mut candidates, header)?;
        }
    }
    for header in candidates {
        let incoming = s
            .retreating
            .iter()
            .copied()
            .filter(|l| f.links[l.0 as usize].to == header);
        if incoming.clone().all(|l| {
            s.dom
                .dominates(header.0 as usize, f.links[l.0 as usize].from.0 as usize)
        }) {
            continue;
        }
        let mut seeds = filled(f.nodes.len(), false)?;
        seeds[header.0 as usize] = true;
        for link in incoming {
            seeds[f.links[link.0 as usize].from.0 as usize] = true;
        }
        if let Some(members) = close(storage, f, g, &seeds, header)? {
            let mut latches = Vec::new();
            for link in &f.links {
                if link.to == header && members[link.from.0 as usize] {
                    push(&mut latches, link.id)?;
                }
            }
            if !latches.is_empty() {
                heads[header.0 as usize] = latches;
                closed[header.0 as usize] = Some(members);
            }
        }
    }
    Ok(closed)
}
