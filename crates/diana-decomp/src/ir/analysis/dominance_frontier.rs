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
        for &source in cfg.blocks.keys() {
            for target in cfg.successors(source) {
                let stop = dominators.idom(target);
                let mut cursor = Some(source);
                // This edge contributes its target to the source's frontier
                // and each ancestor below the target's immediate dominator.
                // A previously inserted target already covered that suffix.
                while let Some(block) = cursor {
                    if Some(block) == stop || !frontiers.entry(block).or_default().insert(target) {
                        break;
                    }
                    cursor = dominators.idom(block);
                }
            }
        }

        // Explicit callers can supply a tree containing additional vertices.
        // Keep the original child-before-parent diagnostics in tree order.
        if dominators.len() != cfg.blocks.len() {
            for block in dominators.postorder() {
                for child in dominators.children(block) {
                    if !cfg.blocks.contains_key(&child) {
                        return Err(DominanceError::MissingTopology(child));
                    }
                    if !cfg.blocks.contains_key(&block)
                        && frontiers.get(&child).is_some_and(|targets| {
                            targets
                                .iter()
                                .any(|&target| dominators.idom(target) != Some(block))
                        })
                    {
                        return Err(DominanceError::MissingTopology(block));
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
