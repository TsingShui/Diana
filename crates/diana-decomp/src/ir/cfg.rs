//! Control Flow Graph
//!
//! CFG with separated edge storage.
//!
//! Uses BTreeMap/BTreeSet for deterministic iteration order.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::block::{Block, BlockId, ExceptionHandler};
use super::{ArgType, InstructionId, MethodDescriptor};

/// Edge type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EdgeKind {
    Normal,
    True,
    False,
    SwitchCase(i32),
    SwitchDefault,
    Exception,
}

impl EdgeKind {
    pub fn is_exception(self) -> bool {
        self == Self::Exception
    }

    pub fn is_switch_dispatch(self) -> bool {
        matches!(self, Self::SwitchCase(_) | Self::SwitchDefault)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodContext {
    owner: ArgType,
    name: String,
    descriptor: MethodDescriptor,
    is_static: bool,
    declared_synchronized: bool,
}

impl MethodContext {
    pub fn new(
        owner: ArgType,
        name: impl Into<String>,
        descriptor: MethodDescriptor,
        is_static: bool,
    ) -> Self {
        Self {
            owner,
            name: name.into(),
            descriptor,
            is_static,
            declared_synchronized: false,
        }
    }

    pub fn synthetic(name: impl Into<String>) -> Self {
        Self::new(
            ArgType::object("java/lang/Object"),
            name,
            MethodDescriptor {
                parameters: Vec::new(),
                return_type: ArgType::VOID,
            },
            true,
        )
    }

    pub fn owner(&self) -> &ArgType {
        &self.owner
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn descriptor(&self) -> &MethodDescriptor {
        &self.descriptor
    }

    pub fn is_static(&self) -> bool {
        self.is_static
    }

    pub fn with_declared_synchronization(mut self, declared: bool) -> Self {
        self.declared_synchronized = declared;
        self
    }

    pub fn is_declared_synchronized(&self) -> bool {
        self.declared_synchronized
    }
}

/// Control Flow Graph
#[derive(Debug, Clone)]
pub struct CFG {
    method: MethodContext,
    label: String,
    /// Entry block
    pub entry: BlockId,
    /// Blocks (ordered by BlockId for deterministic iteration)
    pub blocks: BTreeMap<BlockId, Block>,
    /// Successors: block -> [(successor, edge_kind)] (ordered by BlockId)
    successors: BTreeMap<BlockId, Vec<(BlockId, EdgeKind)>>,
    /// Reverse of the successor table, sorted by (source, kind).
    incoming: BTreeMap<BlockId, Vec<(BlockId, EdgeKind)>>,
    /// Exception handlers
    pub handlers: Vec<ExceptionHandler>,
    /// DEX protected-range membership captured before CFG topology edits.
    exception_coverage: BTreeMap<BlockId, BTreeSet<(u32, u32)>>,
    coverage_captured: bool,
    /// Register count
    pub registers: u32,
    /// Input register count
    pub ins: u32,
    /// Code variable for implicit `this`, if this method is not static.
    this_code_var: Option<u32>,
    /// Code variables for source-level method parameters in declaration order.
    param_code_vars: Vec<Option<u32>>,
    /// Optional debug info (local variable names + line numbers) from the DEX
    /// `debug_info_item`. Used by the renderer to name locals from their
    /// original source names when present.
    pub debug_info: Option<crate::frontend::DebugInfo>,
    /// True after the canonical CFG/SSA pipeline has prepared this graph.
    analysis_prepared: bool,
}

impl CFG {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            method: MethodContext::synthetic(name.clone()),
            label: name,
            entry: BlockId::new(0),
            blocks: BTreeMap::new(),
            successors: BTreeMap::new(),
            incoming: BTreeMap::new(),
            handlers: Vec::new(),
            exception_coverage: BTreeMap::new(),
            coverage_captured: false,
            registers: 0,
            ins: 0,
            this_code_var: None,
            param_code_vars: Vec::new(),
            debug_info: None,
            analysis_prepared: false,
        }
    }

    /// Freezes stable instruction identities before region ownership facts are
    /// built. Later synthetic instructions deliberately remain unowned.
    pub fn identify_instructions(&mut self) {
        for (next, instruction) in self
            .blocks
            .values_mut()
            .flat_map(|block| block.insns.iter_mut())
            .enumerate()
        {
            instruction.id = InstructionId::new(next);
        }
    }

    pub fn is_analysis_prepared(&self) -> bool {
        self.analysis_prepared
    }

    pub(crate) fn mark_analysis_prepared(&mut self) {
        self.analysis_prepared = true;
    }

    pub fn this_code_variable(&self) -> Option<u32> {
        self.this_code_var
    }

    pub fn parameter_code_variables(&self) -> &[Option<u32>] {
        &self.param_code_vars
    }

    pub(crate) fn set_source_variables(
        &mut self,
        this_variable: Option<u32>,
        parameter_variables: Vec<Option<u32>>,
    ) {
        self.this_code_var = this_variable;
        self.param_code_vars = parameter_variables;
    }

    pub fn with_method(method: MethodContext) -> Self {
        let label = method.name().to_string();
        Self {
            method,
            label,
            ..Self::new("")
        }
    }

    pub fn method(&self) -> &MethodContext {
        &self.method
    }

    pub fn set_method(&mut self, method: MethodContext) {
        self.label = method.name().to_string();
        self.method = method;
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }

    pub fn subgraph(&self, label: impl Into<String>) -> Self {
        let mut graph = Self::with_method(self.method.clone());
        graph.set_label(label);
        graph
    }

    // === Block access ===

    pub fn block(&self, id: impl Into<BlockId>) -> Option<&Block> {
        self.blocks.get(&id.into())
    }

    pub fn block_mut(&mut self, id: impl Into<BlockId>) -> Option<&mut Block> {
        self.blocks.get_mut(&id.into())
    }

    pub fn entry_block(&self) -> Option<&Block> {
        self.blocks.get(&self.entry)
    }

    /// Get all block IDs in sorted order.
    pub fn block_ids(&self) -> Vec<BlockId> {
        self.blocks.keys().copied().collect()
    }

    /// Iterate over blocks in sorted order.
    pub fn blocks_iter(&self) -> impl Iterator<Item = &Block> {
        self.blocks.values()
    }

    pub fn num_blocks(&self) -> usize {
        self.blocks.len()
    }

    /// Count the number of nodes that are part of the graph structure.
    ///
    /// This counts blocks that have entries in the successors map (i.e., have edges).
    /// Used by structuring algorithm to track progress, since remove_block_from_graph
    /// only removes edges but keeps block content.
    pub fn num_graph_nodes(&self) -> usize {
        self.successors.len()
    }

    /// Get all block IDs that are part of the graph structure.
    ///
    /// Returns only blocks that have entries in the successors map.
    pub fn graph_node_ids(&self) -> Vec<BlockId> {
        self.graph_nodes().collect()
    }

    pub(crate) fn graph_nodes(&self) -> impl ExactSizeIterator<Item = BlockId> + Clone + '_ {
        self.successors.keys().copied()
    }

