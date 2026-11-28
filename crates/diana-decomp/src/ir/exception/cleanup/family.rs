//! Recovery of a cleanup shared by several physical catch-all entries.

use super::{
    same_physical_dispatch, BlockId, CleanupProof, CleanupRecovery, CleanupRecoveryResult,
    CleanupRootFilter, HandlerKind, InsnType, InstructionEffects, NormalCleanupCandidates,
    TryRegion, CFG,
};
use crate::ir::exception::{CatchHandler, ExceptionInvariantError};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::ir::exception) struct RecoveredFamily {
    pub(in crate::ir::exception) scope: TryRegion,
    pub(in crate::ir::exception) participants: BTreeSet<(u32, u32)>,
    pub(in crate::ir::exception) recovery: CleanupRecoveryResult,
}

impl CleanupRecovery<'_> {
    pub(in crate::ir::exception) fn recover_families(
        &self,
        regions: &[TryRegion],
        representatives: &BTreeMap<BlockId, BlockId>,
    ) -> Result<Vec<RecoveredFamily>, ExceptionInvariantError> {
        let mut occurrences = BTreeMap::<BlockId, Vec<(&TryRegion, &CatchHandler)>>::new();
        for region in regions {
            for handler in &region.handlers {
                for &rethrow in &handler.rethrow_blocks {
                    occurrences
                        .entry(rethrow)
                        .or_default()
                        .push((region, handler));
                }
            }
        }
        let mut next_id = regions.iter().map(|region| region.id).max().unwrap_or(0);
        let mut recovered = Vec::new();
        for (rethrow, owners) in occurrences {
            let entries = owners
                .iter()
                .flat_map(|(_, handler)| handler.entry_blocks.iter().copied())
                .collect::<BTreeSet<_>>();
            if entries.len() < 2
                || owners.iter().any(|(_, handler)| {
                    handler.kind != HandlerKind::Cleanup
                        || !handler.is_catch_all()
                        || handler.rethrow_blocks.len() != 1
                })
            {
                continue;
            }
            let Some(family) = CleanupFamily::analyze(self.cfg, entries, rethrow) else {
                continue;
            };
            let Some(id) = next_id.checked_add(1) else {
                break;
            };
            let Some(mut scope) = family.scope(id, &owners) else {
                continue;
            };
            let result = self.recover_scope(
                &mut scope,
                &BTreeSet::new(),
                &BTreeSet::new(),
                representatives,
                true,
            )?;
            if scope.handlers[0].kind == HandlerKind::Finally {
                next_id = id;
                recovered.push(RecoveredFamily {
                    scope,
                    participants: owners
                        .iter()
                        .map(|(region, handler)| (region.id, handler.id))
                        .collect(),
                    recovery: result,
                });
            }
        }
        Ok(recovered)
    }
}

struct CleanupFamily<'cfg> {
    cfg: &'cfg CFG,
    entries: BTreeSet<BlockId>,
    adapters: BTreeSet<BlockId>,
    cleanup: BlockId,
    rethrow: BlockId,
}

impl<'cfg> CleanupFamily<'cfg> {
    fn analyze(cfg: &'cfg CFG, entries: BTreeSet<BlockId>, rethrow: BlockId) -> Option<Self> {
        let mut adapters = BTreeSet::new();
        let mut cleanup = None;
        for &entry in &entries {
            let mut visited = BTreeSet::new();
            let mut block = entry;
            loop {
                if !visited.insert(block) {
                    return None;
                }
                let body = cfg.block(block)?;
                if body.insns.iter().any(|insn| {
                    !matches!(
                        insn.insn_type,
                        InsnType::Goto | InsnType::MoveException | InsnType::Throw
                    ) && !InstructionEffects::is_ssa_bookkeeping(insn)
                        && !InstructionEffects::of_tree(insn).is_pure()
                }) {
                    if cleanup.is_some_and(|known| known != block) {
                        return None;
                    }
                    cleanup = Some(block);
                    break;
                }
                if cfg
                    .successors_with_kind(block)
                    .iter()
                    .any(|(_, kind)| kind.is_exception())
                {
                    return None;
                }
                let mut next = cfg.normal_successors(block);
                let target = next.next()?;
                if next.next().is_some() {
                    return None;
                }
                adapters.insert(block);
                block = target;
            }
        }
        let cleanup = cleanup?;
        let pending_throw = cfg.block(rethrow)?.terminator()?;
        if cfg
            .block(cleanup)?
            .insns
            .iter()
            .filter(|insn| insn.can_throw())
            .any(|insn| !same_physical_dispatch(cfg, insn.offset, pending_throw.offset))
        {
            return None;
        }
        if (cleanup != rethrow
            && (!cfg.normal_successors(cleanup).eq([rethrow])
                || cfg
                    .incoming_edges(rethrow)
                    .iter()
                    .any(|(source, kind)| *source != cleanup || kind.is_exception())))
            || cfg
                .incoming_edges(cleanup)
                .iter()
                .any(|(source, kind)| kind.is_exception() || !adapters.contains(source))
        {
            return None;
        }
        for &adapter in &adapters {
            if cfg.incoming_edges(adapter).iter().any(|(source, kind)| {
                if kind.is_exception() {
                    !entries.contains(&adapter)
                } else {
                    !adapters.contains(source)
                }
            }) {
                return None;
            }
        }
        Some(Self {
            cfg,
            entries,
            adapters,
            cleanup,
            rethrow,
        })
    }

