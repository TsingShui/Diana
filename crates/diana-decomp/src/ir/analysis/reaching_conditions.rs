//! Symbolic reachability facts for acyclic ordinary-control-flow graphs.
//!
//! Every block is associated with the exact Boolean condition under which it
//! is reached from the graph entry. ROBDD variables follow topological
//! evaluation order and provide canonical equivalence and implication.

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};
use std::fmt;

use foldhash::HashMap;
use smallvec::SmallVec;

use crate::ir::bdd::{Bdd, BddContext, BddError};
use crate::ir::{BlockId, BoolExpr, BoolVariable, EdgeKind, InsnType, CFG};

#[derive(Debug, Clone)]
pub struct ReachingCondition {
    domain: Bdd,
}

impl ReachingCondition {
    pub fn domain(&self) -> Bdd {
        self.domain
    }
}

pub struct ReachingConditions {
    order: Vec<BlockId>,
    conditions: Vec<ReachingCondition>,
    reachability: ReachabilityMatrix,
    branch_variables: Vec<BlockId>,
    boolean_domain: BddContext,
}

impl ReachingConditions {
    pub fn analyze(cfg: &CFG) -> Result<Self, ReachingConditionError> {
        Self::analyze_nodes(cfg, cfg.entry, cfg.graph_nodes())
    }

    /// Compute exact reaching conditions for an induced acyclic CFG slice.
    ///
    /// Keeping the Boolean domain local is important for gated SSA: a Phi
    /// selection depends on the branch frontier that feeds its join, not on
    /// every predicate in the containing method.
    pub fn analyze_slice(
        cfg: &CFG,
        entry: BlockId,
        nodes: &BTreeSet<BlockId>,
    ) -> Result<Self, ReachingConditionError> {
        Self::analyze_nodes(cfg, entry, nodes.iter().copied())
    }

    fn analyze_nodes(
        cfg: &CFG,
        entry: BlockId,
        nodes: impl ExactSizeIterator<Item = BlockId> + Clone,
    ) -> Result<Self, ReachingConditionError> {
        let topology = AcyclicTopology { cfg };
        let (order, ranks) = topology.sort(entry, nodes)?;
        // Keep each source's validated role next to its topological rank.
        // Incoming edges reuse the role and the literal's ordinal.
        let mut branch_variables = Vec::new();
        let controls = order
            .iter()
            .map(|&block| {
                let Some(body) = cfg.block(block) else {
                    return EdgeSource::Missing;
                };
                match body.terminator().map(|terminator| terminator.insn_type) {
                    Some(InsnType::If) => {
                        let ordinal = branch_variables.len();
                        branch_variables.push(block);
                        EdgeSource::Branch(ordinal)
                    }
                    Some(InsnType::Switch) => EdgeSource::Switch,
                    _ => EdgeSource::Ordinary,
                }
            })
            .collect::<SmallVec<[_; 8]>>();
        let boolean_domain =
            BddContext::ordered(branch_variables.iter().copied().map(BoolVariable::Block));
        let mut conditions = Vec::<ReachingCondition>::with_capacity(order.len());
        for block in &order {
            let domain = if *block == entry {
                boolean_domain.truth()
            } else {
                let terms = topology
                    .incoming(*block, &ranks)
                    .map(|(source, rank, edge)| {
                        let predecessor = conditions.get(rank).ok_or(
                            ReachingConditionError::MissingPredecessorCondition {
                                block: *block,
                                predecessor: source,
                            },
                        )?;
                        let edge = controls[rank].compile(&boolean_domain, source, *block, edge)?;
                        Ok(boolean_domain.and(predecessor.domain, edge)?)
                    })
                    .collect::<Result<SmallVec<[_; 2]>, ReachingConditionError>>()?;
                terms
                    .into_iter()
                    .try_fold(boolean_domain.falsity(), |left, right| {
                        boolean_domain.or(left, right)
                    })?
            };
            if domain.is_false() {
                return Err(ReachingConditionError::UnreachableBlock(*block));
            }
            conditions.push(ReachingCondition { domain });
        }
        let reachability = ReachabilityMatrix::analyze(cfg, &order, ranks)?;
        Ok(Self {
            order,
            conditions,
            reachability,
            branch_variables,
            boolean_domain,
        })
    }