    /// Remove graph-topology entries that are no longer reachable from the entry.
    ///
    /// This preserves block contents for code generation, unlike `PruneUnreachable`,
    /// and is intended for reduction algorithms that temporarily collapse CFG
    /// topology while still needing the original blocks later.
    pub fn prune_unreachable_graph_nodes(&mut self) -> Vec<BlockId> {
        let reachable = self.reachable();
        let to_remove: Vec<_> = self
            .successors
            .keys()
            .copied()
            .filter(|id| !reachable.contains(id))
            .collect();

        for id in &to_remove {
            self.remove_block_from_graph(*id);
        }

        to_remove
    }

    /// Check if a block is part of the graph structure.
    ///
    /// After `remove_block_from_graph`, the block content still exists but it's
    /// no longer part of the graph topology.
    pub fn is_graph_node(&self, id: BlockId) -> bool {
        self.successors.contains_key(&id)
    }

    pub fn add_block(&mut self, block: Block) {
        let id = block.id;
        self.blocks.insert(id, block);
        self.successors.entry(id).or_default();
    }

    /// Remove a block from the CFG, along with all edges to/from it.
    pub fn remove_block(&mut self, id: BlockId) {
        self.remove_block_from_graph(id);
        self.blocks.remove(&id);
        self.exception_coverage.remove(&id);
    }

