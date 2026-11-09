//! Orders a contraction of original normal links. Callers supply the established
//! representatives and decide which internal self-edges belong to a compound.
//! Boundary/exception admission stays with the common CompletionInterface.

use super::super::build::*;
use super::super::*;
use std::{cmp::Reverse, collections::BinaryHeap};

pub(super) fn order(
    f: &RegionFacts,
    representatives: &[usize],
    count: usize,
    first: Option<usize>,
    repeat: Option<NodeId>,
    collapsed: impl Fn(NodeId) -> bool,
    key: impl Fn(usize) -> NodeId,
) -> Result<Option<Vec<usize>>, RegionError> {
    if representatives.len() != f.nodes.len() {
        return Err(bug("fragment representatives have the wrong extent"));
    }
    // MAX marks an absent vertex until its first original member is installed.
    let mut incoming = filled(count, usize::MAX)?;
    let mut outgoing = lists(count)?;
    for &vertex in representatives {
        if vertex == usize::MAX {
            continue;
        }
        *incoming
            .get_mut(vertex)
            .ok_or(bug("fragment representative names an unknown vertex"))? = 0;
    }
    for link in &f.links {
        let from = representatives[link.from.0 as usize];
        let to = representatives[link.to.0 as usize];
        if from == usize::MAX
            || to == usize::MAX
            || Some(link.to) == repeat
            || (from == to && collapsed(link.from))
        {
            continue;
        }
        push(&mut outgoing[from], to)?;
        incoming[to] = incoming[to]
            .checked_add(1)
            .ok_or(crate::RequestError::PlatformCapacity)?;
    }
    let preferred = first.filter(|&vertex| incoming.get(vertex) == Some(&0));
    let mut ready = BinaryHeap::new();
    ready
        .try_reserve(count)
        .map_err(crate::RequestError::Allocation)?;
    let mut remaining = 0usize;
    for (vertex, &pending) in incoming.iter().enumerate() {
        if pending != usize::MAX {
            remaining += 1;
            if pending == 0 {
                ready.push(Reverse((Some(vertex) != preferred, key(vertex), vertex)));
            }
        }
    }
    let mut result = Vec::new();
    while let Some(Reverse((_, _, pick))) = ready.pop() {
        push(&mut result, pick)?;
        for &to in &outgoing[pick] {
            incoming[to] -= 1;
            if incoming[to] == 0 {
                ready.push(Reverse((true, key(to), to)));
            }
        }
    }
    Ok((result.len() == remaining).then_some(result))
}
