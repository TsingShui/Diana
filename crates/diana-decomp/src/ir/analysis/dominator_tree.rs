//! Dominator Tree computation
//!
//! This module provides dominator tree computation for CFG analysis,
//! used by region ownership and semantic scheduling.

use super::ReachingConditions;
use crate::ir::{BlockId, CFG};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[cfg(test)]
#[path = "../../../tests/internal/dag_dominators.rs"]
mod dag_dominators;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DominanceError {
    MissingEntry(BlockId),
    MissingTopology(BlockId),
    InconsistentEdge { source: BlockId, target: BlockId },
    InvalidTraversal(BlockId),
    MissingDominator(BlockId),
    Unreachable(Vec<BlockId>),
}

impl std::fmt::Display for DominanceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEntry(entry) => write!(formatter, "dominance entry {entry} is absent"),
            Self::MissingTopology(block) => {
                write!(formatter, "dominance topology is missing block {block}")
            }
            Self::InconsistentEdge { source, target } => write!(
                formatter,
                "dominance topology contains an inconsistent edge {source} -> {target}"
            ),
            Self::InvalidTraversal(block) => {
                write!(formatter, "dominance traversal is malformed at {block}")
            }
            Self::MissingDominator(block) => {
                write!(formatter, "dominance algorithm did not resolve {block}")
            }
            Self::Unreachable(blocks) => {
                write!(
                    formatter,
                    "dominance topology has unreachable blocks {blocks:?}"
                )
            }
        }
    }
}

impl std::error::Error for DominanceError {}

/// Dominator Tree structure
#[derive(Debug, Clone)]
pub struct DominatorTree {
    entry: BlockId,
    /// Sorted by block identity, independently of graph traversal order.
    nodes: Arc<Vec<DominatorNode>>,
    virtual_child: Option<BlockId>,
}

#[derive(Debug, Clone)]
struct DominatorNode {
    block: BlockId,
    parent: Option<BlockId>,
    children: smallvec::SmallVec<[BlockId; 2]>,
    preorder: usize,
    subtree_end: usize,
}

impl DominatorTree {
    /// Compute exact dominance over every CFG edge using Lengauer-Tarjan.
    pub fn compute(cfg: &CFG) -> Result<Self, DominanceError> {
        Self::solve(cfg.entry, Topology::from_cfg::<false>(cfg)?, true)
    }

    pub(super) fn compute_postdominance(cfg: &CFG) -> Result<Self, DominanceError> {
        Self::solve(BlockId::INVALID, Topology::from_cfg::<true>(cfg)?, false)
    }

    /// With no exception edges every path originates at the method entry.
    /// Adding the normal-flow forest's virtual root changes only the parent of
    /// that entry. The real vertices keep their traversal and sibling order.
    pub(super) fn with_virtual_root(&self) -> Result<Self, DominanceError> {
        if self.contains(BlockId::INVALID) {
            return Err(DominanceError::MissingEntry(BlockId::INVALID));
        }
        Ok(Self {
            entry: BlockId::INVALID,
            nodes: Arc::clone(&self.nodes),
            virtual_child: Some(self.entry),
        })
    }

    /// Compute a dominator tree for an explicit topology snapshot.
    ///
    /// Structuring keeps original block bodies after removing graph-topology
    /// nodes, so callers can pass only the live graph nodes and a cached
    /// predecessor snapshot to avoid scanning preserved-but-dead blocks.
    pub fn compute_with_predecessors(
        cfg: &CFG,
        entry: BlockId,
        blocks: Vec<BlockId>,
        predecessors: &BTreeMap<BlockId, Vec<BlockId>>,
    ) -> Result<Self, DominanceError> {
        Self::compute_in(cfg, entry, blocks, predecessors)
    }

