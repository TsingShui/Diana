//! Select enclosing interfaces before their children; commit on stack unwind.
//! No child is constructed against a boundary that its selected parent later
//! replaces. Sibling selections own disjoint whole parts, including empty arms.

use super::super::build::*;
use super::super::dominance::Structure;
use super::super::graph::Graph;
use super::super::interface::CompletionInterface;
use super::super::*;
use super::parts::*;
use crate::p5::StorageMethod;
use std::collections::HashSet;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum Selection {
    Loop(LoopId),
    Synchronized(SyncId),
    Try(TryId),
    Predicate(NodeId),
}
fn selection(c: &Candidate) -> Selection {
    match c.kind {
        RegionKind::Loop(id) => Selection::Loop(id),
        RegionKind::Synchronized(id) => Selection::Synchronized(id),
        RegionKind::Try(id) => Selection::Try(id),
        _ => Selection::Predicate(c.entry),
    }
}

enum Work {
    Scope(Part),
    Candidate(Candidate, CompletionInterface),
    Commit(Candidate, CompletionInterface, usize),
    Assemble(Part, usize),
}

fn exclusions<'a>(
    c: &Candidate,
    f: &'a RegionFacts,
) -> impl Iterator<Item = Selection> + Clone + 'a {
    let tests = match c.kind {
        RegionKind::Condition(id) => f.conditions[id.0 as usize].tests.as_slice(),
        _ => &[],
    };
    std::iter::once(selection(c)).chain(
        tests
            .iter()
            .map(|test| Selection::Predicate(test.predicate)),
    )
}

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
    s: &Structure,
    owners: &mut [RegionId],
    root: Part,
) -> Result<RegionId, RegionError> {
    let mut work = Vec::new();
    let mut results = Vec::new();
    let mut excluded = HashSet::new();
    push(&mut work, Work::Scope(root))?;
    while let Some(task) = work.pop() {
        match task {
            Work::Scope(part) => {
                let set = mask(f.nodes.len(), &part.nodes)?;
                let interface = CompletionInterface::from_membership(storage, f, g, &set)?;
                let mut boundary = interface.normal_boundary(f)?;
                if let Some(follow) = part.follow {
                    boundary[follow.0 as usize] = true;
                }
                let mut proposals =
                    super::candidates::candidates(storage, f, g, s, &part, &boundary)?;
                // Larger enclosing interfaces are selected first. Equal-size
                // protection/loop priorities preserve established scope ownership.
                proposals.sort_unstable_by_key(|c| {
                    (
                        usize::MAX - c.members.len(),
                        match c.kind {
                            RegionKind::Loop(_) => 0,
                            RegionKind::Synchronized(_) => 1,
                            RegionKind::Try(_) => 2,
                            RegionKind::Condition(_) => 3,
                            _ => 4,
                        },
                        match c.kind {
                            RegionKind::Try(id) => {
                                usize::MAX - f.tries[id.0 as usize].handlers.len()
                            }
                            _ => 0,
                        },
                        c.entry.0,
                        match c.successor_boundary {
                            None => 0,
                            Some(SuccessorBoundary::True(link)) => 1u64 + u64::from(link.0),
                            Some(SuccessorBoundary::False(link)) => {
                                (1u64 << 32) + u64::from(link.0)
                            }
                        },
                        c.follow.map(|n| n.0),
                    )
                });
                let mut occupied = filled(f.nodes.len(), false)?;
                let mut selected = Vec::new();
                for mut candidate in proposals {
                    if excluded.contains(&selection(&candidate))
                        || match candidate.kind {
                            RegionKind::Condition(id) => {
                                f.conditions[id.0 as usize].tests.iter().any(|test| {
                                    excluded.contains(&Selection::Predicate(test.predicate))
                                })
                            }
                            _ => false,
                        }
                        || candidate
                            .members
                            .iter()
                            .any(|n| !set[n.0 as usize] || occupied[n.0 as usize])
                    {
                        continue;
                    }
                    if let Some(parts) =
                        super::switch::private_switch_parts(storage, f, g, &candidate, &boundary)?
                    {
                        candidate.parts = parts.parts;
                        if let RegionKind::Switch { cases, .. } = &mut candidate.kind {
                            *cases = Some(parts.cases);
                        }
                    }
                    let candidate_set = mask(f.nodes.len(), &candidate.members)?;
                    let completion =
                        CompletionInterface::from_membership(storage, f, g, &candidate_set)?;
                    let accepted = match super::accept::accept(
                        storage,
                        f,
                        g,
                        owners,
                        &candidate,
                        &completion,
                        &candidate_set,
                    )? {
                        Ok(()) => super::accept::match_branch_loops(f, &candidate, &candidate_set)?,
                        Err(reason) => Err(reason),
                    };
                    if let Err(reason) = accepted {
                        super::trace::rejected(
                            candidate.entry,
                            &reason,
                            candidate.members.len(),
                            candidate.follow,
                            candidate.successor_boundary.is_some(),
                        );
                        push(
                            &mut f.issues,
                            PlanningIssue::Graph {
                                entry: candidate.entry,
                                reason,
                            },
                        )?;
                        continue;
                    }
                    for &node in &candidate.members {
                        occupied[node.0 as usize] = true;
                    }
                    push(&mut selected, (candidate, completion))?;
                }
                let count = selected.len();
                push(&mut work, Work::Assemble(part, count))?;
                for (candidate, completion) in selected.into_iter().rev() {
                    push(&mut work, Work::Candidate(candidate, completion))?;
                }
            }
            Work::Candidate(mut candidate, completion) => {
                let keys = exclusions(&candidate, f);
                excluded
                    .try_reserve(keys.clone().count())
                    .map_err(crate::RequestError::Allocation)?;
                excluded.extend(keys);
                let mut parts = std::mem::take(&mut candidate.parts);
                if let RegionKind::Loop(id) = candidate.kind
                    && let Some(body) = parts.first_mut()
                {
                    body.completion = PartCompletion::LoopBody(id);
                }
                let count = parts.len();
                push(&mut work, Work::Commit(candidate, completion, count))?;
                for part in parts.into_iter().rev() {
                    push(&mut work, Work::Scope(part))?;
                }
            }
            Work::Commit(candidate, completion, count) => {
                for key in exclusions(&candidate, f) {
                    excluded.remove(&key);
                }
                let start = results
                    .len()
                    .checked_sub(count)
                    .ok_or(bug("region child result underflow"))?;
                let children = copy(&results[start..])?;
                results.truncate(start);
                if let Err(reason) =
                    super::accept::match_successor_boundary(f, owners, &candidate, &completion)?
                {
                    super::trace::rejected(
                        candidate.entry,
                        &reason,
                        candidate.members.len(),
                        candidate.follow,
                        candidate.successor_boundary.is_some(),
                    );
                    push(
                        &mut f.issues,
                        PlanningIssue::Graph {
                            entry: candidate.entry,
                            reason,
                        },
                    )?;
                    // The interface and its child ownership remain intact on
                    // refusal. Assemble those same children once; no alternate
                    // boundary, analysis or reconstruction is attempted.
                    let region = super::stack::compose(
                        storage,
                        f,
                        g,
                        owners,
                        Part {
                            nodes: candidate.members,
                            entry: Some(candidate.entry),
                            follow: candidate.follow,
                            completion: PartCompletion::Ordinary,
                        },
                    )?;
                    push(&mut results, region)?;
                    continue;
                }
                let region = contract(
                    f,
                    owners,
                    candidate.kind,
                    Some(candidate.entry),
                    candidate.follow,
                    children,
                    &candidate.members,
                )?;
                push(&mut results, region)?;
            }
            Work::Assemble(part, count) => {
                let start = results
                    .len()
                    .checked_sub(count)
                    .ok_or(bug("region scope result underflow"))?;
                results.truncate(start);
                let region = super::stack::compose(storage, f, g, owners, part)?;
                push(&mut results, region)?;
            }
        }
    }
    if results.len() != 1 {
        return Err(bug("region construction lost its root result"));
    }
    results.pop().ok_or(bug("region construction has no root"))
}
