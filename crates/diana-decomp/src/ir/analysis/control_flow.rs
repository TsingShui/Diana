//! Immutable ordinary-control-flow facts shared by region analyses.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::ir::{BlockId, InsnType, CFG};

use super::{DominanceError, DominatorTree, PostDominatorTree};

#[derive(Debug, Clone)]
pub struct ControlFlowFacts {
    predecessors: BTreeMap<BlockId, Vec<BlockId>>,
    semantic_predecessors: Option<BTreeMap<BlockId, Vec<BlockId>>>,
    dominators: Arc<DominatorTree>,
    semantic_dominators: DominatorTree,
    postdominators: PostDominatorTree,
    phi_continuations: Option<ControlContinuations>,
    structural_continuations: ControlContinuations,
}

impl ControlFlowFacts {
    pub fn analyze(cfg: &CFG) -> Result<Self, DominanceError> {
        Self::analyze_with_dominators(cfg, None)
    }

    pub(crate) fn analyze_with_dominators(
        cfg: &CFG,
        semantic_dominators: Option<DominatorTree>,
    ) -> Result<Self, DominanceError> {
        let predecessors = cfg.normal_predecessor_snapshot();
        let semantic_predecessors = cfg
            .has_exception_edges()
            .then(|| cfg.predecessor_snapshot());
        let dominators = match (&semantic_dominators, &semantic_predecessors) {
            (Some(complete), None) => complete.with_virtual_root()?,
            _ => DominatorTree::compute_normal(cfg, cfg.block_ids(), &predecessors)?,
        };
        let semantic_dominators = match semantic_dominators {
            Some(tree) => tree,
            None => DominatorTree::compute_with_predecessors(
                cfg,
                cfg.entry,
                cfg.block_ids(),
                semantic_predecessors.as_ref().unwrap_or(&predecessors),
            )?,
        };
        let postdominators = PostDominatorTree::compute(cfg)?;
        let boundaries = ControlContinuations::phi_boundaries(cfg);
        let phi_continuations =
            (!boundaries.is_empty()).then(|| ControlContinuations::preserving(cfg, &boundaries));
        let structural_continuations = ControlContinuations::preserving(cfg, &BTreeSet::new());
        Ok(Self {
            predecessors,
            semantic_predecessors,
            dominators: Arc::new(dominators),
            semantic_dominators,
            postdominators,
            phi_continuations,
            structural_continuations,
        })
    }

    pub fn predecessors(&self, block: BlockId) -> Option<&[BlockId]> {
        self.predecessors.get(&block).map(Vec::as_slice)
    }

    pub(crate) fn normal_dominance(
        &self,
    ) -> (&BTreeMap<BlockId, Vec<BlockId>>, Arc<DominatorTree>) {
        (&self.predecessors, Arc::clone(&self.dominators))
    }

    pub fn semantic_predecessors(&self, block: BlockId) -> Option<&[BlockId]> {
        self.semantic_predecessors
            .as_ref()
            .unwrap_or(&self.predecessors)
            .get(&block)
            .map(Vec::as_slice)
    }

    pub fn dominators(&self) -> &DominatorTree {
        &self.dominators
    }

    pub fn semantic_dominators(&self) -> &DominatorTree {
        &self.semantic_dominators
    }

    pub fn postdominators(&self) -> &PostDominatorTree {
        &self.postdominators
    }

    pub fn continuation(&self, target: BlockId) -> BlockId {
        self.phi_continuations
            .as_ref()
            .unwrap_or(&self.structural_continuations)
            .destination(target)
    }

    /// Returns the semantic destination used by structural control analysis.
    ///
    /// Unlike [`Self::continuation`], this view contracts transparent synthetic
    /// edge blocks even when they anchor Phi inputs. The physical blocks remain
    /// in the CFG so edge-specific copies retain their identity.
    pub fn structural_continuation(&self, target: BlockId) -> BlockId {
        self.structural_continuations.destination(target)
    }
}

#[derive(Debug, Clone)]
pub struct ControlContinuations {
    destinations: BTreeMap<BlockId, BlockId>,
}

impl ControlContinuations {
    pub fn analyze(cfg: &CFG) -> Self {
        Self::preserving(cfg, &Self::phi_boundaries(cfg))
    }

    fn phi_boundaries(cfg: &CFG) -> BTreeSet<BlockId> {
        cfg.blocks
            .values()
            .flat_map(|block| &block.insns)
            .filter(|instruction| instruction.insn_type == InsnType::Phi)
            .flat_map(|instruction| {
                instruction
                    .payload
                    .phi_edges
                    .iter()
                    .map(|(predecessor, _)| *predecessor)
            })
            .collect()
    }

    pub fn preserving(cfg: &CFG, boundaries: &BTreeSet<BlockId>) -> Self {
        let mut next: BTreeMap<_, _> = cfg
            .blocks
            .iter()
            .filter_map(|(&id, block)| {
                if !block.synthetic || boundaries.contains(&id) {
                    return None;
                }
                // Exactly one physical successor, of any ordinary edge kind.
                let &[(target, kind)] = cfg.successors_with_kind(id) else {
                    return None;
                };
                if kind.is_exception()
                    || cfg
                        .predecessors_with_kind(id)
                        .iter()
                        .any(|(_, kind)| kind.is_exception())
                    || !block
                        .insns
                        .iter()
                        .all(|insn| matches!(insn.insn_type, InsnType::Nop | InsnType::Goto))
                {
                    return None;
                }
                Some((id, target))
            })
            .collect();

        let mut destinations = BTreeMap::new();
        let mut path = Vec::new();
        while let Some((start, mut current)) = next.pop_first() {
            path.clear();
            path.push(start);
            while let Some(target) = next.remove(&current) {
                path.push(current);
                current = target;
            }
            // Every member of a cycle returns to itself. An incoming path
            // stops at the first cycle member it encounters, including when
            // that member was already resolved on an earlier walk.
            let end = path
                .iter()
                .position(|&block| block == current)
                .unwrap_or(path.len());
            let destination = destinations.get(&current).copied().unwrap_or(current);
            destinations.extend(path[..end].iter().map(|&block| (block, destination)));
        }
        Self { destinations }
    }

    pub fn destination(&self, target: BlockId) -> BlockId {
        self.destinations.get(&target).copied().unwrap_or(target)
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/control_continuations.rs"]
mod continuation_tests;