    /// Use the ordinary-edge order already established for this region's
    /// reaching conditions. In a DAG a node's immediate dominator is the
    /// deepest common dominator of its predecessors, all of which precede it.
    pub(in crate::ir) fn for_reaching_conditions(
        cfg: &CFG,
        facts: &ReachingConditions,
    ) -> Result<Self, DominanceError> {
        // The explicit-topology contract diagnoses a live exceptional edge
        // without an ordinary counterpart, or a root with absent topology.
        if cfg.has_exception_edges() || cfg.block(cfg.entry).is_none() {
            return Self::compute_with_predecessors(
                cfg,
                cfg.entry,
                cfg.graph_node_ids(),
                &cfg.normal_predecessor_snapshot(),
            );
        }
        let order = facts.order();
        if order.first() != Some(&cfg.entry) {
            return Err(DominanceError::MissingEntry(cfg.entry));
        }
        let mut ancestors = Vec::with_capacity(order.len());
        let mut nodes = Vec::with_capacity(order.len());
        for (rank, &block) in order.iter().enumerate() {
            let parent = if rank == 0 {
                ancestors.push(DagAncestor::default());
                None
            } else {
                let mut incoming = cfg
                    .normal_predecessors(block)
                    .filter_map(|source| facts.rank(source));
                let first = incoming
                    .next()
                    .ok_or(DominanceError::MissingDominator(block))?;
                if first >= rank {
                    return Err(DominanceError::InvalidTraversal(block));
                }
                let mut parent = first;
                for source in incoming {
                    if source >= rank {
                        return Err(DominanceError::InvalidTraversal(block));
                    }
                    parent = DagAncestor::common(&ancestors, parent, source);
                }
                ancestors.push(DagAncestor::child(&ancestors, parent));
                Some(order[parent])
            };
            nodes.push(DominatorNode {
                block,
                parent,
                children: Default::default(),
                preorder: 0,
                subtree_end: 0,
            });
        }
        Self::from_parents(cfg.entry, nodes)
    }

    /// Compute a dominance forest over ordinary control flow. A synthetic root
    /// connects the method entry and exception-dispatch entries that ordinary
    /// control flow cannot reach from the method entry.
    pub fn compute_normal(
        cfg: &CFG,
        mut blocks: Vec<BlockId>,
        predecessors: &BTreeMap<BlockId, Vec<BlockId>>,
    ) -> Result<Self, DominanceError> {
        // An explicit caller may supply missing rows in arbitrary order.
        if blocks.contains(&BlockId::INVALID) {
            return Err(DominanceError::MissingEntry(BlockId::INVALID));
        }
        for &block in &blocks {
            if !predecessors.contains_key(&block) {
                return Err(DominanceError::MissingTopology(block));
            }
        }
        blocks.sort_unstable();
        blocks.dedup();
        let topology = Topology::normal_forest(cfg, blocks, Some(predecessors))?;
        Self::solve(BlockId::INVALID, topology, true)
    }

    /// Ordinary dominance on the entire physical CFG. Its canonical
    /// successor relation supplies both halves of the numbered topology.
    pub(crate) fn normal(cfg: &CFG) -> Result<Self, DominanceError> {
        let topology = Topology::normal_forest(cfg, cfg.block_ids(), None)?;
        Self::solve(BlockId::INVALID, topology, true)
    }

    fn compute_in(
        cfg: &CFG,
        entry: BlockId,
        blocks: Vec<BlockId>,
        predecessors: &BTreeMap<BlockId, Vec<BlockId>>,
    ) -> Result<Self, DominanceError> {
        let block_set = blocks.iter().copied().collect::<BTreeSet<_>>();
        let successors = blocks
            .iter()
            .copied()
            .map(|block| {
                let targets = cfg
                    .successors_with_kind(block)
                    .iter()
                    .map(|(target, _)| *target)
                    .filter(|target| block_set.contains(target))
                    .collect();
                (block, targets)
            })
            .collect();
        Self::compute_topology(entry, blocks, predecessors, &successors)
    }

    pub(crate) fn compute_topology(
        entry: BlockId,
        blocks: Vec<BlockId>,
        predecessors: &BTreeMap<BlockId, Vec<BlockId>>,
        successors: &BTreeMap<BlockId, Vec<BlockId>>,
    ) -> Result<Self, DominanceError> {
        Self::solve(
            entry,
            Topology::new(blocks, predecessors, successors)?,
            true,
        )
    }