    pub fn capture_exception_coverage(&mut self) {
        if self.coverage_captured {
            return;
        }
        let mut coverage: BTreeMap<_, _> = self
            .blocks
            .keys()
            .map(|&id| (id, BTreeSet::new()))
            .collect();
        if !self.handlers.is_empty() {
            let mut sites: Vec<_> = self
                .blocks
                .iter()
                .flat_map(|(&block, body)| body.insns.iter().map(move |insn| (insn.offset, block)))
                .collect();
            sites.sort_unstable();
            let ranges: BTreeSet<_> = self
                .handlers
                .iter()
                .map(|handler| (handler.start, handler.end))
                .collect();
            for (start, end) in ranges {
                let first = sites.partition_point(|&(offset, _)| offset < start);
                for &(_, block) in sites[first..]
                    .iter()
                    .take_while(|(offset, _)| *offset < end)
                {
                    coverage.entry(block).or_default().insert((start, end));
                }
            }
        }
        self.exception_coverage = coverage;
        self.coverage_captured = true;
    }

    pub fn exception_coverage(&self, block: BlockId) -> &BTreeSet<(u32, u32)> {
        self.exception_coverage
            .get(&block)
            .unwrap_or_else(|| Self::empty_coverage())
    }

    pub fn set_exception_coverage(&mut self, block: BlockId, ranges: BTreeSet<(u32, u32)>) {
        self.exception_coverage.insert(block, ranges);
    }

    pub fn common_exception_coverage(&self, left: BlockId, right: BlockId) -> BTreeSet<(u32, u32)> {
        self.exception_coverage(left)
            .intersection(self.exception_coverage(right))
            .copied()
            .collect()
    }

    pub fn exception_coverage_for(&self, instructions: &[super::InsnNode]) -> BTreeSet<(u32, u32)> {
        self.handlers
            .iter()
            .filter(|handler| {
                instructions
                    .iter()
                    .any(|instruction| handler.covers(instruction.offset))
            })
            .map(|handler| (handler.start, handler.end))
            .collect()
    }

    pub fn has_exception_coverage(&self, block: BlockId) -> bool {
        self.coverage_captured && self.exception_coverage.contains_key(&block)
    }

