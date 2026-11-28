//! Exact inputs for a catch shared by the copies of a finally body.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{
    analysis::{InstructionEffects, PhiMerge, SsaValueGraph, SsaVar},
    exception::{CatchHandler, CleanupCatchValue},
    BlockId, InsnArg, InsnType, RegionEdge, StatementOrigin, CFG,
};

pub(in crate::ir::exception) struct CatchValueTransfer {
    result: SsaVar,
    changes_input: bool,
    replaced: BTreeSet<BlockId>,
    inputs: BTreeMap<RegionEdge, SsaVar>,
}

pub(super) fn analyze(
    cfg: &CFG,
    values: &SsaValueGraph,
    handler: &CatchHandler,
    normal_cleanup: &BTreeSet<BlockId>,
) -> Vec<CatchValueTransfer> {
    let proof = CatchValues {
        cfg,
        values,
        handler,
        normal_cleanup,
    };
    let targets = cfg.successors_with_kind(handler.canonical_entry);
    let mut transfers = Vec::new();
    for &(target, kind) in targets {
        if kind.is_exception() && proof.owns_catch(target) {
            let Some(block) = cfg.block(target) else {
                continue;
            };
            for phi in block
                .insns
                .iter()
                .filter(|insn| insn.insn_type == InsnType::Phi)
                .filter_map(|insn| insn.result.as_ref())
                .filter_map(SsaVar::from_reg)
                .filter_map(|result| values.phi(result))
            {
                if let Some(transfer) = proof.inputs(phi) {
                    transfers.push(transfer);
                }
            }
        }
    }
    transfers
}

pub(in crate::ir::exception) fn compose(
    cfg: &CFG,
    values: &SsaValueGraph,
    transfers: Vec<CatchValueTransfer>,
    elided: &BTreeSet<StatementOrigin>,
    contracted: &BTreeMap<BlockId, BlockId>,
) -> BTreeMap<SsaVar, CleanupCatchValue> {
    let mut groups = BTreeMap::<SsaVar, Vec<CatchValueTransfer>>::new();
    for transfer in transfers {
        groups.entry(transfer.result).or_default().push(transfer);
    }
    groups
        .into_iter()
        .filter_map(|(result, group)| {
            if !group.iter().any(|transfer| transfer.changes_input) {
                return None;
            }
            let phi = values.phi(result)?;
            let mut inputs = BTreeMap::new();
            let mut replaced = BTreeSet::new();
            for transfer in group {
                if !replaced.is_disjoint(&transfer.replaced) {
                    return None;
                }
                replaced.extend(transfer.replaced);
                for (edge, value) in transfer.inputs {
                    add_input(&mut inputs, edge, value)?;
                }
            }
            // Direct entries, such as a call after normal cleanup, still stage
            // their own value. Account for them only after all families are known.
            for input in &phi.inputs {
                if replaced.contains(&input.predecessor) {
                    continue;
                }
                let edge = RegionEdge {
                    source: input.predecessor,
                    target: phi.block,
                    kind: input.edge_kind,
                };
                if !edge.kind.is_exception() {
                    return None;
                }
                add_input(&mut inputs, edge, input.value)?;
            }
            let mut sites = BTreeMap::new();
            for (&edge, &value) in &inputs {
                if edge.kind.is_exception() {
                    let origin = throw_origin(cfg, edge.source)?;
                    if elided.contains(&origin) {
                        return None;
                    }
                } else if contracted.contains_key(&edge.source) {
                    return None;
                }
                // The assignment runs before dispatch chooses an exception edge.
                // It may represent multiple outcomes only when they agree.
                let site = (edge.source, edge.kind.is_exception());
                if sites
                    .insert(site, value)
                    .is_some_and(|prior| prior != value)
                {
                    return None;
                }
            }
            let mut seen = BTreeSet::new();
            inputs.retain(|edge, _| seen.insert((edge.source, edge.kind.is_exception())));
            Some((
                result,
                CleanupCatchValue {
                    target: phi.block,
                    inputs,
                },
            ))
        })
        .collect()
}

fn throw_origin(cfg: &CFG, block: BlockId) -> Option<StatementOrigin> {
    let terminator = cfg.block(block)?.terminator()?;
    terminator.can_throw().then_some(StatementOrigin {
        block,
        instruction: terminator.id,
    })
}

struct CatchValues<'a> {
    cfg: &'a CFG,
    values: &'a SsaValueGraph,
    handler: &'a CatchHandler,
    normal_cleanup: &'a BTreeSet<BlockId>,
}