    fn solve(
        entry: BlockId,
        topology: Topology,
        require_all_reachable: bool,
    ) -> Result<Self, DominanceError> {
        let start = Topology::position(&topology.nodes, entry)
            .ok_or(DominanceError::MissingEntry(entry))?;
        if topology.nodes.len() == 1 {
            return Ok(Self {
                entry,
                virtual_child: None,
                nodes: Arc::new(vec![DominatorNode {
                    block: entry,
                    parent: None,
                    children: Default::default(),
                    preorder: 0,
                    subtree_end: 1,
                }]),
            });
        }
        let traversal = DepthFirstNumbering::compute(start, &topology);
        if require_all_reachable && traversal.vertices.len() != topology.nodes.len() {
            let unreachable = topology
                .nodes
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(index, block)| {
                    (traversal.indices[index] == usize::MAX).then_some(block)
                })
                .collect();
            return Err(DominanceError::Unreachable(unreachable));
        }

        let mut algorithm = LengauerTarjan::new(traversal, &topology);
        drop(topology);
        Self::from_parents(entry, algorithm.compute()?)
    }

    fn from_parents(entry: BlockId, mut nodes: Vec<DominatorNode>) -> Result<Self, DominanceError> {
        nodes.sort_unstable_by_key(|node| node.block);
        for i in 0..nodes.len() {
            let child = &nodes[i];
            if let Some(parent) = child.parent {
                let block = child.block;
                let parent_index = Self::position(&nodes, parent)
                    .ok_or(DominanceError::MissingDominator(parent))?;
                nodes[parent_index].children.push(block);
            }
        }
        // A subtree occupies a contiguous range in dominator-tree preorder.
        // Queries on a chain can use these bounds without following every parent.
        let mut next = 0;
        let root = Self::position(&nodes, entry).ok_or(DominanceError::MissingEntry(entry))?;
        let mut pending = vec![(root, false)];
        while let Some((index, exiting)) = pending.pop() {
            if exiting {
                nodes[index].subtree_end = next;
            } else {
                nodes[index].preorder = next;
                next += 1;
                pending.push((index, true));
                for &child in nodes[index].children.iter().rev() {
                    let child = Self::position(&nodes, child)
                        .ok_or(DominanceError::MissingDominator(child))?;
                    pending.push((child, false));
                }
            }
        }
        Ok(Self {
            entry,
            nodes: Arc::new(nodes),
            virtual_child: None,
        })
    }

    fn index_of(&self, block: BlockId) -> Option<usize> {
        Self::position(&self.nodes, block)
    }

    fn position(nodes: &[DominatorNode], block: BlockId) -> Option<usize> {
        let index = block.raw() as usize;
        if nodes.get(index).is_some_and(|node| node.block == block) {
            return Some(index);
        }
        nodes.binary_search_by_key(&block, |node| node.block).ok()
    }

    /// Get immediate dominator of a node.
    pub fn idom(&self, node: BlockId) -> Option<BlockId> {
        if self.virtual_child == Some(node) {
            return Some(BlockId::INVALID);
        }
        self.nodes[self.index_of(node)?].parent
    }

    /// Get children of a node in block identity order.
    pub fn children(&self, node: BlockId) -> impl DoubleEndedIterator<Item = BlockId> + '_ {
        let children = if node == BlockId::INVALID && self.virtual_child.is_some() {
            self.virtual_child.as_slice()
        } else {
            self.index_of(node)
                .map_or(&[][..], |i| self.nodes[i].children.as_slice())
        };
        children.iter().copied()
    }

    pub(super) fn len(&self) -> usize {
        self.nodes.len() + usize::from(self.virtual_child.is_some())
    }

    pub fn contains(&self, node: BlockId) -> bool {
        (node == BlockId::INVALID && self.virtual_child.is_some()) || self.index_of(node).is_some()
    }

    /// Get all nodes dominated by the given node (including itself)
    pub fn dominated_by(&self, node: BlockId) -> BTreeSet<BlockId> {
        if !self.contains(node) {
            return BTreeSet::new();
        }
        let mut result = BTreeSet::new();
        let mut pending = vec![node];
        while let Some(current) = pending.pop() {
            if result.insert(current) {
                pending.extend(self.children(current));
            }
        }
        result
    }

    /// Check if `dom` dominates `node`
    pub fn dominates(&self, dom: BlockId, node: BlockId) -> bool {
        if dom == BlockId::INVALID && self.virtual_child.is_some() {
            return self.contains(node);
        }
        let (Some(dom), Some(node)) = (self.index_of(dom), self.index_of(node)) else {
            return false;
        };
        (self.nodes[dom].preorder..self.nodes[dom].subtree_end).contains(&self.nodes[node].preorder)
    }

    /// Check if `sdom` strictly dominates `node` (dominates but not equal)
    pub fn strictly_dominates(&self, sdom: BlockId, node: BlockId) -> bool {
        sdom != node && self.dominates(sdom, node)
    }

    /// Get entry node
    pub fn entry(&self) -> BlockId {
        self.entry
    }

    /// Get all nodes in dominator tree preorder
    pub fn preorder(&self) -> Vec<BlockId> {
        let mut result = Vec::new();
        let mut pending = vec![self.entry];
        while let Some(node) = pending.pop() {
            result.push(node);
            pending.extend(self.children(node).rev());
        }
        result
    }

    /// Get all nodes in dominator tree postorder
    pub fn postorder(&self) -> Vec<BlockId> {
        let mut result = Vec::new();
        let mut pending = vec![(self.entry, false)];
        while let Some((node, exiting)) = pending.pop() {
            if exiting {
                result.push(node);
                continue;
            }
            pending.push((node, true));
            pending.extend(self.children(node).rev().map(|child| (child, false)));
        }
        result
    }
}