    pub fn edge_condition(
        &self,
        cfg: &CFG,
        source: BlockId,
        target: BlockId,
        edge: EdgeKind,
    ) -> Result<Bdd, ReachingConditionError> {
        let control = match cfg.block(source) {
            None => EdgeSource::Missing,
            Some(block) => match block.terminator().map(|term| term.insn_type) {
                Some(InsnType::If) => EdgeSource::NamedBranch,
                Some(InsnType::Switch) => EdgeSource::Switch,
                _ => EdgeSource::Ordinary,
            },
        };
        control.compile(&self.boolean_domain, source, target, edge)
    }

    pub fn order(&self) -> &[BlockId] {
        &self.order
    }

    pub fn rank(&self, block: BlockId) -> Option<usize> {
        self.reachability.indices.get(&block).copied()
    }

    pub fn condition(&self, block: BlockId) -> Option<&ReachingCondition> {
        self.ranked_condition(block).map(|(_, condition)| condition)
    }

    pub(in crate::ir) fn ranked_condition(
        &self,
        block: BlockId,
    ) -> Option<(usize, &ReachingCondition)> {
        let rank = self.rank(block)?;
        Some((rank, self.conditions.get(rank)?))
    }

    pub fn branch_variables(&self) -> &[BlockId] {
        &self.branch_variables
    }

    pub fn truth(&self) -> Bdd {
        self.boolean_domain.truth()
    }

    pub(in crate::ir) fn branch_literal(
        &self,
        index: usize,
    ) -> Result<Bdd, ReachingConditionError> {
        Ok(self.boolean_domain.compile_index(index, true)?)
    }

    pub fn compile(&self, expression: &BoolExpr) -> Result<Bdd, ReachingConditionError> {
        Ok(self.boolean_domain.compile(expression)?)
    }

    pub fn expression(
        &self,
        domain: Bdd,
        node_limit: usize,
    ) -> Result<Option<(BoolExpr, usize)>, ReachingConditionError> {
        Ok(self.boolean_domain.expression(domain, node_limit)?)
    }

    pub fn and(&self, left: Bdd, right: Bdd) -> Result<Bdd, ReachingConditionError> {
        Ok(self.boolean_domain.and(left, right)?)
    }

    pub fn or(&self, left: Bdd, right: Bdd) -> Result<Bdd, ReachingConditionError> {
        Ok(self.boolean_domain.or(left, right)?)
    }

    pub fn not(&self, value: Bdd) -> Result<Bdd, ReachingConditionError> {
        Ok(self.boolean_domain.not(value)?)
    }

    pub fn constrain(&self, value: Bdd, care: Bdd) -> Result<Bdd, ReachingConditionError> {
        Ok(self.boolean_domain.constrain(value, care)?)
    }

    pub fn implies(&self, premise: Bdd, consequence: Bdd) -> Result<bool, ReachingConditionError> {
        Ok(self.boolean_domain.implies_bdd(premise, consequence)?)
    }

    pub fn reaches(&self, source: BlockId, target: BlockId) -> bool {
        self.reachability.reaches(source, target)
    }

    pub fn any_reaches(&self, sources: &BTreeSet<BlockId>, targets: &BTreeSet<BlockId>) -> bool {
        self.reachability.any_reaches(
            sources.len(),
            sources.iter().filter_map(|source| self.rank(*source)),
            targets.len(),
            targets.iter().filter_map(|target| self.rank(*target)),
        )
    }