    fn scope(&self, id: u32, owners: &[(&TryRegion, &CatchHandler)]) -> Option<TryRegion> {
        let (_, template) = owners.first()?;
        let pending = &template.rethrow_blocks;
        let starts = owners
            .iter()
            .flat_map(|(region, _)| region.normal_exit_blocks.iter().copied())
            .flat_map(|source| self.cfg.normal_successors(source))
            .filter(|block| !self.adapters.contains(block) && *block != self.cleanup)
            .collect::<BTreeSet<_>>();
        let completions = starts
            .into_iter()
            .flat_map(|start| NormalCleanupCandidates::new(self.cfg, start).entries())
            .filter(|candidate| {
                *candidate != self.cleanup
                    && !self.adapters.contains(candidate)
                    && self.cfg.block(*candidate).is_some_and(|body| {
                        !CleanupProof::body_instructions(body, false).is_empty()
                    })
                    && CleanupRootFilter::new(self.cfg, pending).accepts(self.cleanup, *candidate)
            })
            .collect::<BTreeSet<_>>();
        if completions.is_empty() {
            return None;
        }
        let sources = self
            .entries
            .iter()
            .flat_map(|entry| self.cfg.incoming_edges(*entry))
            .filter_map(|(block, kind)| kind.is_exception().then_some(block))
            .collect::<BTreeSet<_>>();
        let entry = sources
            .iter()
            .copied()
            .min_by_key(|block| self.cfg.block(*block).map(|body| body.offset))?;
        let blocks = self.protected_blocks(entry, &sources, &completions)?;
        let mut handler = (*template).clone();
        handler.semantic_entry = self.cleanup;
        handler.canonical_entry = self.cleanup;
        let exception = self
            .cfg
            .block(self.rethrow)?
            .terminator()?
            .args
            .first()?
            .as_register()?
            .clone();
        handler.exception_value = Some(exception.clone());
        handler.canonical_exception_value = Some(exception);
        handler.entry_blocks = self.entries.clone();
        handler.adapter_blocks = self.adapters.clone();
        handler.semantic_blocks = vec![self.cleanup];
        if self.rethrow != self.cleanup {
            handler.semantic_blocks.push(self.rethrow);
        }
        handler.blocks = self
            .adapters
            .iter()
            .copied()
            .chain(handler.semantic_blocks.iter().copied())
            .collect();
        handler.lexical_blocks = handler.semantic_blocks.clone();
        let normal_exit_blocks = blocks
            .iter()
            .copied()
            .filter(|block| {
                self.cfg
                    .normal_successors(*block)
                    .any(|next| completions.contains(&next))
            })
            .collect::<Vec<_>>();
        if normal_exit_blocks.is_empty() {
            return None;
        }
        let start_offset = self.cfg.block(entry)?.offset;
        let end_offset = blocks
            .iter()
            .filter_map(|block| self.cfg.block(*block))
            .filter(|body| !body.synthetic)
            .flat_map(|body| {
                body.insns
                    .iter()
                    .map(|insn| insn.offset)
                    .chain([body.offset])
            })
            .filter(|offset| *offset != u32::MAX)
            .max()?
            .checked_add(1)?;
        Some(TryRegion {
            id,
            start_offset,
            end_offset,
            blocks: blocks.into_iter().collect(),
            handlers: vec![handler],
            parent: None,
            children: Vec::new(),
            normal_exit_blocks,
        })
    }

    fn protected_blocks(
        &self,
        entry: BlockId,
        sources: &BTreeSet<BlockId>,
        completions: &BTreeSet<BlockId>,
    ) -> Option<BTreeSet<BlockId>> {
        let mut blocks = BTreeSet::new();
        let mut pending = vec![entry];
        while let Some(block) = pending.pop() {
            if !blocks.insert(block) {
                continue;
            }
            let body = self.cfg.block(block)?;
            let successors = self.cfg.successors_with_kind(block);
            if successors.is_empty()
                || body
                    .insns
                    .iter()
                    .filter(|insn| insn.can_throw())
                    .any(|insn| {
                        !self
                            .cfg
                            .handlers
                            .iter()
                            .any(|clause| clause.is_catch_all() && clause.covers(insn.offset))
                    })
            {
                return None;
            }
            for &(target, kind) in successors {
                if self.entries.contains(&target) {
                    if !kind.is_exception() {
                        return None;
                    }
                } else if completions.contains(&target) {
                    if kind.is_exception() {
                        return None;
                    }
                } else if self.adapters.contains(&target)
                    || target == self.cleanup
                    || target == self.rethrow
                {
                    return None;
                } else {
                    pending.push(target);
                }
            }
        }
        if !sources.is_subset(&blocks) {
            return None;
        }
        for exit in [self.cleanup, self.rethrow] {
            if self
                .cfg
                .successors_with_kind(exit)
                .iter()
                .any(|(target, kind)| {
                    kind.is_exception()
                        && (blocks.contains(target)
                            || self.adapters.contains(target)
                            || completions.contains(target))
                })
            {
                return None;
            }
        }
        for &block in &blocks {
            if block == entry {
                continue;
            }
            if self
                .cfg
                .incoming_edges(block)
                .iter()
                .any(|(source, _)| !blocks.contains(source))
            {
                return None;
            }
        }
        Some(blocks)
    }
}