/// One level-ancestor jump per node keeps the online tree linear in space.
/// Joining two successive jumps of equal length makes long ancestor walks
/// skip whole levels while retaining a parent step for the final approach.
#[derive(Default)]
struct DagAncestor {
    parent: usize,
    jump: usize,
    depth: usize,
}

impl DagAncestor {
    fn child(tree: &[Self], parent: usize) -> Self {
        let p = &tree[parent];
        let j = &tree[p.jump];
        let jj = &tree[j.jump];
        let jump = if p.depth - j.depth == j.depth - jj.depth {
            j.jump
        } else {
            parent
        };
        Self {
            parent,
            jump,
            depth: p.depth + 1,
        }
    }

    fn common(tree: &[Self], mut left: usize, mut right: usize) -> usize {
        if tree[left].depth < tree[right].depth {
            std::mem::swap(&mut left, &mut right);
        }
        while tree[left].depth > tree[right].depth {
            let jump = tree[left].jump;
            left = if tree[jump].depth >= tree[right].depth {
                jump
            } else {
                tree[left].parent
            };
        }
        while left != right {
            if tree[left].jump != tree[right].jump {
                left = tree[left].jump;
                right = tree[right].jump;
            } else {
                left = tree[left].parent;
                right = tree[right].parent;
            }
        }
        left
    }
}

type Neighbors = smallvec::SmallVec<[usize; 2]>;

struct Topology {
    nodes: Vec<BlockId>,
    predecessors: Vec<Neighbors>,
    successors: Vec<Neighbors>,
}

impl Topology {
    fn position(nodes: &[BlockId], block: BlockId) -> Option<usize> {
        let direct = block.raw() as usize;
        if nodes.get(direct) == Some(&block) {
            Some(direct)
        } else {
            nodes.binary_search(&block).ok()
        }
    }

    fn new(
        mut nodes: Vec<BlockId>,
        predecessors: &BTreeMap<BlockId, Vec<BlockId>>,
        successors: &BTreeMap<BlockId, Vec<BlockId>>,
    ) -> Result<Self, DominanceError> {
        nodes.sort_unstable();
        nodes.dedup();
        let mut incoming = Vec::with_capacity(nodes.len());
        let mut outgoing = Vec::with_capacity(nodes.len());
        for &node in &nodes {
            let neighbors = |edges: &BTreeMap<BlockId, Vec<BlockId>>| {
                let mut indices: Neighbors = edges
                    .get(&node)
                    .ok_or(DominanceError::MissingTopology(node))?
                    .iter()
                    .filter_map(|&neighbor| Self::position(&nodes, neighbor))
                    .collect();
                indices.sort_unstable();
                indices.dedup();
                Ok::<_, DominanceError>(indices)
            };
            outgoing.push(neighbors(successors)?);
            incoming.push(neighbors(predecessors)?);
        }
        let topology = Self {
            nodes,
            predecessors: incoming,
            successors: outgoing,
        };
        topology.check_inverse()?;
        Ok(topology)
    }