    pub(in crate::ir) fn any_rank_reaches(&self, sources: &[usize], targets: &[usize]) -> bool {
        self.reachability.any_reaches(
            sources.len(),
            sources.iter().copied(),
            targets.len(),
            targets.iter().copied(),
        )
    }
}

struct AcyclicTopology<'a> {
    cfg: &'a CFG,
}

impl AcyclicTopology<'_> {
    // The CFG already keeps incoming edges sorted by (source, kind), which
    // is also the canonical order for combining predecessor BDD terms.
    fn incoming<'a>(
        &'a self,
        block: BlockId,
        indices: &'a HashMap<BlockId, usize>,
    ) -> impl Iterator<Item = (BlockId, usize, EdgeKind)> + 'a {
        self.cfg
            .predecessors_with_kind(block)
            .iter()
            .copied()
            .filter(|(_, kind)| !kind.is_exception())
            .filter_map(|(source, kind)| indices.get(&source).map(|&index| (source, index, kind)))
    }

    fn sort(
        &self,
        entry: BlockId,
        nodes: impl ExactSizeIterator<Item = BlockId> + Clone,
    ) -> Result<(Vec<BlockId>, HashMap<BlockId, usize>), ReachingConditionError> {
        // Membership, indegree and final rank use the same keys throughout.
        let mut indegree: HashMap<_, _> = nodes.clone().map(|block| (block, 0)).collect();
        if !indegree.contains_key(&entry) {
            return Err(ReachingConditionError::MissingEntry(entry));
        }
        let mut ready = BinaryHeap::new();
        for block in nodes {
            let degree = self.incoming(block, &indegree).count();
            indegree.insert(block, degree);
            if degree == 0 {
                ready.push(Reverse(block));
            }
        }
        if ready.len() != 1 || ready.peek() != Some(&Reverse(entry)) {
            return Err(ReachingConditionError::MultipleEntries {
                expected: entry,
                actual: ready.peek().map(|&Reverse(block)| block),
            });
        }
        let mut order = Vec::with_capacity(indegree.len());
        while let Some(Reverse(block)) = ready.pop() {
            order.push(block);
            for target in self.cfg.normal_successors(block) {
                let Some(degree) = indegree.get_mut(&target) else {
                    continue;
                };
                *degree = degree
                    .checked_sub(1)
                    .ok_or(ReachingConditionError::MalformedTopology(target))?;
                if *degree == 0 {
                    ready.push(Reverse(target));
                }
            }
        }
        if order.len() != indegree.len() {
            return Err(ReachingConditionError::CyclicGraph);
        }
        // The degree table is dead after Kahn's traversal. Reuse its keys
        // for reaching-condition rows and all later rank queries.
        for (rank, block) in order.iter().copied().enumerate() {
            indegree.insert(block, rank);
        }
        Ok((order, indegree))
    }
}

struct ReachabilityMatrix {
    indices: HashMap<BlockId, usize>,
    row_words: usize,
    bits: Vec<u64>,
}

impl ReachabilityMatrix {
    const BYTE_LIMIT: usize = 8 * 1024 * 1024;

    fn analyze(
        cfg: &CFG,
        order: &[BlockId],
        indices: HashMap<BlockId, usize>,
    ) -> Result<Self, ReachingConditionError> {
        let words = order.len().div_ceil(u64::BITS as usize);
        let bytes = order
            .len()
            .checked_mul(words)
            .and_then(|cells| cells.checked_mul(std::mem::size_of::<u64>()))
            .ok_or(ReachingConditionError::ResourceLimit {
                resource: ReachingResource::Reachability,
                limit: Self::BYTE_LIMIT,
            })?;
        if bytes > Self::BYTE_LIMIT {
            return Err(ReachingConditionError::ResourceLimit {
                resource: ReachingResource::Reachability,
                limit: Self::BYTE_LIMIT,
            });
        }
        let mut bits = vec![0u64; bytes / std::mem::size_of::<u64>()];
        for (source_index, &source) in order.iter().enumerate().rev() {
            // A DAG successor has a strictly greater rank. Its completed row
            // and the source row therefore occupy disjoint parts of the table.
            let split = (source_index + 1) * words;
            let (prefix, suffix) = bits.split_at_mut(split);
            let row = &mut prefix[source_index * words..];
            for target_index in cfg
                .normal_successors(source)
                .filter_map(|target| indices.get(&target).copied())
            {
                row[target_index / u64::BITS as usize] |=
                    1u64 << (target_index % u64::BITS as usize);
                let start = (target_index - source_index - 1) * words;
                for (word, descendants) in row.iter_mut().zip(&suffix[start..start + words]) {
                    *word |= descendants;
                }
            }
        }
        Ok(Self {
            indices,
            row_words: words,
            bits,
        })
    }

