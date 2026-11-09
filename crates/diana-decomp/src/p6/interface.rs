//! Exact original transfers at a P6 candidate's private interface. This is
//! nomination/admission input, not a second graph or an output verifier.

use super::build::*;
use super::graph::Graph;
use super::*;
use crate::p2::Terminal;
use crate::p5::StorageMethod;

pub(super) struct CompletionInterface {
    pub method_entry: bool,
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
        Self::from_membership(storage, f, g, &set)
    }

    /// Reuses the owning candidate's membership instead of allocating another
    /// mask and copying the same original-node list for each boundary consumer.
    pub fn from_membership(
        storage: &StorageMethod<'_>,
        f: &RegionFacts,
        g: &Graph,
        set: &[bool],
    ) -> Result<Self, RegionError> {
        if set.len() != f.nodes.len() {
            return Err(bug("completion membership has the wrong extent"));
        }
        Self::matching(storage, f, g, |node| set[node.0 as usize])
    }

    pub fn for_region(
        storage: &StorageMethod<'_>,
        f: &RegionFacts,
        g: &Graph,
        region: RegionId,
    ) -> Result<Self, RegionError> {
        Self::matching(storage, f, g, f.membership(region)?)
    }

    fn matching(
        storage: &StorageMethod<'_>,
        f: &RegionFacts,
        g: &Graph,
        inside: impl Fn(NodeId) -> bool,
    ) -> Result<Self, RegionError> {
        let mut result = Self {
            method_entry: !f.nodes.is_empty() && inside(NodeId(0)),
            entrances: Vec::new(),
            exceptional_entrances: Vec::new(),
            outlets: Vec::new(),
            exceptional_outlets: Vec::new(),
            abrupt: Vec::new(),
        };
        for link in &f.links {
            match (inside(link.from), inside(link.to)) {
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
                let internal = Graph::exception_target(f, storage, edge).is_some_and(&inside);
                match (inside(f.nodes[from].id), internal) {
                    (false, true) => push(&mut result.exceptional_entrances, edge)?,
                    (true, false) => push(&mut result.exceptional_outlets, edge)?,
                    _ => {}
                }
            }
        }
        for source in &f.nodes {
            if inside(source.id)
                && source.terminal
                && matches!(
                    storage.execution().blocks()[source.block.0 as usize].terminal(),
                    Some(Terminal::Return { .. } | Terminal::Throw { .. })
                )
            {
                push(&mut result.abrupt, source.id)?;
            }
        }
        Ok(result)
    }

    pub fn entry_nodes(
        &self,
        storage: &StorageMethod<'_>,
        f: &RegionFacts,
        entry: Option<NodeId>,
    ) -> Result<Vec<NodeId>, RegionError> {
        let mut entries = Vec::new();
        if let Some(entry) = entry {
            push(&mut entries, entry)?;
        }
        if self.method_entry && !entries.contains(&NodeId(0)) {
            push(&mut entries, NodeId(0))?;
        }
        for &link in &self.entrances {
            let target = f.links[link.0 as usize].to;
            if !entries.contains(&target) {
                push(&mut entries, target)?;
            }
        }
        for &edge in &self.exceptional_entrances {
            if let Some(target) = Graph::exception_target(f, storage, edge)
                && !entries.contains(&target)
            {
                push(&mut entries, target)?;
            }
        }
        Ok(entries)
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
