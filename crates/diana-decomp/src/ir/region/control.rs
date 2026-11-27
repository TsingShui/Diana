//! Loop and switch regions derived from graph invariants.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::analysis::ControlFlowFacts;
use crate::ir::{BlockId, InsnType, CFG};

use super::{LoopRegion, RegionId, RegionInvariantError, RegionKind, RegionTree, SwitchRegion};

#[derive(Clone, Debug)]
struct ControlCandidate {
    kind: RegionKind,
    entry: BlockId,
    core: BTreeSet<BlockId>,
    blocks: BTreeSet<BlockId>,
}

impl ControlCandidate {
    fn insert_maximal_laminar(
        self,
        cfg: &CFG,
        tree: &mut RegionTree,
        handlers: &BTreeMap<RegionId, Vec<RegionId>>,
        facts: &ControlFlowFacts,
    ) -> Result<(), RegionInvariantError> {
        let Self {
            mut kind,
            entry,
            mut core,
            mut blocks,
        } = self;
        let owner = Self::lexical_owner(tree, entry, &core)?;
        let domain = tree
            .region(owner)
            .ok_or(RegionInvariantError::UnknownRegion(owner))?
            .blocks
            .clone();
        core.retain(|block| domain.contains(block));
        blocks.retain(|block| domain.contains(block));
        let closure = ControlRegionClosure::new(cfg, facts, tree, handlers, entry, &domain);
        let blocks = closure.close(&blocks, matches!(&kind, RegionKind::Loop(_)))?;
        Self::refresh_loop_follow(cfg, facts, &mut kind, &blocks);
        // Domain clipping can drop the header when it sits outside the lexical
        // owner of the body (e.g. loop header above a nested try). Inserting
        // that truncated region would leave `entry` outside `blocks`.
        if !blocks.contains(&entry) {
            return Ok(());
        }
        if Self::already_inserted(tree, &kind, entry, &blocks) {
            return Ok(());
        }
        let foreign_entries = Self::foreign_entries(facts, entry, &blocks);
        let placement = if foreign_entries.is_empty() {
            tree.insert_laminar_region(kind.clone(), entry, blocks.clone())?
        } else {
            super::RegionPlacement::Residual
        };
        match placement {
            super::RegionPlacement::Inserted(_) => Ok(()),
            super::RegionPlacement::Residual if core != blocks => {
                let closure = ControlRegionClosure::new(cfg, facts, tree, handlers, entry, &domain);
                let core = closure.close(&core, matches!(&kind, RegionKind::Loop(_)))?;
                Self::refresh_loop_follow(cfg, facts, &mut kind, &core);
                if !core.contains(&entry) {
                    return Ok(());
                }
                let placement = if Self::foreign_entries(facts, entry, &core).is_empty() {
                    tree.insert_laminar_region(kind, entry, core)?
                } else {
                    super::RegionPlacement::Residual
                };
                match placement {
                    super::RegionPlacement::Inserted(_) => Ok(()),
                    super::RegionPlacement::Residual => Ok(()),
                }
            }
            super::RegionPlacement::Residual => Ok(()),
        }
    }

    fn lexical_owner(
        tree: &RegionTree,
        entry: BlockId,
        core: &BTreeSet<BlockId>,
    ) -> Result<RegionId, RegionInvariantError> {
        core.iter().try_fold(tree.owner(entry)?, |owner, block| {
            tree.common_ancestor(owner, tree.owner(*block)?)
        })
    }

    fn refresh_loop_follow(
        cfg: &CFG,
        facts: &ControlFlowFacts,
        kind: &mut RegionKind,
        blocks: &BTreeSet<BlockId>,
    ) {
        let RegionKind::Loop(region) = kind else {
            return;
        };
        let successors = blocks
            .iter()
            .copied()
            .flat_map(|source| cfg.normal_successors(source))
            .filter(|target| !blocks.contains(target))
            .collect::<BTreeSet<_>>();
        region.follow = LoopFollow::new(cfg, facts, blocks, &successors, &region.latches).analyze();
    }

    fn already_inserted(
        tree: &RegionTree,
        kind: &RegionKind,
        entry: BlockId,
        blocks: &BTreeSet<BlockId>,
    ) -> bool {
        tree.regions().any(|region| {
            region.entry == Some(entry)
                && blocks.is_subset(&region.blocks)
                && matches!(
                    (&region.kind, kind),
                    (RegionKind::Loop(_), RegionKind::Loop(_))
                        | (RegionKind::Switch(_), RegionKind::Switch(_))
                )
        })
    }

    fn foreign_entries(
        facts: &ControlFlowFacts,
        entry: BlockId,
        blocks: &BTreeSet<BlockId>,
    ) -> Vec<(BlockId, BlockId)> {
        blocks
            .iter()
            .copied()
            .filter(|block| *block != entry)
            .flat_map(|block| {
                facts
                    .predecessors(block)
                    .into_iter()
                    .flatten()
                    .copied()
                    .filter(move |predecessor| !blocks.contains(predecessor))
                    .map(move |predecessor| (predecessor, block))
            })
            .collect()
    }
}

/// Closes control regions over lexical children and semantically recurrent
/// paths. Ordinary backedge discovery cannot see a continuation reached only
/// through exception handlers, even when that continuation returns to the
/// natural-loop header.
struct ControlRegionClosure<'a> {
    cfg: &'a CFG,
    facts: &'a ControlFlowFacts,
    tree: &'a RegionTree,
    handlers: &'a BTreeMap<RegionId, Vec<RegionId>>,
    handler_owners: BTreeMap<RegionId, BTreeSet<RegionId>>,
    entry: BlockId,
    domain: &'a BTreeSet<BlockId>,
}