impl CatchValues<'_> {
    fn owns_catch(&self, target: BlockId) -> bool {
        if self
            .cfg
            .predecessors_with_kind(target)
            .iter()
            .any(|edge| !edge.1.is_exception())
            || !self.cfg.block(target).is_some_and(|block| {
                block
                    .insns
                    .iter()
                    .any(|insn| insn.insn_type == InsnType::MoveException)
            })
        {
            return false;
        }
        // A capture may be split into a handler pad and its first protected
        // operation. A continuation also reached outside this catch cannot
        // establish an owned restore port.
        let mut visited = BTreeSet::new();
        let mut current = target;
        while visited.insert(current) {
            let Some(block) = self.cfg.block(current) else {
                return false;
            };
            if block.insns.iter().any(|insn| {
                !InstructionEffects::is_ssa_bookkeeping(insn)
                    && !InstructionEffects::of_tree(insn).is_pure()
                    && !matches!(insn.insn_type, InsnType::MoveException | InsnType::Throw)
            }) {
                return true;
            }
            let mut successors = self.cfg.normal_successors(current);
            let Some(next) = successors.next() else {
                return false;
            };
            if successors.next().is_some()
                || self
                    .cfg
                    .predecessors_with_kind(next)
                    .iter()
                    .any(|&(source, kind)| source != current || kind.is_exception())
            {
                return false;
            }
            current = next;
        }
        false
    }

    fn inputs(&self, phi: &PhiMerge) -> Option<CatchValueTransfer> {
        let canonical = self.handler.canonical_entry;
        let value = Self::exceptional_input(phi, canonical)?;
        if self.values.value(phi.result)?.uses.is_empty() {
            return None;
        }
        let changes_input = self.normal_cleanup.iter().any(|&block| {
            Self::exceptional_input(phi, block).is_some_and(|normal| normal != value)
        });
        // If cancellation throws, or succeeds and lets the pending fault
        // resume, the same outer catch must observe the same value.
        if self
            .handler
            .rethrow_blocks
            .iter()
            .any(|&block| Self::exceptional_input(phi, block) != Some(value))
        {
            return None;
        }
        let mut inputs = self.abrupt_inputs(canonical, value)?;
        for &block in self.normal_cleanup {
            let value = Self::exceptional_input(phi, block)?;
            let predecessors = self.cfg.predecessors_with_kind(block);
            if predecessors.is_empty() {
                return None;
            }
            for &(source, kind) in predecessors {
                if kind.is_exception() || !self.cfg.normal_successors(source).eq([block]) {
                    return None;
                }
                let edge = RegionEdge {
                    source,
                    target: block,
                    kind,
                };
                let value = self.value_on_entry(edge, value)?;
                add_input(&mut inputs, edge, value)?;
            }
        }
        let mut replaced = self.normal_cleanup.clone();
        replaced.insert(canonical);
        replaced.extend(self.handler.rethrow_blocks.iter().copied());
        Some(CatchValueTransfer {
            result: phi.result,
            changes_input,
            replaced,
            inputs,
        })
    }

    fn exceptional_input(phi: &PhiMerge, source: BlockId) -> Option<SsaVar> {
        phi.inputs.iter().find_map(|input| {
            (input.predecessor == source && input.edge_kind.is_exception()).then_some(input.value)
        })
    }

    fn abrupt_inputs(&self, entry: BlockId, value: SsaVar) -> Option<BTreeMap<RegionEdge, SsaVar>> {
        let mut inputs = BTreeMap::new();
        let mut pending = vec![(entry, value)];
        let mut visited = BTreeSet::new();
        while let Some((target, value)) = pending.pop() {
            if !visited.insert((target, value)) {
                continue;
            }
            let predecessors = self.cfg.predecessors_with_kind(target);
            if predecessors.is_empty() {
                return None;
            }
            for &(source, kind) in predecessors {
                let edge = RegionEdge {
                    source,
                    target,
                    kind,
                };
                let value = self.value_on_entry(edge, value)?;
                if !kind.is_exception() && self.handler.adapter_blocks.contains(&source) {
                    pending.push((source, value));
                } else if self.can_capture(edge) {
                    add_input(&mut inputs, edge, value)?;
                } else {
                    return None;
                }
            }
        }
        (!inputs.is_empty()).then_some(inputs)
    }

    fn value_on_entry(&self, edge: RegionEdge, mut value: SsaVar) -> Option<SsaVar> {
        while let Some(definition) = self.values.value(value)?.definition {
            if definition.block != edge.target {
                break;
            }
            if let Some(phi) = self.values.phi(value) {
                return phi.inputs.iter().find_map(|input| {
                    (input.predecessor == edge.source && input.edge_kind == edge.kind)
                        .then_some(input.value)
                });
            }
            let move_insn = self.cfg.block(edge.target)?.insns.get(definition.index)?;
            if move_insn.insn_type != InsnType::Move {
                return None;
            }
            let source = move_insn.args.first().and_then(InsnArg::as_register)?;
            let source = SsaVar::from_reg(source)?;
            if self.values.value(source)?.definition.is_some_and(|prior| {
                prior.block == definition.block && prior.index >= definition.index
            }) {
                return None;
            }
            value = source;
        }
        Some(value)
    }

    fn can_capture(&self, edge: RegionEdge) -> bool {
        edge.kind.is_exception()
            && !self.normal_cleanup.contains(&edge.source)
            && !self.handler.rethrow_blocks.contains(&edge.source)
            && throw_origin(self.cfg, edge.source).is_some()
    }
}

fn add_input(
    inputs: &mut BTreeMap<RegionEdge, SsaVar>,
    edge: RegionEdge,
    value: SsaVar,
) -> Option<()> {
    if inputs
        .insert(edge, value)
        .is_some_and(|previous| previous != value)
    {
        return None;
    }
    Some(())
}
