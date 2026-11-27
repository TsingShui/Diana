//! Search disjoint single-entry branch arms on one immutable acyclic graph.
use super::{BlockId, BranchArm, ContinuationTail, FlowNode, FlowTopology, SemanticFlowGraph};
use foldhash::HashMap;
use smallvec::SmallVec;
use std::cell::OnceCell;
use std::ops::Range;

struct ArmRow<'a> {
    block: BlockId,
    node: &'a FlowNode,
    tail: OnceCell<Option<ContinuationTail>>,
    targets: SmallVec<[(BlockId, ArmEdge); 2]>,
    predecessors: SmallVec<[usize; 2]>,
    is_continuation: bool,
}

#[derive(Clone, Copy)]
enum ArmEdge {
    Local(usize),
    Continue,
    Outside,
}

struct ArmMembers {
    visited: Range<usize>,
    continuation: Option<usize>,
}

pub(super) struct BranchArmSearch<'a> {
    graph: &'a SemanticFlowGraph,
    topology: &'a FlowTopology,
    ranks: &'a HashMap<BlockId, usize>,
    rows: Vec<ArmRow<'a>>,
    marks: Vec<usize>,
    generation: usize,
    pending: Vec<usize>,
    members: Vec<usize>,
}

impl<'a> BranchArmSearch<'a> {
    pub(super) fn new(
        graph: &'a SemanticFlowGraph,
        topology: &'a FlowTopology,
        order: &[BlockId],
        ranks: &'a HashMap<BlockId, usize>,
    ) -> Self {
        // The topological order contains each graph node once. Boundaries
        // retain their block identity because a candidate may end at one.
        let rows = order
            .iter()
            .map(|&block| {
                let targets = topology
                    .targets(block)
                    .iter()
                    .map(|&target| {
                        let edge = if graph.continues.contains_key(&target) {
                            ArmEdge::Continue
                        } else if graph.exits.contains(&target) {
                            ArmEdge::Outside
                        } else {
                            ranks
                                .get(&target)
                                .copied()
                                .map_or(ArmEdge::Outside, ArmEdge::Local)
                        };
                        (target, edge)
                    })
                    .collect();
                let predecessors = topology
                    .predecessors
                    .get(&block)
                    .into_iter()
                    .flatten()
                    .map(|source| ranks[source])
                    .collect();
                ArmRow {
                    block,
                    node: &graph.nodes[&block],
                    tail: OnceCell::new(),
                    targets,
                    predecessors,
                    is_continuation: graph.continues.contains_key(&block),
                }
            })
            .collect();
        Self {
            graph,
            topology,
            ranks,
            rows,
            marks: vec![0; order.len()],
            generation: 0,
            pending: Vec::new(),
            members: Vec::new(),
        }
    }

    pub(super) fn pair(
        &mut self,
        source: BlockId,
        when_true: BlockId,
        when_false: BlockId,
        join: BlockId,
    ) -> Option<(BranchArm, BranchArm)> {
        // Two generations distinguish the arms. Marks from a rejected
        // candidate cannot contribute to the next candidate's ownership.
        if self.generation >= usize::MAX - 2 {
            self.marks.fill(0);
            self.generation = 0;
        }
        self.generation += 2;
        self.members.clear();
        let left = self.scan(source, when_true, join, self.generation)?;
        let right = self.scan(source, when_false, join, self.generation + 1)?;
        let left = self.finish(left);
        let right = self.finish(right);
        (!(left.is_empty() && right.is_empty())).then_some((left, right))
    }

    fn scan(
        &mut self,
        source: BlockId,
        entry: BlockId,
        join: BlockId,
        mark: usize,
    ) -> Option<ArmMembers> {
        let start = self.members.len();
        if entry == join {
            return Some(ArmMembers {
                visited: start..start,
                continuation: None,
            });
        }
        self.pending.clear();
        self.pending.push(*self.ranks.get(&entry)?);
        let mut continuation = None;
        let mut reaches_join = false;
        let mut open_boundary = false;
        while let Some(rank) = self.pending.pop() {
            let row = &self.rows[rank];
            // A private epilogue is part of the arm only when the join cuts
            // through it. Otherwise its body is attached after lowering.
            let tail = row.tail.get_or_init(|| {
                ContinuationTail::analyze(self.graph, &self.topology.predecessors, row.block)
            });
            if tail
                .as_ref()
                .is_some_and(|tail| !tail.blocks.contains(&join))
            {
                if reaches_join || continuation.is_some_and(|other| other != rank) {
                    return None;
                }
                continuation = Some(rank);
                continue;
            }
            if self.marks[rank] == mark {
                continue;
            }
            if row.is_continuation || self.marks[rank] >= self.generation {
                return None;
            }
            self.marks[rank] = mark;
            self.members.push(rank);
            for &(target, edge) in &row.targets {
                if target == join {
                    if continuation.is_some() {
                        return None;
                    }
                    reaches_join = true;
                } else {
                    match edge {
                        ArmEdge::Local(next) => self.pending.push(next),
                        ArmEdge::Continue => open_boundary = true,
                        ArmEdge::Outside => return None,
                    }
                }
            }
        }
        let members = &self.members[start..];
        if !reaches_join && continuation.is_none() && !open_boundary {
            let mut terminals = members
                .iter()
                .map(|&rank| &self.rows[rank])
                .filter(|row| row.targets.is_empty())
                .peekable();
            if terminals.peek().is_none()
                || terminals.any(|row| row.node.completion(self.graph.region).normal)
            {
                return None;
            }
        }
        if entry != source
            && members.iter().any(|&rank| {
                let row = &self.rows[rank];
                row.predecessors.iter().any(|&pred| {
                    self.marks[pred] != mark
                        && !(row.block == entry && self.rows[pred].block == source)
                })
            })
        {
            return None;
        }
        Some(ArmMembers {
            visited: start..self.members.len(),
            continuation,
        })
    }

    fn finish(&self, arm: ArmMembers) -> BranchArm {
        BranchArm {
            nodes: self.members[arm.visited]
                .iter()
                .map(|&rank| self.rows[rank].block)
                .collect(),
            continuation: arm
                .continuation
                .and_then(|rank| self.rows[rank].tail.get()?.clone()),
        }
    }
}