impl<'a> ControlRegionClosure<'a> {
    fn new(
        cfg: &'a CFG,
        facts: &'a ControlFlowFacts,
        tree: &'a RegionTree,
        handlers: &'a BTreeMap<RegionId, Vec<RegionId>>,
        entry: BlockId,
        domain: &'a BTreeSet<BlockId>,
    ) -> Self {
        let mut handler_owners = BTreeMap::<RegionId, BTreeSet<RegionId>>::new();
        for (owner, attached) in handlers {
            for handler in attached {
                handler_owners.entry(*handler).or_default().insert(*owner);
            }
        }
        Self {
            cfg,
            facts,
            tree,
            handlers,
            handler_owners,
            entry,
            domain,
        }
    }

    fn close(
        &self,
        seed: &BTreeSet<BlockId>,
        recurrent: bool,
    ) -> Result<BTreeSet<BlockId>, RegionInvariantError> {
        let mut blocks = seed.clone();
        loop {
            let mut closed = self.tree.laminar_closure(&blocks);
            let handlers = self.handler_blocks(&closed)?;
            closed.extend(handlers.iter().copied());
            let recurrent_blocks = if recurrent {
                self.recurrent_paths(&closed)
            } else {
                BTreeSet::new()
            };
            if recurrent {
                closed.extend(recurrent_blocks.iter().copied());
            }
            if closed == blocks {
                return Ok(blocks);
            }
            blocks = closed;
        }
    }

    fn handler_blocks(
        &self,
        blocks: &BTreeSet<BlockId>,
    ) -> Result<BTreeSet<BlockId>, RegionInvariantError> {
        let mut owned = BTreeSet::new();
        let enclosed = self
            .handlers
            .keys()
            .copied()
            .filter(|owner| {
                self.tree
                    .region(*owner)
                    .is_some_and(|region| region.blocks.is_subset(blocks))
            })
            .collect::<BTreeSet<_>>();
        let candidates = enclosed
            .iter()
            .flat_map(|owner| self.handlers.get(owner).into_iter().flatten().copied())
            .collect::<BTreeSet<_>>();
        for handler in candidates {
            let Some(owners) = self.handler_owners.get(&handler) else {
                continue;
            };
            if !owners.is_subset(&enclosed) {
                continue;
            }
            let handler_region = self
                .tree
                .region(handler)
                .ok_or(RegionInvariantError::UnknownRegion(handler))?;
            if handler_region.blocks.is_subset(self.domain) {
                owned.extend(handler_region.blocks.iter().copied());
            }
        }
        Ok(owned)
    }

    fn recurrent_paths(&self, blocks: &BTreeSet<BlockId>) -> BTreeSet<BlockId> {
        let returning = self.reverse_reachable_to_entry();
        let mut candidates = BTreeSet::new();
        let mut pending = blocks
            .iter()
            .copied()
            .flat_map(|block| self.cfg.normal_successors(block))
            .collect::<Vec<_>>();
        while let Some(block) = pending.pop() {
            if !self.domain.contains(&block)
                || blocks.contains(&block)
                || !returning.contains(&block)
                || !candidates.insert(block)
            {
                continue;
            }
            pending.extend(self.cfg.normal_successors(block));
        }

        loop {
            let scope = blocks
                .iter()
                .chain(candidates.iter())
                .copied()
                .collect::<BTreeSet<_>>();
            let foreign = candidates
                .iter()
                .copied()
                .filter(|block| {
                    self.facts
                        .predecessors(*block)
                        .into_iter()
                        .flatten()
                        .any(|predecessor| !scope.contains(predecessor))
                })
                .collect::<Vec<_>>();
            if foreign.is_empty() {
                return candidates;
            }
            for block in foreign {
                candidates.remove(&block);
            }
        }
    }

    fn reverse_reachable_to_entry(&self) -> BTreeSet<BlockId> {
        let mut reachable = BTreeSet::from([self.entry]);
        let mut pending = self
            .facts
            .predecessors(self.entry)
            .into_iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        while let Some(block) = pending.pop() {
            if !self.domain.contains(&block)
                || !self
                    .facts
                    .semantic_dominators()
                    .dominates(self.entry, block)
                || !reachable.insert(block)
            {
                continue;
            }
            pending.extend(
                self.facts
                    .predecessors(block)
                    .into_iter()
                    .flatten()
                    .copied(),
            );
        }
        reachable
    }
}

pub(super) struct ControlRegionAnalysis<'a> {
    cfg: &'a CFG,
    facts: &'a ControlFlowFacts,
}

pub(super) struct PreparedControlRegions {
    loops: Vec<ControlCandidate>,
    switches: Vec<ControlCandidate>,
    exceptional_latches: BTreeMap<BlockId, BTreeSet<BlockId>>,
}

impl<'a> ControlRegionAnalysis<'a> {
    pub(super) fn new(cfg: &'a CFG, facts: &'a ControlFlowFacts) -> Self {
        Self { cfg, facts }
    }

    pub(super) fn apply(
        &self,
        tree: &mut RegionTree,
    ) -> Result<PreparedControlRegions, RegionInvariantError> {
        let mut prepared = self.normal_loops()?;
        Self::insert_candidates(
            self.cfg,
            tree,
            prepared.loops.clone(),
            &BTreeMap::new(),
            self.facts,
        )?;
        prepared.switches = self.switch_candidates()?;
        Self::insert_candidates(
            self.cfg,
            tree,
            prepared.switches.clone(),
            &BTreeMap::new(),
            self.facts,
        )?;
        Ok(prepared)
    }

