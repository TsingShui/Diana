//! Partitions an original switch at distinct case entries and orders its case
//! DAG once. Nonadjacent arrivals retain original gotos; shared work has one body.

use super::super::build::*;
use super::super::graph::Graph;
use super::super::*;
use super::parts::*;
use crate::p5::StorageMethod;

pub(super) struct SwitchParts {
    pub parts: Vec<Part>,
    pub cases: Vec<Vec<LinkId>>,
}

struct Case {
    entry: NodeId,
    arms: Vec<LinkId>,
    nodes: Vec<NodeId>,
    next: Option<usize>,
}

fn groups(
    f: &RegionFacts,
    arms: &[LinkId],
) -> Result<(Vec<Case>, Vec<Option<usize>>), RegionError> {
    let mut cases: Vec<Case> = Vec::new();
    let mut entries = filled(f.nodes.len(), None)?;
    for &arm in arms {
        let entry = f.links[arm.0 as usize].to;
        let index = if let Some(index) = entries[entry.0 as usize] {
            index
        } else {
            let index = cases.len();
            entries[entry.0 as usize] = Some(index);
            push(
                &mut cases,
                Case {
                    entry,
                    arms: Vec::new(),
                    nodes: Vec::new(),
                    next: None,
                },
            )?;
            index
        };
        push(&mut cases[index].arms, arm)?;
    }
    Ok((cases, entries))
}

fn copy_free(storage: &StorageMethod<'_>, f: &RegionFacts, arms: &[LinkId]) -> bool {
    arms.iter().all(|arm| {
        f.link_plans[arm.0 as usize].copies.iter().all(|id| {
            let plan = &storage.facts().copy_plans()[id.0 as usize];
            plan.is_complete() && plan.steps().is_empty()
        })
    })
}

pub(super) fn private_switch_parts(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    c: &Candidate,
    boundary: &[bool],
) -> Result<Option<SwitchParts>, RegionError> {
    let RegionKind::Switch { arms, .. } = &c.kind else {
        return Ok(None);
    };
    if arms.len() < 2 {
        return Ok(None);
    }
    let (mut cases, entries) = groups(f, arms)?;
    let set = mask(f.nodes.len(), &c.parts[1].nodes)?;
    let mut owners = filled(f.nodes.len(), None)?;
    let mut interfaces = Vec::new();
    for (index, case) in cases.iter_mut().enumerate() {
        if Some(case.entry) != c.follow
            && (!set[case.entry.0 as usize] || boundary[case.entry.0 as usize])
        {
            return Ok(None);
        }
        let mut frontier = copy(boundary)?;
        for (node, &group) in entries.iter().enumerate() {
            if group.is_some_and(|other| other != index) {
                frontier[node] = true;
            }
        }
        let arm = super::completion::reachable(f, g, &[case.entry], &frontier, c.follow)?;
        case.nodes = list(&arm)?;
        let interface =
            super::completion::CompletionInterface::collect(storage, f, g, &case.nodes)?;
        if !interface.exceptional_entrances.is_empty() {
            return Ok(None);
        }
        for &node in &case.nodes {
            if !set[node.0 as usize] || owners[node.0 as usize].is_some() {
                return Ok(None);
            }
            owners[node.0 as usize] = Some(index);
            if g.outgoing[node.0 as usize].is_empty() && !interface.abrupt.contains(&node) {
                return Ok(None);
            }
        }
        for &outlet in &interface.outlets {
            let target = f.links[outlet.0 as usize].to;
            if Some(target) == c.follow {
                continue;
            }
            if let Some(next) = entries[target.0 as usize] {
                if case.next.is_some_and(|previous| previous != next) {
                    return Ok(None);
                }
                case.next = Some(next);
            } else if !boundary[target.0 as usize] {
                return Ok(None);
            }
        }
        if !normal_arm_acyclic(f, g, &arm)? {
            return Ok(None);
        }
        push(&mut interfaces, interface)?;
    }
    if c.parts[1]
        .nodes
        .iter()
        .any(|n| owners[n.0 as usize].is_none())
    {
        return Ok(None);
    }
    for (case, interface) in cases.iter().zip(&interfaces) {
        if interface.entrances.iter().any(|id| {
            let incoming = f.links[id.0 as usize];
            incoming.to != case.entry
                || (incoming.from != c.entry && owners[incoming.from.0 as usize].is_none())
        }) {
            return Ok(None);
        }
    }
    let Some(order) = order(storage, f, &cases, c.follow)? else {
        return Ok(None);
    };
    let mut parts = Vec::new();
    let mut ordered_cases = Vec::new();
    push(
        &mut parts,
        Part {
            nodes: copy(&[c.entry])?,
            entry: Some(c.entry),
            follow: None,
            completion: PartCompletion::Ordinary,
        },
    )?;
    for index in order {
        let follow = cases[index].next.map(|next| cases[next].entry).or(c.follow);
        let case = &mut cases[index];
        let nodes = std::mem::take(&mut case.nodes);
        push(
            &mut parts,
            Part {
                entry: (!nodes.is_empty()).then_some(case.entry),
                nodes,
                follow,
                completion: PartCompletion::Ordinary,
            },
        )?;
        push(&mut ordered_cases, std::mem::take(&mut case.arms))?;
    }
    Ok(Some(SwitchParts {
        parts,
        cases: ordered_cases,
    }))
}