    fn empty_coverage() -> &'static BTreeSet<(u32, u32)> {
        static EMPTY: std::sync::OnceLock<BTreeSet<(u32, u32)>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(BTreeSet::new)
    }

    /// Remove a block from the graph structure (edges only), but keep its content.
    ///
    /// This is used by the structuring algorithm which needs to modify the graph
    /// topology during iterative reduction, but the block content must be preserved
    /// for code generation after structuring is complete.
    pub fn remove_block_from_graph(&mut self, id: BlockId) {
        for (to, kind) in self.successors.remove(&id).unwrap_or_default() {
            Self::forget_incoming(&mut self.incoming, to, (id, kind));
        }
        for (from, kind) in self.incoming.remove(&id).unwrap_or_default() {
            if let Some(edges) = self.successors.get_mut(&from) {
                edges.retain(|edge| *edge != (id, kind));
            }
        }
    }

    // === Edge operations ===

    pub fn add_edge(&mut self, from: BlockId, to: BlockId, kind: EdgeKind) {
        let successors = self.successors.entry(from).or_default();
        if !successors.contains(&(to, kind)) {
            successors.push((to, kind));
            let incoming = self.incoming.entry(to).or_default();
            let index = incoming.partition_point(|edge| *edge < (from, kind));
            incoming.insert(index, (from, kind));
        }
    }

    pub fn successors(&self, id: impl Into<BlockId>) -> impl Iterator<Item = BlockId> + '_ {
        let id = id.into();
        self.successors
            .get(&id)
            .into_iter()
            .flat_map(|v| v.iter().map(|(s, _)| *s))
    }

    /// Get only normal (non-exception) successors of a block
    pub fn normal_successors(&self, id: impl Into<BlockId>) -> impl Iterator<Item = BlockId> + '_ {
        let id = id.into();
        self.successors.get(&id).into_iter().flat_map(|v| {
            v.iter()
                .filter(|(_, kind)| *kind != EdgeKind::Exception)
                .map(|(s, _)| *s)
        })
    }

    pub fn successors_with_kind(&self, id: impl Into<BlockId>) -> &[(BlockId, EdgeKind)] {
        self.successors
            .get(&id.into())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn predecessors(&mut self, id: impl Into<BlockId>) -> Vec<BlockId> {
        self.get_predecessors(id)
    }

    /// Build a deterministic predecessor snapshot for the current graph.
    pub fn predecessor_snapshot(&self) -> BTreeMap<BlockId, Vec<BlockId>> {
        self.predecessor_snapshot_for(|_| true)
    }

    /// Build predecessors for ordinary control flow, excluding exception
    /// dispatch edges. Structured CFG analyses use this view for dominance and
    /// loop discovery; exception ownership is analyzed separately.
    pub fn normal_predecessor_snapshot(&self) -> BTreeMap<BlockId, Vec<BlockId>> {
        self.predecessor_snapshot_for(|kind| kind != EdgeKind::Exception)
    }

    pub(crate) fn has_exception_edges(&self) -> bool {
        self.successors
            .values()
            .flatten()
            .any(|(_, kind)| kind.is_exception())
    }

    pub fn incoming_edges(&self, target: BlockId) -> Vec<(BlockId, EdgeKind)> {
        self.predecessors_with_kind(target).to_vec()
    }

    pub fn predecessors_with_kind(&self, target: BlockId) -> &[(BlockId, EdgeKind)] {
        self.incoming
            .get(&target)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    fn predecessor_groups(&self, target: BlockId) -> impl Iterator<Item = &[(BlockId, EdgeKind)]> {
        self.predecessors_with_kind(target)
            .chunk_by(|left, right| left.0 == right.0)
    }

    pub(crate) fn normal_predecessors(
        &self,
        target: BlockId,
    ) -> impl Iterator<Item = BlockId> + '_ {
        self.predecessor_groups(target).filter_map(|edges| {
            edges
                .iter()
                .any(|(_, kind)| !kind.is_exception())
                .then_some(edges[0].0)
        })
    }

    pub(crate) fn incoming_sources(&self, target: BlockId) -> impl Iterator<Item = BlockId> + '_ {
        self.predecessor_groups(target).map(|edges| edges[0].0)
    }

    fn predecessor_snapshot_for(
        &self,
        include: impl Fn(EdgeKind) -> bool,
    ) -> BTreeMap<BlockId, Vec<BlockId>> {
        let mut snapshot: BTreeMap<_, _> = self.blocks.keys().map(|&id| (id, Vec::new())).collect();
        for (&to, edges) in &self.incoming {
            let mut sources: Vec<_> = edges
                .iter()
                .filter(|(_, kind)| include(*kind))
                .map(|&(from, _)| from)
                .collect();
            sources.dedup();
            if !sources.is_empty() {
                snapshot.insert(to, sources);
            }
        }
        snapshot
    }

    /// Sorted, distinct sources, regardless of how many edge kinds they carry.
    pub fn get_predecessors(&self, id: impl Into<BlockId>) -> Vec<BlockId> {
        self.incoming_sources(id.into()).collect()
    }

    fn forget_incoming(
        index: &mut BTreeMap<BlockId, Vec<(BlockId, EdgeKind)>>,
        to: BlockId,
        edge: (BlockId, EdgeKind),
    ) {
        if let Some(incoming) = index.get_mut(&to) {
            if let Ok(index) = incoming.binary_search(&edge) {
                incoming.remove(index);
            }
            if incoming.is_empty() {
                index.remove(&to);
            }
        }
    }

    pub fn has_edge(&self, from: BlockId, to: BlockId) -> bool {
        self.successors
            .get(&from)
            .map(|v| v.iter().any(|(s, _)| *s == to))
            .unwrap_or(false)
    }

    /// Remove an edge from the graph
    pub fn remove_edge(&mut self, from: BlockId, to: BlockId) {
        if let Some(edges) = self.successors.get_mut(&from) {
            edges.retain(|(target, _)| *target != to);
        }
        if let Some(edges) = self.incoming.get_mut(&to) {
            edges.retain(|(source, _)| *source != from);
            if edges.is_empty() {
                self.incoming.remove(&to);
            }
        }
    }

    /// Remove one typed edge while preserving parallel edges with a different kind.
    pub fn remove_edge_with_kind(&mut self, from: BlockId, to: BlockId, kind: EdgeKind) {
        if let Some(edges) = self.successors.get_mut(&from) {
            edges.retain(|edge| *edge != (to, kind));
        }
        Self::forget_incoming(&mut self.incoming, to, (from, kind));
    }

    /// Get the kind of edge between two blocks, if it exists.
    pub fn get_edge_kind(&self, from: BlockId, to: BlockId) -> Option<EdgeKind> {
        self.successors
            .get(&from)
            .and_then(|succs| succs.iter().find(|(s, _)| *s == to))
            .map(|(_, k)| *k)
    }

    /// Remove all edges from a block
    pub fn remove_all_edges_from(&mut self, from: BlockId) {
        if let Some(edges) = self.successors.get_mut(&from) {
            for (to, kind) in std::mem::take(edges) {
                Self::forget_incoming(&mut self.incoming, to, (from, kind));
            }
        }
    }

    /// Remove ordinary control flow from a block while preserving exception dispatch.
    pub fn remove_normal_edges_from(&mut self, from: BlockId) -> bool {
        let Some(edges) = self.successors.get_mut(&from) else {
            return false;
        };
        let before = edges.len();
        let incoming = &mut self.incoming;
        edges.retain(|&(to, kind)| {
            if kind == EdgeKind::Exception {
                return true;
            }
            Self::forget_incoming(incoming, to, (from, kind));
            false
        });
        edges.len() != before
    }

    /// Remove all exception edges from the graph.
    pub fn remove_exception_edges(&mut self) {
        for edges in self.successors.values_mut() {
            edges.retain(|(_, kind)| *kind != EdgeKind::Exception);
        }
        self.incoming.retain(|_, edges| {
            edges.retain(|(_, kind)| *kind != EdgeKind::Exception);
            !edges.is_empty()
        });
    }

    // === Traversal ===

    pub fn reverse_postorder(&self) -> Vec<BlockId> {
        let mut visited = BTreeSet::new();
        let mut postorder = Vec::new();
        let mut pending = vec![(self.entry, false)];
        while let Some((block, exiting)) = pending.pop() {
            if exiting {
                postorder.push(block);
                continue;
            }
            if !visited.insert(block) {
                continue;
            }
            pending.push((block, true));
            let mut successors = self.successors(block).collect::<Vec<_>>();
            successors.sort_by(|left, right| right.cmp(left));
            pending.extend(successors.into_iter().map(|successor| (successor, false)));
        }
        postorder.reverse();
        postorder
    }

    pub fn reachable(&self) -> BTreeSet<BlockId> {
        let mut visited = BTreeSet::new();
        let mut stack = vec![self.entry];
        while let Some(id) = stack.pop() {
            if visited.insert(id) {
                stack.extend(self.successors(id));
            }
        }
        visited
    }
}