    pub(super) fn apply_with_handlers(
        &self,
        tree: &mut RegionTree,
        handlers: &BTreeMap<RegionId, Vec<RegionId>>,
        mut prepared: PreparedControlRegions,
    ) -> Result<(), RegionInvariantError> {
        // Candidates depend on the same immutable CFG in both passes. An
        // exception-aware search is needed only for a header whose latches
        // could not be reached through any normal path.
        for (entry, mut latches) in prepared.exceptional_latches {
            let reachable = self.reachable(entry, LoopConnectivity::ExceptionAware);
            latches.retain(|latch| reachable.contains(latch));
            if !latches.is_empty() {
                prepared.loops.push(self.loop_candidate(
                    entry,
                    latches,
                    &reachable,
                    LoopConnectivity::ExceptionAware,
                )?);
            }
        }
        Self::insert_candidates(self.cfg, tree, prepared.loops, handlers, self.facts)?;
        Self::insert_candidates(
            self.cfg,
            tree,
            prepared.switches,
            &BTreeMap::new(),
            self.facts,
        )
    }

    fn insert_candidates(
        cfg: &CFG,
        tree: &mut RegionTree,
        mut candidates: Vec<ControlCandidate>,
        handlers: &BTreeMap<RegionId, Vec<RegionId>>,
        facts: &ControlFlowFacts,
    ) -> Result<(), RegionInvariantError> {
        candidates
            .sort_by_key(|candidate| (std::cmp::Reverse(candidate.blocks.len()), candidate.entry));
        for candidate in candidates {
            candidate.insert_maximal_laminar(cfg, tree, handlers, facts)?;
        }
        Ok(())
    }

    fn normal_loops(&self) -> Result<PreparedControlRegions, RegionInvariantError> {
        let mut back_edges = BTreeMap::<BlockId, BTreeSet<BlockId>>::new();
        for source in self.cfg.blocks.keys().copied() {
            for target in self.cfg.normal_successors(source) {
                if self.cfg.block(target).is_some()
                    && self.facts.semantic_dominators().dominates(target, source)
                {
                    back_edges.entry(target).or_default().insert(source);
                }
            }
        }
        let mut loops = Vec::new();
        let mut exceptional_latches = BTreeMap::new();
        for (entry, latches) in back_edges {
            let reachable = self.reachable(entry, LoopConnectivity::Normal);
            let normal_latches = latches
                .intersection(&reachable)
                .copied()
                .collect::<BTreeSet<_>>();
            if normal_latches.is_empty() {
                exceptional_latches.insert(entry, latches);
            } else {
                loops.push(self.loop_candidate(
                    entry,
                    normal_latches,
                    &reachable,
                    LoopConnectivity::Normal,
                )?);
            }
        }
        Ok(PreparedControlRegions {
            loops,
            exceptional_latches,
            switches: Vec::new(),
        })
    }

    fn loop_candidate(
        &self,
        entry: BlockId,
        latches: BTreeSet<BlockId>,
        reachable: &BTreeSet<BlockId>,
        connectivity: LoopConnectivity,
    ) -> Result<ControlCandidate, RegionInvariantError> {
        let core = self.natural_loop_nodes(reachable, connectivity, entry, &latches)?;
        let successors = self.loop_successors(&core);
        let follow = LoopFollow::new(self.cfg, self.facts, &core, &successors, &latches).analyze();
        let blocks = LoopBody::new(self.cfg, self.facts, entry, follow).analyze(&core);
        let mut kind = RegionKind::Loop(LoopRegion { follow, latches });
        ControlCandidate::refresh_loop_follow(self.cfg, self.facts, &mut kind, &blocks);
        Ok(ControlCandidate {
            kind,
            entry,
            core,
            blocks,
        })
    }

    fn natural_loop_nodes(
        &self,
        reachable: &BTreeSet<BlockId>,
        connectivity: LoopConnectivity,
        header: BlockId,
        latches: &BTreeSet<BlockId>,
    ) -> Result<BTreeSet<BlockId>, RegionInvariantError> {
        let mut blocks = BTreeSet::from([header]);
        let mut pending = latches.iter().copied().collect::<Vec<_>>();
        while let Some(block) = pending.pop() {
            if !reachable.contains(&block)
                || !self.facts.semantic_dominators().dominates(header, block)
                || !blocks.insert(block)
            {
                continue;
            }
            let predecessors = connectivity
                .predecessors(self.facts, block)
                .ok_or(RegionInvariantError::MissingBlock(block))?;
            pending.extend(predecessors.iter().copied());
        }
        Ok(blocks)
    }

    fn reachable(&self, entry: BlockId, connectivity: LoopConnectivity) -> BTreeSet<BlockId> {
        let mut reachable = BTreeSet::new();
        let mut pending = vec![entry];
        while let Some(block) = pending.pop() {
            if self.cfg.block(block).is_none() || !reachable.insert(block) {
                continue;
            }
            match connectivity {
                LoopConnectivity::Normal => pending.extend(self.cfg.normal_successors(block)),
                LoopConnectivity::ExceptionAware => pending.extend(self.cfg.successors(block)),
            }
        }
        reachable
    }

    fn loop_successors(&self, loop_nodes: &BTreeSet<BlockId>) -> BTreeSet<BlockId> {
        loop_nodes
            .iter()
            .copied()
            .flat_map(|source| self.cfg.normal_successors(source))
            .filter(|target| !loop_nodes.contains(target))
            .collect()
    }

