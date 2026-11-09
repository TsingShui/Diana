//! Exact original transfers at a P6 candidate's private interface. This is
//! nomination/admission input, not a second graph or an output verifier.

use super::build::*;
use super::graph::Graph;
use super::*;
use crate::p2::Terminal;
use crate::p5::StorageMethod;

pub(super) struct CompletionInterface {
    pub nodes: Vec<NodeId>,
    pub entrances: Vec<LinkId>,
    pub exceptional_entrances: Vec<crate::p2::EdgeId>,
    pub outlets: Vec<LinkId>,
    /// Includes original uncaught Exit targets. The EdgeId retains site,
    /// priority, completion, before-throw snapshots and handler receive facts.
    pub exceptional_outlets: Vec<crate::p2::EdgeId>,
    pub abrupt: Vec<NodeId>,
}
impl CompletionInterface {
    pub fn collect(
        storage: &StorageMethod<'_>,
        f: &RegionFacts,
        g: &Graph,
        nodes: &[NodeId],
    ) -> Result<Self, RegionError> {
        let mut set = filled(f.nodes.len(), false)?;
        for node in nodes {
            set[node.0 as usize] = true;
        }
        let mut coverage = Vec::new();
        crate::substrate::owned::reserve(&mut coverage, nodes.len())?;
        coverage.extend_from_slice(nodes);
        let mut result = Self {
            nodes: coverage,
            entrances: Vec::new(),
            exceptional_entrances: Vec::new(),
            outlets: Vec::new(),
            exceptional_outlets: Vec::new(),
            abrupt: Vec::new(),
        };
        for link in &f.links {
            match (set[link.from.0 as usize], set[link.to.0 as usize]) {
                (false, true) => push(&mut result.entrances, link.id)?,
                (true, false) => push(&mut result.outlets, link.id)?,
                _ => {}
            }
        }
        for (from, edges) in g.exceptions.iter().enumerate() {
            for &edge in edges {
                if !Graph::exception_possible(f, edge) {
                    continue;
                }
                let internal =
                    Graph::exception_target(f, storage, edge).is_some_and(|to| set[to.0 as usize]);
                match (set[from], internal) {
                    (false, true) => push(&mut result.exceptional_entrances, edge)?,
                    (true, false) => push(&mut result.exceptional_outlets, edge)?,
                    _ => {}
                }
            }
        }
        for &node in nodes {
            let source = &f.nodes[node.0 as usize];
            if source.terminal
                && matches!(
                    storage.execution().blocks()[source.block.0 as usize].terminal(),
                    Some(Terminal::Return { .. } | Terminal::Throw { .. })
                )
            {
                push(&mut result.abrupt, node)?;
            }
        }
        Ok(result)
    }

    /// Only normal outlet targets constrain a normal child walk. Exceptional
    /// coverage does not create a normal follow or a runtime transfer.
    pub fn normal_boundary(&self, f: &RegionFacts) -> Result<Vec<bool>, RegionError> {
        let mut boundary = filled(f.nodes.len(), false)?;
        for &link in &self.outlets {
            boundary[f.links[link.0 as usize].to.0 as usize] = true;
        }
        Ok(boundary)
    }
}