    fn check_inverse(&self) -> Result<(), DominanceError> {
        for (source, targets) in self.successors.iter().enumerate() {
            for &target in targets {
                if self.predecessors[target].binary_search(&source).is_err() {
                    return Err(DominanceError::InconsistentEdge {
                        source: self.nodes[source],
                        target: self.nodes[target],
                    });
                }
            }
        }
        for (target, sources) in self.predecessors.iter().enumerate() {
            for &source in sources {
                if self.successors[source].binary_search(&target).is_err() {
                    return Err(DominanceError::InconsistentEdge {
                        source: self.nodes[source],
                        target: self.nodes[target],
                    });
                }
            }
        }
        Ok(())
    }

    fn normal_forest(
        cfg: &CFG,
        mut nodes: Vec<BlockId>,
        supplied: Option<&BTreeMap<BlockId, Vec<BlockId>>>,
    ) -> Result<Self, DominanceError> {
        let root = nodes.len();
        if nodes.last() == Some(&BlockId::INVALID) {
            return Err(DominanceError::MissingEntry(BlockId::INVALID));
        }
        let entry =
            Self::position(&nodes, cfg.entry).ok_or(DominanceError::MissingTopology(cfg.entry))?;
        nodes.push(BlockId::INVALID);
        let mut incoming = vec![Neighbors::new(); nodes.len()];
        let mut outgoing = Vec::with_capacity(nodes.len());
        let mut roots = vec![false; root];
        for (source, &block) in nodes[..root].iter().enumerate() {
            let mut targets = Neighbors::new();
            for &(target, kind) in cfg.successors_with_kind(block) {
                if let Some(index) = Self::position(&nodes[..root], target) {
                    if kind.is_exception() {
                        roots[index] = true;
                    } else {
                        targets.push(index);
                    }
                }
            }
            targets.sort_unstable();
            targets.dedup();
            if let Some(snapshot) = supplied {
                let row = &mut incoming[source];
                row.extend(
                    snapshot[&block]
                        .iter()
                        .filter_map(|&id| Self::position(&nodes, id)),
                );
                row.sort_unstable();
                row.dedup();
            } else {
                for &target in &targets {
                    incoming[target].push(source);
                }
            }
            outgoing.push(targets);
        }
        // Only exception entries outside ordinary entry reachability need a
        // virtual edge. The method entry itself is always a forest root.
        let mut visited = vec![false; root];
        let mut pending = vec![entry];
        visited[entry] = true;
        while let Some(source) = pending.pop() {
            roots[source] = false;
            for &target in &outgoing[source] {
                if !std::mem::replace(&mut visited[target], true) {
                    pending.push(target);
                }
            }
        }
        roots[entry] = true;
        let mut entries = Neighbors::new();
        for (index, is_root) in roots.into_iter().enumerate() {
            if is_root {
                entries.push(index);
                if incoming[index].last() != Some(&root) {
                    incoming[index].push(root);
                }
            }
        }
        outgoing.push(entries);
        let topology = Self {
            nodes,
            predecessors: incoming,
            successors: outgoing,
        };
        if supplied.is_some() {
            topology.check_inverse()?;
        }
        Ok(topology)
    }