    fn any_reaches(
        &self,
        source_count: usize,
        sources: impl Iterator<Item = usize>,
        target_count: usize,
        mut targets: impl Iterator<Item = usize>,
    ) -> bool {
        if source_count == 0 || target_count == 0 {
            return false;
        }
        let mut rows = sources.map(|rank| {
            let start = rank * self.row_words;
            &self.bits[start..start + self.row_words]
        });
        if source_count == 1 {
            return rows
                .next()
                .is_some_and(|row| targets.any(|bit| row[bit / 64] & (1 << (bit % 64)) != 0));
        }
        if target_count == 1 {
            return targets
                .next()
                .is_some_and(|bit| rows.any(|row| row[bit / 64] & (1 << (bit % 64)) != 0));
        }
        let mut mask = SmallVec::<[u64; 4]>::from_elem(0, self.row_words);
        for bit in targets {
            mask[bit / 64] |= 1 << (bit % 64);
        }
        rows.any(|row| row.iter().zip(&mask).any(|(word, mask)| word & mask != 0))
    }

    fn reaches(&self, source: BlockId, target: BlockId) -> bool {
        let (Some(source), Some(target)) = (
            self.indices.get(&source).copied(),
            self.indices.get(&target).copied(),
        ) else {
            return false;
        };
        self.bits[source * self.row_words + target / u64::BITS as usize]
            & (1u64 << (target % u64::BITS as usize))
            != 0
    }
}

#[derive(Clone, Copy)]
enum EdgeSource {
    Missing,
    Ordinary,
    Branch(usize),
    NamedBranch,
    Switch,
}

impl EdgeSource {
    fn compile(
        self,
        domain: &BddContext,
        source: BlockId,
        target: BlockId,
        edge: EdgeKind,
    ) -> Result<Bdd, ReachingConditionError> {
        match (self, edge) {
            (Self::Branch(index), EdgeKind::True | EdgeKind::False) => {
                Ok(domain.compile_index(index, edge == EdgeKind::True)?)
            }
            (Self::NamedBranch, EdgeKind::True | EdgeKind::False) => {
                Ok(domain.compile_literal(&BoolVariable::Block(source), edge == EdgeKind::True)?)
            }
            (Self::Branch(_) | Self::NamedBranch, _) => {
                Err(ReachingConditionError::MalformedBranchEdge {
                    source,
                    target,
                    edge,
                })
            }
            (Self::Switch, _) => Err(ReachingConditionError::UnownedSwitch(source)),
            (Self::Ordinary, EdgeKind::Normal) => Ok(domain.truth()),
            (Self::Ordinary, _) => Err(ReachingConditionError::UnexpectedEdge {
                source,
                target,
                edge,
            }),
            (Self::Missing, _) => Err(ReachingConditionError::MissingBlock(source)),
        }
    }
}