    fn switch_candidates(&self) -> Result<Vec<ControlCandidate>, RegionInvariantError> {
        let mut candidates = Vec::new();
        for (&header, block) in &self.cfg.blocks {
            if !block
                .terminator()
                .is_some_and(|terminator| terminator.insn_type == InsnType::Switch)
            {
                continue;
            }
            let successors = self.cfg.normal_successors(header).collect::<Vec<_>>();
            if successors.is_empty() {
                return Err(RegionInvariantError::MalformedSwitch {
                    block: header,
                    successors,
                });
            }
            let follow = SwitchFollow::new(self.facts, header, &successors).analyze();
            let mut blocks = BTreeSet::new();
            let mut pending = vec![header];
            while let Some(node) = pending.pop() {
                if Some(node) == follow
                    || !self.facts.dominators().dominates(header, node)
                    || !blocks.insert(node)
                {
                    continue;
                }
                pending.extend(self.cfg.normal_successors(node));
            }
            if blocks.len() > 1 {
                candidates.push(ControlCandidate {
                    kind: RegionKind::Switch(SwitchRegion { follow }),
                    entry: header,
                    core: blocks.clone(),
                    blocks,
                });
            }
        }
        Ok(candidates)
    }
}

#[derive(Clone, Copy)]
enum LoopConnectivity {
    Normal,
    ExceptionAware,
}

impl LoopConnectivity {
    fn predecessors(self, facts: &ControlFlowFacts, block: BlockId) -> Option<&[BlockId]> {
        match self {
            Self::Normal => facts.predecessors(block),
            Self::ExceptionAware => facts.semantic_predecessors(block),
        }
    }
}

struct LoopFollow<'a> {
    cfg: &'a CFG,
    facts: &'a ControlFlowFacts,
    core: &'a BTreeSet<BlockId>,
    exits: BTreeSet<BlockId>,
    guard_follow: Option<BlockId>,
}

impl<'a> LoopFollow<'a> {
    fn new(
        cfg: &'a CFG,
        facts: &'a ControlFlowFacts,
        core: &'a BTreeSet<BlockId>,
        successors: &BTreeSet<BlockId>,
        latches: &BTreeSet<BlockId>,
    ) -> Self {
        let exits = successors
            .iter()
            .map(|target| facts.structural_continuation(*target))
            .filter(|target| !LoopExit::is_terminal(cfg, *target))
            .collect();
        let guard_follow = Self::guard_follow(cfg, facts, core, latches);
        Self {
            cfg,
            facts,
            core,
            exits,
            guard_follow,
        }
    }

    fn analyze(&self) -> Option<BlockId> {
        let convergence = self
            .facts
            .postdominators()
            .nearest_common(&self.exits.iter().copied().collect::<Vec<_>>());
        if convergence.is_some_and(|block| {
            self.cfg.block(block).is_some_and(|block| {
                block
                    .insns
                    .iter()
                    .any(|instruction| instruction.insn_type == InsnType::Phi)
            })
        }) {
            return convergence;
        }
        if self.guard_follow.is_some() {
            return self.guard_follow;
        }
        if self.exits.len() == 1 {
            return self.exits.first().copied();
        }
        convergence.or_else(|| self.weak())
    }

    fn guard_follow(
        cfg: &CFG,
        facts: &ControlFlowFacts,
        core: &BTreeSet<BlockId>,
        latches: &BTreeSet<BlockId>,
    ) -> Option<BlockId> {
        let exits = core
            .iter()
            .copied()
            .flat_map(|source| {
                cfg.normal_successors(source)
                    .filter(|target| !core.contains(target))
                    .map(move |target| (source, facts.structural_continuation(target)))
            })
            .filter(|(source, _)| {
                latches
                    .iter()
                    .all(|latch| facts.semantic_dominators().dominates(*source, *latch))
            })
            .collect::<Vec<_>>();
        let controlling = exits
            .iter()
            .filter(|(source, _)| {
                exits.iter().all(|(other, _)| {
                    source == other || facts.semantic_dominators().dominates(*source, *other)
                })
            })
            .map(|(_, target)| *target)
            .collect::<BTreeSet<_>>();
        match controlling.iter().copied().collect::<Vec<_>>().as_slice() {
            [target] => Some(*target),
            _ => None,
        }
    }

    fn weak(&self) -> Option<BlockId> {
        let domain = self.domain();
        let valid = domain
            .iter()
            .copied()
            .filter(|candidate| {
                !self.core.contains(candidate) && !LoopExit::is_terminal(self.cfg, *candidate) && {
                    let mut active = BTreeSet::new();
                    let mut memo = BTreeMap::new();
                    self.exits.iter().all(|start| {
                        Self::cover_node(
                            self.cfg,
                            self.core,
                            *start,
                            *candidate,
                            &mut active,
                            &mut memo,
                        )
                    })
                }
            })
            .collect::<BTreeSet<_>>();
        let earliest = valid
            .iter()
            .copied()
            .filter(|candidate| {
                valid
                    .iter()
                    .all(|other| candidate == other || self.reaches(*candidate, *other))
            })
            .collect::<Vec<_>>();
        match earliest.as_slice() {
            [candidate] => Some(*candidate),
            _ => None,
        }
    }

    fn domain(&self) -> BTreeSet<BlockId> {
        let mut domain = BTreeSet::new();
        let mut pending = self.exits.iter().copied().collect::<Vec<_>>();
        while let Some(block) = pending.pop() {
            if self.core.contains(&block) || !domain.insert(block) {
                continue;
            }
            pending.extend(self.cfg.normal_successors(block));
        }
        domain
    }