    /// Physical edges are the owning CFG's canonical relation. Number each
    /// distinct target once and build the transpose directly from that row.
    fn from_cfg<const POST: bool>(cfg: &CFG) -> Result<Self, DominanceError> {
        let mut nodes = cfg.block_ids();
        let physical = nodes.len();
        if POST && nodes.last() != Some(&BlockId::INVALID) {
            nodes.push(BlockId::INVALID);
        }
        let mut outgoing = Vec::with_capacity(nodes.len());
        let mut incoming = vec![Neighbors::new(); nodes.len()];
        let mut exits = Neighbors::new();
        let mut physical_exit_reference = false;
        for (source, &block) in nodes.iter().enumerate() {
            let mut targets = Neighbors::new();
            if source < physical {
                let mut has_successor = false;
                for &(target, kind) in cfg.successors_with_kind(block) {
                    if POST && kind.is_exception() {
                        continue;
                    }
                    has_successor = true;
                    if let Some(target_index) = Self::position(&nodes, target) {
                        targets.push(target_index);
                        if POST && target == BlockId::INVALID {
                            physical_exit_reference = true;
                        }
                    }
                }
                if POST && !has_successor {
                    targets.push(nodes.len() - 1);
                    exits.push(source);
                }
            }
            targets.sort_unstable();
            targets.dedup();
            for &target in &targets {
                incoming[target].push(source);
            }
            outgoing.push(targets);
        }
        let mut topology = Self {
            nodes,
            predecessors: incoming,
            successors: outgoing,
        };
        if POST {
            std::mem::swap(&mut topology.predecessors, &mut topology.successors);
            // The virtual exit dispatches only to physical blocks with no
            // normal successor, including when their sole edge is exceptional.
            *topology
                .successors
                .last_mut()
                .expect("virtual exit present") = exits;
            if physical_exit_reference {
                // A caller-supplied edge using the reserved exit ID may
                // conflict with the virtual row. Preserve the classified error.
                topology.check_inverse()?;
            }
        }
        Ok(topology)
    }
}

struct DepthFirstNumbering {
    vertices: Vec<usize>,
    indices: Vec<usize>,
    parents: Vec<Option<usize>>,
}

impl DepthFirstNumbering {
    fn compute(entry: usize, topology: &Topology) -> Self {
        let mut vertices = vec![entry];
        let mut indices = vec![usize::MAX; topology.nodes.len()];
        indices[entry] = 0;
        let mut parents = vec![None];
        let mut pending = vec![(entry, 0usize)];
        while let Some((node, next_child)) = pending.last_mut() {
            let targets = &topology.successors[*node];
            if *next_child == targets.len() {
                pending.pop();
                continue;
            }
            let target = targets[*next_child];
            *next_child += 1;
            if indices[target] == usize::MAX {
                parents.push(Some(indices[*node]));
                indices[target] = vertices.len();
                vertices.push(target);
                pending.push((target, 0));
            }
        }
        Self {
            vertices,
            indices,
            parents,
        }
    }
}

struct LengauerTarjan {
    vertices: Vec<BlockId>,
    predecessors: Vec<Neighbors>,
    parents: Vec<Option<usize>>,
    semi: Vec<usize>,
    labels: Vec<usize>,
    ancestors: Vec<Option<usize>>,
    buckets: Vec<Vec<usize>>,
    immediate: Vec<Option<usize>>,
}

impl LengauerTarjan {
    fn new(traversal: DepthFirstNumbering, topology: &Topology) -> Self {
        let count = traversal.vertices.len();
        let predecessor_indices = traversal
            .vertices
            .iter()
            .map(|&node| {
                topology.predecessors[node]
                    .iter()
                    .map(|&source| traversal.indices[source])
                    .filter(|&index| index != usize::MAX)
                    .collect()
            })
            .collect();
        Self {
            vertices: traversal
                .vertices
                .into_iter()
                .map(|node| topology.nodes[node])
                .collect(),
            predecessors: predecessor_indices,
            parents: traversal.parents,
            semi: (0..count).collect(),
            labels: (0..count).collect(),
            ancestors: vec![None; count],
            buckets: vec![Vec::new(); count],
            immediate: vec![None; count],
        }
    }

    fn compute(&mut self) -> Result<Vec<DominatorNode>, DominanceError> {
        for node in (1..self.vertices.len()).rev() {
            for i in 0..self.predecessors[node].len() {
                let predecessor = self.predecessors[node][i];
                let representative = self.evaluate(predecessor)?;
                self.semi[node] = self.semi[node].min(self.semi[representative]);
            }
            self.buckets[self.semi[node]].push(node);
            let parent =
                self.parents[node].ok_or(DominanceError::InvalidTraversal(self.vertices[node]))?;
            self.ancestors[node] = Some(parent);
            for member in std::mem::take(&mut self.buckets[parent]) {
                let representative = self.evaluate(member)?;
                self.immediate[member] = Some(if self.semi[representative] < self.semi[member] {
                    representative
                } else {
                    parent
                });
            }
        }
        for node in 1..self.vertices.len() {
            if self.immediate[node] != Some(self.semi[node]) {
                let provisional = self.immediate[node]
                    .ok_or(DominanceError::MissingDominator(self.vertices[node]))?;
                self.immediate[node] = self.immediate[provisional];
            }
        }
        Ok(self
            .vertices
            .iter()
            .enumerate()
            .map(|(i, &block)| DominatorNode {
                block,
                parent: self.immediate[i].map(|parent| self.vertices[parent]),
                children: Default::default(),
                preorder: 0,
                subtree_end: 0,
            })
            .collect())
    }