fn order(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    cases: &[Case],
    follow: Option<NodeId>,
) -> Result<Option<Vec<usize>>, RegionError> {
    let mut incoming = filled(cases.len(), 0usize)?;
    for case in cases {
        if let Some(next) = case.next {
            incoming[next] = incoming[next]
                .checked_add(1)
                .ok_or(bug("case predecessor count overflow"))?;
        }
    }
    for (index, case) in cases.iter().enumerate() {
        // A header's copy would also run on lexical fallthrough. Shared labels
        // similarly cannot execute each other's entry copies. Keep such cases
        // explicit unless P5 has already established identity copies.
        if (incoming[index] != 0 || case.arms.len() > 1) && !copy_free(storage, f, &case.arms) {
            return Ok(None);
        }
    }
    let empty = cases.iter().position(|case| Some(case.entry) == follow);
    let mut roots = Vec::new();
    for (index, &count) in incoming.iter().enumerate() {
        if count == 0 && Some(index) != empty {
            push(&mut roots, index)?;
        }
    }
    let mut order = Vec::new();
    let mut seen = filled(cases.len(), false)?;
    for root in roots {
        let mut at = Some(root);
        while let Some(index) = at {
            if seen[index] {
                return Ok(None);
            }
            seen[index] = true;
            push(&mut order, index)?;
            // Several original case outlets may share this entry. Visit it
            // adjacent only after its last predecessor; other predecessors
            // retain explicit original gotos to its one printed label.
            at = if let Some(next) = cases[index].next {
                incoming[next] = incoming[next]
                    .checked_sub(1)
                    .ok_or(bug("case predecessor count underflow"))?;
                (incoming[next] == 0).then_some(next)
            } else {
                None
            };
        }
    }
    // A direct dispatch to the external follow must not accidentally enter the
    // next case body. Its empty group is therefore last, including its copies.
    if let Some(empty) = empty {
        seen[empty] = true;
        push(&mut order, empty)?;
    }
    if seen.iter().any(|&seen| !seen) {
        return Ok(None);
    }
    Ok(Some(order))
}

fn normal_arm_acyclic(f: &RegionFacts, g: &Graph, arm: &[bool]) -> Result<bool, RegionError> {
    let mut incoming = filled(f.nodes.len(), 0usize)?;
    let mut queue = Vec::new();
    let mut remaining = 0usize;
    for (node, &inside) in arm.iter().enumerate() {
        if inside {
            remaining += 1;
            incoming[node] = g.incoming[node]
                .iter()
                .filter(|l| arm[f.links[l.0 as usize].from.0 as usize])
                .count();
            if incoming[node] == 0 {
                push(&mut queue, NodeId(id(node)?))?;
            }
        }
    }
    while let Some(node) = queue.pop() {
        remaining -= 1;
        for &link in &g.outgoing[node.0 as usize] {
            let target = f.links[link.0 as usize].to;
            if arm[target.0 as usize] {
                incoming[target.0 as usize] -= 1;
                if incoming[target.0 as usize] == 0 {
                    push(&mut queue, target)?;
                }
            }
        }
    }
    Ok(remaining == 0)
}