    fn cover_node(
        cfg: &CFG,
        core: &BTreeSet<BlockId>,
        block: BlockId,
        candidate: BlockId,
        active: &mut BTreeSet<BlockId>,
        memo: &mut BTreeMap<BlockId, bool>,
    ) -> bool {
        if block == candidate || LoopExit::is_terminal(cfg, block) {
            return true;
        }
        if core.contains(&block) {
            return false;
        }
        if let Some(result) = memo.get(&block) {
            return *result;
        }
        if !active.insert(block) {
            return false;
        }
        let mut successors = cfg.normal_successors(block).peekable();
        let result = successors.peek().is_some()
            && successors
                .all(|successor| Self::cover_node(cfg, core, successor, candidate, active, memo));
        active.remove(&block);
        memo.insert(block, result);
        result
    }

    fn reaches(&self, source: BlockId, target: BlockId) -> bool {
        let mut visited = BTreeSet::new();
        let mut pending = vec![source];
        while let Some(block) = pending.pop() {
            if block == target {
                return true;
            }
            if self.core.contains(&block) || !visited.insert(block) {
                continue;
            }
            pending.extend(self.cfg.normal_successors(block));
        }
        false
    }
}

/// Computes the lexical body of a natural loop.
///
/// The SCC gives the cyclic core, but source-level loop bodies also own exit
/// paths before the lexical follow and private paths that must return or throw.
/// The latter are a backward fixed point over normal successors, constrained
/// by dominance and closed normal predecessors so a shared continuation is not
/// pulled into the loop.
struct LoopBody<'a> {
    cfg: &'a CFG,
    facts: &'a ControlFlowFacts,
    header: BlockId,
    follow: Option<BlockId>,
}

impl<'a> LoopBody<'a> {
    fn new(
        cfg: &'a CFG,
        facts: &'a ControlFlowFacts,
        header: BlockId,
        follow: Option<BlockId>,
    ) -> Self {
        Self {
            cfg,
            facts,
            header,
            follow,
        }
    }

    fn analyze(&self, core: &BTreeSet<BlockId>) -> BTreeSet<BlockId> {
        let mut body = core.clone();
        self.extend_normal(core, &mut body);
        body.extend(self.abrupt(core, &body));
        body
    }

    fn extend_normal(&self, core: &BTreeSet<BlockId>, body: &mut BTreeSet<BlockId>) {
        let Some(follow) = self.follow else {
            return;
        };
        let mut candidates = BTreeSet::new();
        let mut active = BTreeSet::new();
        let mut memo = BTreeMap::new();
        let mut pending = self
            .successors(core)
            .into_iter()
            .filter(|target| *target != follow)
            .collect::<Vec<_>>();
        while let Some(block) = pending.pop() {
            if block == follow
                || self.cfg.block(block).is_none()
                || body.contains(&block)
                || candidates.contains(&block)
                || !self
                    .facts
                    .semantic_dominators()
                    .dominates(self.header, block)
                || !LoopFollow::cover_node(self.cfg, core, block, follow, &mut active, &mut memo)
            {
                continue;
            }
            candidates.insert(block);
            pending.extend(
                self.cfg
                    .normal_successors(block)
                    .filter(|target| *target != follow),
            );
        }

        self.remove_foreign_entries(body, &mut candidates);
        body.extend(candidates);
    }

    fn abrupt(
        &self,
        core: &BTreeSet<BlockId>,
        normal_body: &BTreeSet<BlockId>,
    ) -> BTreeSet<BlockId> {
        let terminating = self.terminating();
        let mut abrupt = BTreeSet::new();
        let mut pending = self
            .successors(core)
            .into_iter()
            .filter(|block| terminating.contains(block))
            .collect::<Vec<_>>();
        while let Some(block) = pending.pop() {
            if !terminating.contains(&block) || !abrupt.insert(block) {
                continue;
            }
            pending.extend(
                self.cfg
                    .normal_successors(block)
                    .filter(|successor| terminating.contains(successor)),
            );
        }

        self.remove_foreign_entries(normal_body, &mut abrupt);
        abrupt
    }

    fn remove_foreign_entries(&self, body: &BTreeSet<BlockId>, candidates: &mut BTreeSet<BlockId>) {
        let mut foreign = candidates
            .iter()
            .copied()
            .filter(|block| {
                self.facts
                    .predecessors(*block)
                    .into_iter()
                    .flatten()
                    .any(|parent| !body.contains(parent) && !candidates.contains(parent))
            })
            .collect::<Vec<_>>();
        while let Some(block) = foreign.pop() {
            if candidates.remove(&block) && !body.contains(&block) {
                foreign.extend(
                    self.cfg
                        .normal_successors(block)
                        .filter(|target| candidates.contains(target)),
                );
            }
        }
    }

    fn terminating(&self) -> BTreeSet<BlockId> {
        let mut terminating = BTreeSet::new();
        let mut remaining = BTreeMap::new();
        for block in self.cfg.blocks.keys().copied() {
            if Some(block) == self.follow
                || !self
                    .facts
                    .semantic_dominators()
                    .dominates(self.header, block)
            {
                continue;
            }
            if LoopExit::is_terminal(self.cfg, block) {
                terminating.insert(block);
            } else {
                let count = self.cfg.normal_successors(block).count();
                if count != 0 {
                    remaining.insert(block, count);
                }
            }
        }
        let mut pending = terminating.iter().copied().collect::<Vec<_>>();
        while let Some(block) = pending.pop() {
            // Count edge occurrences on both sides. A switch may have several
            // distinct case edges leading to the same successor.
            for &(predecessor, kind) in self.cfg.predecessors_with_kind(block) {
                if kind.is_exception() {
                    continue;
                }
                if let Some(count) = remaining.get_mut(&predecessor) {
                    *count -= 1;
                    if *count == 0 {
                        terminating.insert(predecessor);
                        pending.push(predecessor);
                    }
                }
            }
        }
        terminating
    }