    fn evaluate(&mut self, node: usize) -> Result<usize, DominanceError> {
        if self.ancestors[node].is_none() {
            return Ok(self.labels[node]);
        }
        let mut path = Vec::new();
        let mut current = node;
        while let Some(parent) = self.ancestors[current] {
            if self.ancestors[parent].is_none() {
                break;
            }
            path.push(current);
            current = parent;
        }
        for member in path.into_iter().rev() {
            let parent = self.ancestors[member]
                .ok_or(DominanceError::InvalidTraversal(self.vertices[member]))?;
            if self.semi[self.labels[parent]] < self.semi[self.labels[member]] {
                self.labels[member] = self.labels[parent];
            }
            self.ancestors[member] = self.ancestors[parent];
        }
        Ok(self.labels[node])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::block::Block;
    use crate::ir::cfg::EdgeKind;

    fn make_cfg_diamond() -> CFG {
        // Diamond pattern:
        //     0 (entry)
        //    / \
        //   1   2
        //    \ /
        //     3
        let mut cfg = CFG::new("diamond");
        cfg.add_block(Block::new(BlockId::new(0)));
        cfg.add_block(Block::new(BlockId::new(1)));
        cfg.add_block(Block::new(BlockId::new(2)));
        cfg.add_block(Block::new(BlockId::new(3)));

        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::True);
        cfg.add_edge(BlockId::new(0), BlockId::new(2), EdgeKind::False);
        cfg.add_edge(BlockId::new(1), BlockId::new(3), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(2), BlockId::new(3), EdgeKind::Normal);

        cfg.entry = BlockId::new(0);
        cfg
    }

    fn make_cfg_while_loop() -> CFG {
        // While loop:
        //   0 (entry)
        //   |
        //   1 (header) <----
        //  / \              |
        // 2   3 (body) -----
        //
        let mut cfg = CFG::new("while");
        cfg.add_block(Block::new(BlockId::new(0)));
        cfg.add_block(Block::new(BlockId::new(1)));
        cfg.add_block(Block::new(BlockId::new(2)));
        cfg.add_block(Block::new(BlockId::new(3)));

        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::False); // exit
        cfg.add_edge(BlockId::new(1), BlockId::new(3), EdgeKind::True); // body
        cfg.add_edge(BlockId::new(3), BlockId::new(1), EdgeKind::Normal); // back edge

