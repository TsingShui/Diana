//! Dominance-frontier computation over the canonical dominator tree.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{BlockId, CFG};

use super::{DominanceError, DominatorTree};

#[derive(Debug, Clone)]
pub struct DominanceFrontier {
    frontiers: BTreeMap<BlockId, BTreeSet<BlockId>>,
}

impl DominanceFrontier {
    pub fn compute(cfg: &CFG, dominators: &DominatorTree) -> Result<Self, DominanceError> {
        for &block in cfg.blocks.keys() {
            if !dominators.contains(block) {
                return Err(DominanceError::MissingTopology(block));
            }
        }
        let mut frontiers = BTreeMap::<BlockId, BTreeSet<BlockId>>::new();
        for &block in cfg.blocks.keys() {
            for successor in cfg.successors(block) {
                if dominators.idom(successor) != Some(block) {
                    frontiers.entry(block).or_default().insert(successor);
                }
            }
        }

        // An empty local frontier stays empty throughout the upward pass.
        // Explicit caller-provided trees can include additional vertices;
        // retain the topology checks for that case.
        if !frontiers.is_empty() || dominators.len() != cfg.blocks.len() {
            let mut inherited = Vec::new();
            for block in dominators.postorder() {
                for child in dominators.children(block) {
                    if !cfg.blocks.contains_key(&child) {
                        return Err(DominanceError::MissingTopology(child));
                    }
                    if let Some(boundaries) = frontiers.get(&child) {
                        inherited.extend(
                            boundaries
                                .iter()
                                .copied()
                                .filter(|&boundary| dominators.idom(boundary) != Some(block)),
                        );
                    }
                    if !inherited.is_empty() {
                        if !cfg.blocks.contains_key(&block) {
                            return Err(DominanceError::MissingTopology(block));
                        }
                        frontiers
                            .entry(block)
                            .or_default()
                            .extend(inherited.drain(..));
                    }
                }
            }
        }
        Ok(Self { frontiers })
    }

    pub fn frontier(&self, block: BlockId) -> impl Iterator<Item = BlockId> + '_ {
        self.frontiers
            .get(&block)
            .into_iter()
            .flat_map(|frontier| frontier.iter().copied())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.frontiers.is_empty()
    }

    pub fn in_frontier(&self, block: BlockId, of: BlockId) -> bool {
        self.frontiers
            .get(&of)
            .is_some_and(|frontier| frontier.contains(&block))
    }
}
