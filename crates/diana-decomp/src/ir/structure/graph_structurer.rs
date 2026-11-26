//! Region-local semantic graph structuring.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::analysis::{ReachingConditionError, StrongComponents};
use crate::ir::semantic::SemanticFactory;
use crate::ir::{BlockId, RegionId, SemanticNode, CFG};

use super::{
    acyclic::{AcyclicStructure, AcyclicStructureError, AcyclicStructurer},
    flow_graph::SemanticFlowGraph,
    StructureError,
};

pub(super) struct GraphStructurer<'a, 'semantic> {
    cfg: &'a CFG,
    semantic: &'semantic SemanticFactory<'semantic>,
    region: RegionId,
    seeded: BTreeMap<BlockId, SemanticNode>,
    terminal_seeds: BTreeSet<BlockId>,
    force_graph_reduction: bool,
}

impl<'a, 'semantic> GraphStructurer<'a, 'semantic> {
    pub(super) fn new(
        cfg: &'a CFG,
        semantic: &'semantic SemanticFactory<'semantic>,
        region: RegionId,
    ) -> Self {
        Self {
            cfg,
            semantic,
            region,
            seeded: BTreeMap::new(),
            terminal_seeds: BTreeSet::new(),
            force_graph_reduction: false,
        }
    }

    pub(super) fn seeded(mut self, nodes: BTreeMap<BlockId, SemanticNode>) -> Self {
        self.seeded = nodes;
        self
    }

    pub(super) fn terminal_seeds(mut self, blocks: BTreeSet<BlockId>) -> Self {
        self.terminal_seeds = blocks;
        self
    }

    pub(super) fn force_graph_reduction(mut self, force: bool) -> Self {
        self.force_graph_reduction = force;
        self
    }

    pub(super) fn structure(mut self) -> Result<SemanticNode, StructureError> {
        if self.cfg.num_graph_nodes() == 0 {
            return Ok(SemanticNode::Empty);
        }
        if self.force_graph_reduction || !self.terminal_seeds.is_empty() {
            return self.reduce_graph();
        }
        let mut structurer = AcyclicStructurer::new(self.cfg, self.semantic, &mut self.seeded);
        if let Some(node) = structurer
            .linear()
            .map_err(|source| StructureError::Acyclic {
                region: self.region,
                source,
            })?
        {
            return Ok(node);
        }
        match structurer.structure_branches() {
            Ok(AcyclicStructure::Structured(node)) => Ok(node),
            Ok(AcyclicStructure::RequiresGraphReduction(_))
            | Err(AcyclicStructureError::ReachingCondition(ReachingConditionError::CyclicGraph)) => {
                self.reduce_graph()
            }
            Err(source) => {
                // Kahn's traversal establishes acyclicity before any BDD or
                // lowering work. A root/degree error can preempt that traversal
                // even in a cyclic graph; retain cycle reduction in this case.
                if matches!(
                    source,
                    AcyclicStructureError::ReachingCondition(
                        ReachingConditionError::MissingEntry(_)
                            | ReachingConditionError::MultipleEntries { .. }
                            | ReachingConditionError::MalformedTopology(_)
                    )
                ) && !StrongComponents::analyze(self.cfg, self.cfg.graph_node_ids()).is_acyclic()
                {
                    return self.reduce_graph();
                }
                Err(StructureError::Acyclic {
                    region: self.region,
                    source,
                })
            }
        }
    }

    fn reduce_graph(self) -> Result<SemanticNode, StructureError> {
        SemanticFlowGraph::structure(
            self.cfg,
            self.semantic,
            self.region,
            self.seeded,
            &self.terminal_seeds,
        )
    }
}