        cfg.entry = BlockId::new(0);
        cfg
    }

    fn make_cfg_nested_if() -> CFG {
        // Nested if:
        //      0
        //     / \
        //    1   4
        //   / \
        //  2   3
        //   \ /
        //    4
        let mut cfg = CFG::new("nested_if");
        cfg.add_block(Block::new(BlockId::new(0)));
        cfg.add_block(Block::new(BlockId::new(1)));
        cfg.add_block(Block::new(BlockId::new(2)));
        cfg.add_block(Block::new(BlockId::new(3)));
        cfg.add_block(Block::new(BlockId::new(4)));

        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::True);
        cfg.add_edge(BlockId::new(0), BlockId::new(4), EdgeKind::False);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::True);
        cfg.add_edge(BlockId::new(1), BlockId::new(3), EdgeKind::False);
        cfg.add_edge(BlockId::new(2), BlockId::new(4), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(3), BlockId::new(4), EdgeKind::Normal);

        cfg.entry = BlockId::new(0);
        cfg
    }

    #[test]
    fn test_domtree_diamond() {
        let cfg = make_cfg_diamond();
        let dom = DominatorTree::compute(&cfg).unwrap();

        // 0 dominates all
        assert!(dom.dominates(BlockId::new(0), BlockId::new(0)));
        assert!(dom.dominates(BlockId::new(0), BlockId::new(1)));
        assert!(dom.dominates(BlockId::new(0), BlockId::new(2)));
        assert!(dom.dominates(BlockId::new(0), BlockId::new(3)));

        // 1 and 2 only dominate themselves
        assert!(dom.dominates(BlockId::new(1), BlockId::new(1)));
        assert!(!dom.dominates(BlockId::new(1), BlockId::new(3)));
        assert!(dom.dominates(BlockId::new(2), BlockId::new(2)));
        assert!(!dom.dominates(BlockId::new(2), BlockId::new(3)));

        // idom of 3 is 0 (not 1 or 2)
        assert_eq!(dom.idom(BlockId::new(3)), Some(BlockId::new(0)));
    }

    #[test]
    fn test_domtree_while_loop() {
        let cfg = make_cfg_while_loop();
        let dom = DominatorTree::compute(&cfg).unwrap();

        // 0 dominates all
        assert!(dom.dominates(BlockId::new(0), BlockId::new(1)));
        assert!(dom.dominates(BlockId::new(0), BlockId::new(2)));
        assert!(dom.dominates(BlockId::new(0), BlockId::new(3)));

        // 1 (header) dominates 2 and 3
        assert!(dom.dominates(BlockId::new(1), BlockId::new(2)));
        assert!(dom.dominates(BlockId::new(1), BlockId::new(3)));

        // idom of 1 is 0
        assert_eq!(dom.idom(BlockId::new(1)), Some(BlockId::new(0)));
        // idom of 3 is 1
        assert_eq!(dom.idom(BlockId::new(3)), Some(BlockId::new(1)));
    }

    #[test]
    fn test_domtree_nested_if() {
        let cfg = make_cfg_nested_if();
        let dom = DominatorTree::compute(&cfg).unwrap();

        // 0 dominates all
        for i in 0..5 {
            assert!(dom.dominates(BlockId::new(0), BlockId::new(i)));
        }

        // 1 dominates 2 and 3
        assert!(dom.dominates(BlockId::new(1), BlockId::new(2)));
        assert!(dom.dominates(BlockId::new(1), BlockId::new(3)));
        // 1 does not dominate 4 (multiple paths to 4)
        assert!(!dom.dominates(BlockId::new(1), BlockId::new(4)));
    }

    #[test]
    fn test_dominated_by() {
        let cfg = make_cfg_nested_if();
        let dom = DominatorTree::compute(&cfg).unwrap();

        // Nodes dominated by 1
        let dom_by_1 = dom.dominated_by(BlockId::new(1));
        assert!(dom_by_1.contains(&BlockId::new(1)));
        assert!(dom_by_1.contains(&BlockId::new(2)));
        assert!(dom_by_1.contains(&BlockId::new(3)));
        assert!(!dom_by_1.contains(&BlockId::new(4)));
    }

    #[test]
    fn normal_flow_ignores_exception_shortcuts_and_back_edges() {
        let mut cfg = CFG::new("normal_flow_with_exception_edges");
        for id in 0..3 {
            cfg.add_block(Block::new(BlockId::new(id)));
        }
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(0), BlockId::new(2), EdgeKind::Exception);
        cfg.add_edge(BlockId::new(2), BlockId::new(1), EdgeKind::Exception);

        let predecessors = cfg.normal_predecessor_snapshot();
        let dom = DominatorTree::compute_normal(&cfg, cfg.block_ids(), &predecessors).unwrap();
        assert_eq!(dom.idom(BlockId::new(2)), Some(BlockId::new(1)));
    }

    #[test]
    fn test_preorder_postorder() {
        let cfg = make_cfg_diamond();
        let dom = DominatorTree::compute(&cfg).unwrap();

        let pre = dom.preorder();
        let post = dom.postorder();

        // Entry should be first in preorder, last in postorder
        assert_eq!(pre[0], BlockId::new(0));
        assert_eq!(*post.last().unwrap(), BlockId::new(0));

        // Both should have all nodes
        assert_eq!(pre.len(), 4);
        assert_eq!(post.len(), 4);
    }
}