    fn successors(&self, blocks: &BTreeSet<BlockId>) -> BTreeSet<BlockId> {
        blocks
            .iter()
            .copied()
            .flat_map(|source| self.cfg.normal_successors(source))
            .filter(|target| !blocks.contains(target))
            .collect()
    }
}

struct SwitchFollow<'a> {
    facts: &'a ControlFlowFacts,
    header: BlockId,
    successors: &'a [BlockId],
}

impl<'a> SwitchFollow<'a> {
    fn new(facts: &'a ControlFlowFacts, header: BlockId, successors: &'a [BlockId]) -> Self {
        Self {
            facts,
            header,
            successors,
        }
    }

    fn analyze(&self) -> Option<BlockId> {
        let dominated = |node: &BlockId| {
            *node != self.header && self.facts.dominators().dominates(self.header, *node)
        };
        self.facts
            .postdominators()
            .nearest_common(self.successors)
            .filter(dominated)
            .or_else(|| {
                self.facts
                    .postdominators()
                    .convergences(self.successors, 2)
                    .into_iter()
                    .find(dominated)
            })
    }
}

struct LoopExit;

impl LoopExit {
    fn is_terminal(cfg: &CFG, block: BlockId) -> bool {
        cfg.normal_successors(block).next().is_none()
            && cfg.block(block).is_some_and(|block| {
                block.terminator().is_some_and(|instruction| {
                    instruction.insn_type.is_terminal() || instruction.payload.no_return
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ArgType, Block, EdgeKind, InsnArg, InsnNode, RegisterArg};

    #[test]
    fn loop_region_owns_phi_edge_blocks_until_its_common_follow() {
        let mut cfg = CFG::new("loop_exit_arm");
        for id in 0..=6 {
            if id == 5 {
                let mut edge = Block::synthetic(id);
                edge.push(crate::ir::InsnNode::goto(6));
                cfg.add_block(edge);
            } else {
                cfg.add_block(Block::new(id));
            }
        }
        cfg.block_mut(BlockId::new(6)).unwrap().push(InsnNode::phi(
            RegisterArg::new_ssa(0, 1, ArgType::INT),
            vec![
                (4, InsnArg::reg_ssa(0, 0, ArgType::INT)),
                (5, InsnArg::reg_ssa(0, 0, ArgType::INT)),
            ],
        ));
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::True);
        cfg.add_edge(BlockId::new(1), BlockId::new(5), EdgeKind::False);
        cfg.add_edge(BlockId::new(5), BlockId::new(6), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(2), BlockId::new(1), EdgeKind::True);
        cfg.add_edge(BlockId::new(2), BlockId::new(3), EdgeKind::False);
        cfg.add_edge(BlockId::new(3), BlockId::new(4), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(4), BlockId::new(6), EdgeKind::Normal);

        let facts = ControlFlowFacts::analyze(&cfg).unwrap();
        let analysis = ControlRegionAnalysis::new(&cfg, &facts);
        let candidates = analysis.normal_loops().unwrap().loops;

        let candidate = candidates
            .iter()
            .find(|candidate| candidate.entry == BlockId::new(1))
            .unwrap();
        assert_eq!(
            candidate.blocks,
            BTreeSet::from([
                BlockId::new(1),
                BlockId::new(2),
                BlockId::new(3),
                BlockId::new(4),
                BlockId::new(5),
            ])
        );
        assert_eq!(candidate.kind.follow(), Some(BlockId::new(6)));
    }

    #[test]
    fn loop_follow_contracts_distinct_phi_edge_adapters() {
        let mut cfg = CFG::new("loop_phi_exit_adapters");
        for id in 0..=7 {
            if matches!(id, 5 | 6) {
                let mut edge = Block::synthetic(id);
                edge.push(InsnNode::goto(7));
                cfg.add_block(edge);
            } else {
                cfg.add_block(Block::new(id));
            }
        }
        cfg.block_mut(BlockId::new(7)).unwrap().push(InsnNode::phi(
            RegisterArg::new_ssa(0, 1, ArgType::INT),
            vec![
                (5, InsnArg::reg_ssa(0, 0, ArgType::INT)),
                (6, InsnArg::reg_ssa(0, 0, ArgType::INT)),
            ],
        ));
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(2), BlockId::new(3), EdgeKind::True);
        cfg.add_edge(BlockId::new(2), BlockId::new(4), EdgeKind::False);
        cfg.add_edge(BlockId::new(3), BlockId::new(5), EdgeKind::True);
        cfg.add_edge(BlockId::new(3), BlockId::new(1), EdgeKind::False);
        cfg.add_edge(BlockId::new(4), BlockId::new(6), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(5), BlockId::new(7), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(6), BlockId::new(7), EdgeKind::Normal);

        let facts = ControlFlowFacts::analyze(&cfg).unwrap();
        assert_eq!(facts.continuation(BlockId::new(5)), BlockId::new(5));
        assert_eq!(
            facts.structural_continuation(BlockId::new(5)),
            BlockId::new(7)
        );
        assert_eq!(
            facts.structural_continuation(BlockId::new(6)),
            BlockId::new(7)
        );

        let analysis = ControlRegionAnalysis::new(&cfg, &facts);
        let candidates = analysis.normal_loops().unwrap().loops;

        let candidate = candidates
            .iter()
            .find(|candidate| candidate.entry == BlockId::new(1))
            .unwrap();
        assert_eq!(candidate.kind.follow(), Some(BlockId::new(7)));
        assert!(candidate.blocks.contains(&BlockId::new(5)));
        assert!(candidate.blocks.contains(&BlockId::new(6)));
    }

    #[test]
    fn terminal_loop_exit_does_not_hide_lexical_follow() {
        let mut cfg = CFG::new("loop_terminal_exit");
        for id in 0..=5 {
            cfg.add_block(Block::new(id));
        }
        cfg.block_mut(BlockId::new(3))
            .unwrap()
            .push(crate::ir::InsnNode::new(InsnType::Return, 3));
        cfg.block_mut(BlockId::new(5))
            .unwrap()
            .push(crate::ir::InsnNode::new(InsnType::Return, 5));
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::True);
        cfg.add_edge(BlockId::new(1), BlockId::new(4), EdgeKind::False);
        cfg.add_edge(BlockId::new(2), BlockId::new(3), EdgeKind::True);
        cfg.add_edge(BlockId::new(2), BlockId::new(1), EdgeKind::False);
        cfg.add_edge(BlockId::new(4), BlockId::new(5), EdgeKind::Normal);

        let facts = ControlFlowFacts::analyze(&cfg).unwrap();
        let analysis = ControlRegionAnalysis::new(&cfg, &facts);
        let candidates = analysis.normal_loops().unwrap().loops;

        let candidate = candidates
            .iter()
            .find(|candidate| candidate.entry == BlockId::new(1))
            .unwrap();
        assert_eq!(candidate.kind.follow(), Some(BlockId::new(4)));
    }

    #[test]
    fn branching_terminal_exit_does_not_hide_loop_follow() {
        let mut cfg = CFG::new("loop_branching_terminal_exit");
        for id in 0..=8 {
            cfg.add_block(Block::new(id));
        }
        cfg.block_mut(BlockId::new(4))
            .unwrap()
            .push(crate::ir::InsnNode::new(InsnType::Return, 4));
        cfg.block_mut(BlockId::new(8))
            .unwrap()
            .push(crate::ir::InsnNode::new(InsnType::Return, 8));
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(1), BlockId::new(7), EdgeKind::True);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::False);
        cfg.add_edge(BlockId::new(2), BlockId::new(3), EdgeKind::True);
        cfg.add_edge(BlockId::new(2), BlockId::new(1), EdgeKind::False);
        cfg.add_edge(BlockId::new(3), BlockId::new(4), EdgeKind::True);
        cfg.add_edge(BlockId::new(3), BlockId::new(5), EdgeKind::False);
        cfg.add_edge(BlockId::new(5), BlockId::new(6), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(6), BlockId::new(7), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(7), BlockId::new(8), EdgeKind::Normal);