#[derive(Debug)]
pub enum ReachingConditionError {
    MissingEntry(BlockId),
    MissingBlock(BlockId),
    MissingPredecessorCondition {
        block: BlockId,
        predecessor: BlockId,
    },
    MultipleEntries {
        expected: BlockId,
        actual: Option<BlockId>,
    },
    MalformedTopology(BlockId),
    CyclicGraph,
    UnreachableBlock(BlockId),
    MalformedBranchEdge {
        source: BlockId,
        target: BlockId,
        edge: EdgeKind,
    },
    UnexpectedEdge {
        source: BlockId,
        target: BlockId,
        edge: EdgeKind,
    },
    UnownedSwitch(BlockId),
    ResourceLimit {
        resource: ReachingResource,
        limit: usize,
    },
    Bdd(BddError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachingResource {
    Reachability,
}

impl fmt::Display for ReachingConditionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEntry(block) => write!(formatter, "graph entry {block} is missing"),
            Self::MissingBlock(block) => write!(formatter, "graph block {block} is missing"),
            Self::MissingPredecessorCondition { block, predecessor } => write!(
                formatter,
                "predecessor {predecessor} of {block} has no reaching condition"
            ),
            Self::MultipleEntries { expected, actual } => write!(
                formatter,
                "acyclic graph starts at {actual:?}, expected unique entry {expected}"
            ),
            Self::MalformedTopology(block) => {
                write!(
                    formatter,
                    "acyclic graph has malformed indegree for {block}"
                )
            }
            Self::CyclicGraph => formatter.write_str("reaching-condition graph is cyclic"),
            Self::UnreachableBlock(block) => {
                write!(
                    formatter,
                    "block {block} has an unsatisfiable reaching condition"
                )
            }
            Self::MalformedBranchEdge {
                source,
                target,
                edge,
            } => write!(
                formatter,
                "conditional edge {source} -> {target} has kind {edge:?}"
            ),
            Self::UnexpectedEdge {
                source,
                target,
                edge,
            } => write!(
                formatter,
                "ordinary edge {source} -> {target} has unexpected kind {edge:?}"
            ),
            Self::UnownedSwitch(block) => write!(
                formatter,
                "switch block {block} reached the Boolean region structurer"
            ),
            Self::ResourceLimit { resource, limit } => {
                write!(formatter, "{resource:?} resource limit {limit} exceeded")
            }
            Self::Bdd(source) => write!(formatter, "Boolean decision diagram failed: {source}"),
        }
    }
}

impl std::error::Error for ReachingConditionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Bdd(source) => Some(source),
            _ => None,
        }
    }
}

impl ReachingConditionError {
    pub fn requires_graph_reduction(&self) -> bool {
        matches!(self, Self::UnownedSwitch(_))
            || matches!(self, Self::ResourceLimit { .. })
            || matches!(self, Self::Bdd(source) if source.is_resource_limit())
    }
}

impl From<BddError> for ReachingConditionError {
    fn from(source: BddError) -> Self {
        Self::Bdd(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Block, InsnNode};

    #[test]
    fn slice_uses_its_own_entry_condition() {
        let mut cfg = CFG::new("slice");
        for id in 0..=4 {
            cfg.add_block(Block::new(BlockId::new(id)));
        }
        cfg.block_mut(BlockId::new(1))
            .expect("branch block")
            .insns
            .push(InsnNode::new(InsnType::If, 0));
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::True);
        cfg.add_edge(BlockId::new(1), BlockId::new(3), EdgeKind::False);
        cfg.add_edge(BlockId::new(2), BlockId::new(4), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(3), BlockId::new(4), EdgeKind::Normal);

        let nodes = [1, 2, 3, 4]
            .into_iter()
            .map(BlockId::new)
            .collect::<BTreeSet<_>>();
        let facts =
            ReachingConditions::analyze_slice(&cfg, BlockId::new(1), &nodes).expect("local slice");

        assert!(facts
            .condition(BlockId::new(1))
            .expect("entry condition")
            .domain()
            .is_true());
        assert!(facts
            .condition(BlockId::new(4))
            .expect("join condition")
            .domain()
            .is_true());
    }
}