impl fmt::Display for CFG {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "CFG: {} (entry: {})", self.label, self.entry)?;
        for block in self.blocks_iter() {
            writeln!(f, "\n{}", block)?;
            for insn in &block.insns {
                writeln!(f, "  {}", insn)?;
            }
            let succs: Vec<_> = self.successors(block.id).collect();
            if !succs.is_empty() {
                writeln!(f, "  -> {:?}", succs)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfg_basic() {
        let mut cfg = CFG::new("test");
        cfg.add_block(Block::new(0u32));
        cfg.add_block(Block::new(1u32));
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);

        assert_eq!(cfg.num_blocks(), 2);
        let succs: Vec<_> = cfg.successors(0u32).collect();
        assert_eq!(succs, vec![BlockId::new(1)]);
    }

    #[test]
    fn test_prune_unreachable_graph_nodes_preserves_blocks() {
        let mut cfg = CFG::new("test");
        cfg.add_block(Block::new(0u32));
        cfg.add_block(Block::new(1u32));
        cfg.add_block(Block::new(2u32));
        cfg.entry = BlockId::new(0);
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(2), BlockId::new(2), EdgeKind::Normal);

        let removed = cfg.prune_unreachable_graph_nodes();

        assert_eq!(removed, vec![BlockId::new(2)]);
        assert!(cfg.block(BlockId::new(2)).is_some());
        assert!(!cfg.is_graph_node(BlockId::new(2)));
        assert_eq!(cfg.graph_node_ids(), vec![BlockId::new(0), BlockId::new(1)]);
    }
}