        let facts = ControlFlowFacts::analyze(&cfg).unwrap();
        let analysis = ControlRegionAnalysis::new(&cfg, &facts);
        let candidates = analysis.normal_loops().unwrap().loops;

        let candidate = candidates
            .iter()
            .find(|candidate| candidate.entry == BlockId::new(1))
            .unwrap();
        assert_eq!(candidate.kind.follow(), Some(BlockId::new(7)));
        assert_eq!(
            candidate.blocks,
            (1..=6).map(BlockId::new).collect::<BTreeSet<_>>()
        );
    }

    #[test]
    fn header_exit_is_the_lexical_follow_before_nonlocal_body_exits() {
        let mut cfg = CFG::new("loop_with_nonlocal_exit");
        for id in [0, 1, 2, 3, 4, 5, 7] {
            cfg.add_block(Block::new(id));
        }
        cfg.block_mut(BlockId::new(7))
            .unwrap()
            .push(crate::ir::InsnNode::new(InsnType::Return, 7));
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(2), BlockId::new(3), EdgeKind::True);
        cfg.add_edge(BlockId::new(2), BlockId::new(4), EdgeKind::False);
        cfg.add_edge(BlockId::new(3), BlockId::new(1), EdgeKind::True);
        cfg.add_edge(BlockId::new(3), BlockId::new(7), EdgeKind::False);
        cfg.add_edge(BlockId::new(4), BlockId::new(5), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(5), BlockId::new(7), EdgeKind::Normal);

        let facts = ControlFlowFacts::analyze(&cfg).unwrap();
        let analysis = ControlRegionAnalysis::new(&cfg, &facts);
        let candidates = analysis.normal_loops().unwrap().loops;

        let candidate = candidates
            .iter()
            .find(|candidate| candidate.entry == BlockId::new(1))
            .unwrap();
        assert_eq!(candidate.kind.follow(), Some(BlockId::new(4)));
        assert_eq!(
            candidate.blocks,
            BTreeSet::from([BlockId::new(1), BlockId::new(2), BlockId::new(3)])
        );
    }

    #[test]
    fn lexical_follow_precedes_nonterminal_exit_convergence() {
        let mut cfg = CFG::new("loop_with_late_exit_convergence");
        for id in [0, 1, 2, 3, 4, 5, 7, 8] {
            cfg.add_block(Block::new(id));
        }
        cfg.block_mut(BlockId::new(8))
            .unwrap()
            .push(crate::ir::InsnNode::new(InsnType::Return, 8));
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(2), BlockId::new(3), EdgeKind::True);
        cfg.add_edge(BlockId::new(2), BlockId::new(4), EdgeKind::False);
        cfg.add_edge(BlockId::new(3), BlockId::new(1), EdgeKind::True);
        cfg.add_edge(BlockId::new(3), BlockId::new(7), EdgeKind::False);
        cfg.add_edge(BlockId::new(4), BlockId::new(5), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(5), BlockId::new(7), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(7), BlockId::new(8), EdgeKind::Normal);

        let facts = ControlFlowFacts::analyze(&cfg).unwrap();
        let analysis = ControlRegionAnalysis::new(&cfg, &facts);
        let candidates = analysis.normal_loops().unwrap().loops;

        let candidate = candidates
            .iter()
            .find(|candidate| candidate.entry == BlockId::new(1))
            .unwrap();
        assert_eq!(candidate.kind.follow(), Some(BlockId::new(4)));
        assert_eq!(
            candidate.blocks,
            BTreeSet::from([BlockId::new(1), BlockId::new(2), BlockId::new(3)])
        );
    }

    #[test]
    fn loop_closure_owns_shared_handler_continuation_returning_to_header() {
        let mut cfg = CFG::new("loop_handler_continuation");
        for id in [0, 1, 2, 4, 5, 7] {
            cfg.add_block(Block::new(id));
        }
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(2), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(2), BlockId::new(4), EdgeKind::Exception);
        cfg.add_edge(BlockId::new(2), BlockId::new(5), EdgeKind::Exception);
        cfg.add_edge(BlockId::new(4), BlockId::new(7), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(5), BlockId::new(7), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(7), BlockId::new(1), EdgeKind::Normal);

        let facts = ControlFlowFacts::analyze(&cfg).unwrap();
        let mut tree = RegionTree::new(Some(BlockId::new(0)));
        tree.cover_method(&cfg).unwrap();
        let handlers = BTreeMap::new();
        let domain = cfg.block_ids().into_iter().collect();
        let closure =
            ControlRegionClosure::new(&cfg, &facts, &tree, &handlers, BlockId::new(1), &domain);
        let closed = closure
            .close(
                &BTreeSet::from([
                    BlockId::new(1),
                    BlockId::new(2),
                    BlockId::new(4),
                    BlockId::new(5),
                ]),
                true,
            )
            .unwrap();

        assert!(closed.contains(&BlockId::new(7)));
    }

    fn assert_private_terminal_switch_is_nested(terminal: InsnNode) {
        let mut cfg = CFG::new("loop_switch_throw");
        for id in 0..=8 {
            cfg.add_block(Block::new(id));
        }
        cfg.block_mut(BlockId::new(2))
            .unwrap()
            .push(crate::ir::InsnNode::new(InsnType::Switch, 2));
        cfg.block_mut(BlockId::new(5)).unwrap().push(terminal);
        cfg.block_mut(BlockId::new(8))
            .unwrap()
            .push(crate::ir::InsnNode::new(InsnType::Return, 8));
        cfg.add_edge(BlockId::new(0), BlockId::new(1), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(1), BlockId::new(8), EdgeKind::True);
        cfg.add_edge(BlockId::new(1), BlockId::new(2), EdgeKind::False);
        cfg.add_edge(BlockId::new(2), BlockId::new(3), EdgeKind::SwitchCase(0));
        cfg.add_edge(BlockId::new(2), BlockId::new(4), EdgeKind::SwitchCase(1));
        cfg.add_edge(BlockId::new(2), BlockId::new(7), EdgeKind::SwitchDefault);
        cfg.add_edge(BlockId::new(3), BlockId::new(7), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(4), BlockId::new(5), EdgeKind::True);
        cfg.add_edge(BlockId::new(4), BlockId::new(6), EdgeKind::False);
        cfg.add_edge(BlockId::new(6), BlockId::new(7), EdgeKind::Normal);
        cfg.add_edge(BlockId::new(7), BlockId::new(1), EdgeKind::Normal);

        let facts = ControlFlowFacts::analyze(&cfg).unwrap();
        let analysis = ControlRegionAnalysis::new(&cfg, &facts);
        let mut tree = RegionTree::new(Some(BlockId::new(0)));
        tree.cover_method(&cfg).unwrap();
        analysis.apply(&mut tree).unwrap();

        let loop_region = tree
            .regions()
            .find(|region| {
                region.entry == Some(BlockId::new(1)) && matches!(&region.kind, RegionKind::Loop(_))
            })
            .unwrap();
        assert!(loop_region.blocks.contains(&BlockId::new(5)));
        let switch_region = tree
            .regions()
            .find(|region| {
                region.entry == Some(BlockId::new(2))
                    && matches!(&region.kind, RegionKind::Switch(_))
            })
            .unwrap();
        assert_eq!(switch_region.parent, Some(loop_region.id));
        assert!(switch_region.blocks.contains(&BlockId::new(5)));
        assert_eq!(switch_region.kind.follow(), Some(BlockId::new(7)));
    }

    #[test]
    fn switch_with_a_private_throw_is_nested_in_its_loop() {
        assert_private_terminal_switch_is_nested(InsnNode::new(InsnType::Throw, 5));
    }

    #[test]
    fn switch_with_a_private_no_return_call_is_nested_in_its_loop() {
        let mut no_return = InsnNode::new(InsnType::Invoke, 5);
        no_return.payload.no_return = true;
        assert_private_terminal_switch_is_nested(no_return);
    }
}
