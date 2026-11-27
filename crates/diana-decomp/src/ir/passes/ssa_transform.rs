//! SSA (Static Single Assignment) Transformation Pass.
//!
//! This module implements the standard SSA construction algorithm:
//! 1. Place PHI nodes at dominance frontiers
//! 2. Rename variables with unique SSA versions
//!
//! ## Algorithm
//!
//! The algorithm follows the approach from "Efficiently Computing Static Single
//! Assignment Form and the Control Dependence Graph" by Cytron et al.
//!
//! ### PHI Placement
//!
//! For each variable that is defined in the CFG:
//! 1. Find all blocks where it's defined
//! 2. Compute the iterated dominance frontier of these blocks
//! 3. Insert PHI nodes at the frontier blocks
//!
//! ### Variable Renaming
//!
//! Walk the dominator tree in pre-order:
//! 1. For each instruction, rename uses to the current version
//! 2. For definitions, create a new version
//! 3. Update PHI operands in successor blocks
//! 4. Visit dominator tree children

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{
    analysis::{DominanceError, DominanceFrontier, DominatorTree, RegisterLiveness},
    arg::{InsnArg, RegisterArg},
    block::BlockId,
    cfg::CFG,
    insn::{InsnNode, InsnType},
    passes::{Pass, PassResult},
    ty::ArgType,
    InstructionTree, InstructionTreeError, InstructionVisitor,
};

/// SSA transformation pass.
///
/// Converts a CFG into SSA form by:
/// 1. Inserting PHI nodes at join points
/// 2. Renaming registers to unique SSA versions
#[derive(Debug, Default)]
pub struct SSATransform;

#[derive(Debug, Clone)]
pub enum SsaTransformError {
    Dominance(DominanceError),
    InstructionTree(InstructionTreeError),
    MissingExceptionSource(BlockId),
    InvalidExceptionBoundary {
        block: BlockId,
        throwing: Vec<usize>,
    },
    MissingPhiPlacement(BlockId),
    MissingPhiTopology(BlockId),
    MissingThrowingState(BlockId),
    UncoveredInput(u32),
    MissingBlock(BlockId),
    MissingPhiResult(BlockId),
    UnbalancedStack(u32),
    MissingTypeDomain {
        register: u32,
        version: u32,
    },
    MissingDefinition {
        register: u32,
        offset: u32,
    },
    MissingPhiSuccessor(BlockId),
    MissingPhiEdge {
        source: BlockId,
        target: BlockId,
        kind: crate::ir::EdgeKind,
    },
    MissingPhiDefinition {
        source: BlockId,
        target: BlockId,
        register: u32,
    },
    MissingPhiSlot {
        block: BlockId,
        index: usize,
    },
}

impl std::fmt::Display for SsaTransformError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dominance(error) => error.fmt(formatter),
            Self::InstructionTree(error) => error.fmt(formatter),
            Self::MissingExceptionSource(block) => {
                write!(formatter, "missing exception source {block}")
            }
            Self::InvalidExceptionBoundary { block, throwing } => write!(
                formatter,
                "exception source {block} must contain one terminal throwing instruction, found {throwing:?}"
            ),
            Self::MissingPhiPlacement(block) => write!(formatter, "missing Phi block {block}"),
            Self::MissingPhiTopology(block) => {
                write!(
                    formatter,
                    "missing incoming-edge facts for Phi block {block}"
                )
            }
            Self::MissingThrowingState(block) => {
                write!(formatter, "exception source {block} has no throwing state")
            }
            Self::UncoveredInput(register) => write!(
                formatter,
                "input register v{register} is not covered by the method descriptor"
            ),
            Self::MissingBlock(block) => write!(formatter, "missing SSA block {block}"),
            Self::MissingPhiResult(block) => write!(formatter, "Phi in {block} has no result"),
            Self::UnbalancedStack(register) => {
                write!(formatter, "SSA stack for v{register} is unbalanced")
            }
            Self::MissingTypeDomain { register, version } => {
                write!(formatter, "v{register}_{version} has no type domain")
            }
            Self::MissingDefinition { register, offset } => write!(
                formatter,
                "use of v{register} has no reaching definition at offset {offset}"
            ),
            Self::MissingPhiSuccessor(block) => {
                write!(formatter, "missing Phi successor {block}")
            }
            Self::MissingPhiEdge {
                source,
                target,
                kind,
            } => write!(
                formatter,
                "Phi in {target} has no {kind:?} edge from {source}"
            ),
            Self::MissingPhiDefinition {
                source,
                target,
                register,
            } => write!(
                formatter,
                "Phi in {target} has no reaching definition for v{register} from {source}"
            ),
            Self::MissingPhiSlot { block, index } => {
                write!(formatter, "Phi in {block} has no operand slot {index}")
            }
        }
    }
}

impl std::error::Error for SsaTransformError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Dominance(error) => Some(error),
            Self::InstructionTree(error) => Some(error),
            _ => None,
        }
    }
}

impl Pass for SSATransform {
    type Error = SsaTransformError;

    fn name(&self) -> &'static str {
        "ssa_transform"
    }

    fn run(&mut self, cfg: &mut CFG) -> Result<PassResult, Self::Error> {
        Self::construct(cfg).map(|(change, _)| change)
    }
}

impl SSATransform {
    pub(super) fn construct(
        cfg: &mut CFG,
    ) -> Result<(PassResult, DominatorTree), SsaTransformError> {
        Self::model_refining_definitions(cfg);
        let exceptional_definitions = Self::validate_exception_boundaries(cfg)?;

        // Step 1: Compute dominance information
        let dom_tree = DominatorTree::compute(cfg).map_err(SsaTransformError::Dominance)?;
        let dom_frontier =
            DominanceFrontier::compute(cfg, &dom_tree).map_err(SsaTransformError::Dominance)?;

        if !PhiPlacement::has_registers(cfg) {
            return Ok((PassResult::Unchanged, dom_tree));
        }
        if !exceptional_definitions.is_empty() || !dom_frontier.is_empty() {
            let liveness = RegisterLiveness::analyze(cfg);
            let placements =
                PhiPlacement::compute(&dom_frontier, &liveness, &exceptional_definitions);
            PhiPlacement::insert(cfg, &placements)?;
        }

        // Step 4: Rename variables
        let mut renamer = VariableRenamer::default();
        renamer.rename(cfg, &dom_tree)?;

        Ok((PassResult::Changed, dom_tree))
    }
    /// A throwing result is visible only on the normal edge. Keep precisely
    /// the register and handler pairs needed for pruned phi placement.
    fn validate_exception_boundaries(cfg: &CFG) -> Result<Vec<(u32, BlockId)>, SsaTransformError> {
        let mut exceptional_definitions = Vec::new();
        for (&block, body) in &cfg.blocks {
            let mut handlers = cfg
                .successors_with_kind(block)
                .iter()
                .filter_map(|&(target, edge)| edge.is_exception().then_some(target))
                .peekable();
            if handlers.peek().is_none() {
                continue;
            }
            let Some((throwing, _)) = body.insns.split_last().filter(|(last, prefix)| {
                last.can_throw() && !prefix.iter().any(|instruction| instruction.can_throw())
            }) else {
                let throwing = body
                    .insns
                    .iter()
                    .enumerate()
                    .filter_map(|(index, instruction)| instruction.can_throw().then_some(index))
                    .collect();
                return Err(SsaTransformError::InvalidExceptionBoundary { block, throwing });
            };
            if let Some(result) = &throwing.result {
                exceptional_definitions.extend(
                    handlers
                        .filter(|target| cfg.blocks.contains_key(target))
                        .map(|target| (result.reg_num, target)),
                );
            }
        }
        Ok(exceptional_definitions)
    }

    fn model_refining_definitions(cfg: &mut CFG) {
        for block in cfg.blocks.values_mut() {
            for insn in &mut block.insns {
                if insn.insn_type != InsnType::CheckCast || insn.result.is_some() {
                    continue;
                }
                let Some(source) = insn.args.first().and_then(InsnArg::as_register) else {
                    continue;
                };
                let mut result = source.clone();
                if let Some(target_type) = insn.conversion_type() {
                    result.ty = target_type.clone();
                }
                insn.result = Some(result);
            }
        }
    }
}

struct PhiPlacement;

impl PhiPlacement {
    fn has_registers(cfg: &CFG) -> bool {
        if cfg.ins.min(cfg.registers) != 0 {
            return true;
        }
        struct RegisterUse(bool);
        impl InstructionVisitor for RegisterUse {
            fn visit_register(&mut self, _: &RegisterArg) {
                self.0 = true;
            }
        }
        cfg.blocks
            .values()
            .flat_map(|block| &block.insns)
            .any(|instruction| {
                if instruction.result.is_some() {
                    return true;
                }
                let mut used = RegisterUse(false);
                InstructionTree::visit_args(instruction, &mut used);
                used.0
            })
    }

    /// Determine where to place PHI nodes for each variable.
    /// Returns a map from block ID to the set of registers that need PHI nodes.
    /// Uses BTreeMap to ensure deterministic iteration order.
    fn compute(
        dom_frontier: &DominanceFrontier,
        liveness: &RegisterLiveness,
        exceptional_sites: &[(u32, BlockId)],
    ) -> BTreeMap<BlockId, BTreeSet<u32>> {
        let mut phi_placements: BTreeMap<BlockId, BTreeSet<u32>> = BTreeMap::new();
        let mut exceptional_definitions = BTreeMap::<u32, BTreeSet<BlockId>>::new();
        for &(register, block) in exceptional_sites {
            if liveness.is_live_in(block, register) {
                exceptional_definitions
                    .entry(register)
                    .or_default()
                    .insert(block);
            }
        }

        // Every exceptional clobber is also an instruction definition. Inputs
        // and read-only registers have no definition frontier to propagate.
        let mut pending = Vec::new();
        for (register, definitions) in liveness.definitions() {
            // Definitions are already distinct and sorted. Every other block
            // enters the worklist only when its first phi is inserted.
            pending.extend_from_slice(definitions);
            if let Some(edges) = exceptional_definitions.get(&register) {
                for &block in edges {
                    phi_placements.entry(block).or_default().insert(register);
                    if definitions.binary_search(&block).is_err() {
                        pending.push(block);
                    }
                }
            }
            while let Some(block) = pending.pop() {
                for join in dom_frontier.frontier(block) {
                    if liveness.is_live_in(join, register)
                        && phi_placements.entry(join).or_default().insert(register)
                        && definitions.binary_search(&join).is_err()
                    {
                        pending.push(join);
                    }
                }
            }
        }

        phi_placements
    }

    /// Insert PHI nodes into the CFG.
    /// Uses BTreeMap for deterministic iteration order.
    fn insert(
        cfg: &mut CFG,
        phi_placements: &BTreeMap<BlockId, BTreeSet<u32>>,
    ) -> Result<(), SsaTransformError> {
        for (&block_id, vars) in phi_placements {
            let edges = cfg.incoming_edges(block_id);
            let block = cfg
                .blocks
                .get_mut(&block_id)
                .ok_or(SsaTransformError::MissingPhiPlacement(block_id))?;
            // Create PHI nodes at the beginning of the block
            let mut phi_insns = Vec::with_capacity(vars.len() + block.insns.len());
            for &var in vars {
                let mut phi = InsnNode::new(InsnType::Phi, edges.len());
                phi.set_result(RegisterArg::new(var, ArgType::unknown()));
                phi.payload.phi_edges = edges.clone();

                // The values are assigned while renaming each predecessor.
                // A register without an SSA version is an explicit unresolved
                // edge value and will be rejected by SSA validation.
                for _ in &edges {
                    phi.add_arg(InsnArg::reg(var, ArgType::unknown()));
                }

                phi_insns.push(phi);
            }

            // Insert PHI nodes at the beginning
            let existing_insns = std::mem::take(&mut block.insns);
            block.insns = phi_insns;
            block.insns.extend(existing_insns);
        }
        Ok(())
    }
}

/// A definition and its type have the same lifetime in the dominator walk.
/// Exiting a block restores its previous bindings, but keeps version counters.
#[derive(Default)]
struct VariableRenamer {
    registers: foldhash::HashMap<u32, RegisterState>,
    shadowed: Vec<(u32, Option<TypedVersion>)>,
    missing_use: Option<u32>,
}

#[derive(Default)]
struct RegisterState {
    next: u32,
    active: Option<TypedVersion>,
}

struct TypedVersion {
    version: u32,
    ty: ArgType,
}

enum RenameTask {
    Enter(BlockId),
    Exit(usize),
}

impl VariableRenamer {
    fn define(&mut self, reg: u32, ty: &ArgType) -> u32 {
        let state = self.registers.entry(reg).or_default();
        let version = state.next;
        state.next += 1;
        let previous = state.active.replace(TypedVersion {
            version,
            ty: ty.clone(),
        });
        self.shadowed.push((reg, previous));
        version
    }

    fn active(&self, reg: u32) -> Option<&TypedVersion> {
        self.registers.get(&reg)?.active.as_ref()
    }

    fn rename(&mut self, cfg: &mut CFG, dom_tree: &DominatorTree) -> Result<(), SsaTransformError> {
        self.seed_input_registers(cfg)?;
        let mut tasks = vec![RenameTask::Enter(cfg.entry)];
        while let Some(task) = tasks.pop() {
            match task {
                RenameTask::Enter(block) => {
                    let start = self.shadowed.len();
                    let last_result = self.rename_definitions_and_uses(cfg, block)?;
                    let edges = cfg
                        .successors_with_kind(block)
                        .iter()
                        .copied()
                        .collect::<smallvec::SmallVec<[_; 2]>>();
                    for (successor, kind) in edges {
                        // Protected blocks have exactly one throwing instruction,
                        // at the end. Only its result differs on an exceptional
                        // edge. All other bindings survive the throw unchanged.
                        let previous = if kind == crate::ir::EdgeKind::Exception {
                            last_result
                        } else {
                            None
                        };
                        self.fill_phi_operands(cfg, block, successor, kind, previous)?;
                    }
                    tasks.push(RenameTask::Exit(start));
                    tasks.extend(dom_tree.children(block).rev().map(RenameTask::Enter));
                }
                RenameTask::Exit(start) => {
                    for (reg, previous) in self.shadowed.drain(start..).rev() {
                        self.registers
                            .get_mut(&reg)
                            .ok_or(SsaTransformError::UnbalancedStack(reg))?
                            .active = previous;
                    }
                }
            }
        }
        Ok(())
    }

    /// Map wide parameters to both physical input slots before visiting entry.
    fn seed_input_registers(&mut self, cfg: &CFG) -> Result<(), SsaTransformError> {
        let method = cfg.method();
        let receiver = (!method.is_static()).then_some(method.owner());
        let mut types = receiver.into_iter().chain(
            method
                .descriptor()
                .parameters
                .iter()
                .flat_map(|ty| std::iter::repeat_n(ty, 1 + usize::from(ty.is_wide()))),
        );
        for reg in cfg.registers.saturating_sub(cfg.ins)..cfg.registers {
            self.define(
                reg,
                types.next().ok_or(SsaTransformError::UncoveredInput(reg))?,
            );
        }
        self.shadowed.clear();
        Ok(())
    }

    fn rename_definitions_and_uses(
        &mut self,
        cfg: &mut CFG,
        block_id: BlockId,
    ) -> Result<Option<u32>, SsaTransformError> {
        let block = cfg
            .blocks
            .get_mut(&block_id)
            .ok_or(SsaTransformError::MissingBlock(block_id))?;
        for insn in &mut block.insns {
            if insn.insn_type == InsnType::Phi {
                if insn.result.is_none() {
                    return Err(SsaTransformError::MissingPhiResult(block_id));
                }
            } else {
                self.rename_insn_uses(insn)?;
            }
            if let Some(result) = &mut insn.result {
                result.ssa_version = Some(self.define(result.reg_num, &result.ty));
            }
        }
        Ok(block
            .insns
            .last()
            .and_then(|insn| insn.result.as_ref())
            .map(|result| result.reg_num))
    }

    fn rename_insn_uses(&mut self, insn: &mut InsnNode) -> Result<(), SsaTransformError> {
        self.missing_use = None;
        InstructionTree::edit_register_uses(insn, |register| self.rename_use(register));
        if let Some(register) = self.missing_use.take() {
            return Err(SsaTransformError::MissingDefinition {
                register,
                offset: insn.offset,
            });
        }
        Ok(())
    }

    fn rename_use(&mut self, register: &mut RegisterArg) {
        if let Some(value) = self.active(register.reg_num) {
            register.ssa_version = Some(value.version);
            if !register.ty.is_known() && register.ty != value.ty {
                register.ty = value.ty.clone();
            }
        } else {
            self.missing_use.get_or_insert(register.reg_num);
        }
    }
}

impl VariableRenamer {
    /// Fill in PHI operands for a successor block.
    fn fill_phi_operands(
        &mut self,
        cfg: &mut CFG,
        from: BlockId,
        to: BlockId,
        edge_kind: crate::ir::EdgeKind,
        previous_result: Option<u32>,
    ) -> Result<(), SsaTransformError> {
        // Update PHI nodes
        let block = cfg
            .blocks
            .get_mut(&to)
            .ok_or(SsaTransformError::MissingPhiSuccessor(to))?;
        for insn in &mut block.insns {
            if insn.insn_type != InsnType::Phi {
                break; // PHI nodes are at the beginning
            }

            if let Some(result) = &insn.result {
                let pred_index = insn
                    .payload
                    .phi_edges
                    .iter()
                    .position(|edge| *edge == (from, edge_kind))
                    .ok_or(SsaTransformError::MissingPhiEdge {
                        source: from,
                        target: to,
                        kind: edge_kind,
                    })?;
                let reg = result.reg_num;

                // Get current version for this predecessor
                let value = if previous_result == Some(reg) {
                    let (saved_reg, previous) = self
                        .shadowed
                        .last()
                        .ok_or(SsaTransformError::MissingThrowingState(from))?;
                    if *saved_reg != reg {
                        return Err(SsaTransformError::MissingThrowingState(from));
                    }
                    previous.as_ref()
                } else {
                    self.active(reg)
                }
                .ok_or(SsaTransformError::MissingPhiDefinition {
                    source: from,
                    target: to,
                    register: reg,
                })?;
                let slot =
                    insn.args
                        .get_mut(pred_index)
                        .ok_or(SsaTransformError::MissingPhiSlot {
                            block: to,
                            index: pred_index,
                        })?;
                let mut source_reg = RegisterArg::new(reg, value.ty.clone());
                source_reg.ssa_version = Some(value.version);
                *slot = InsnArg::Reg(source_reg);
            } else {
                return Err(SsaTransformError::MissingPhiResult(to));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{block::Block, cfg::EdgeKind, MethodContext, MethodDescriptor};

    fn make_diamond_cfg() -> CFG {
        // Diamond CFG:
        //     0: v0 = 1
        //    / \
        //   1   2
        //   v0=2 v0=3
        //    \ /
        //     3: use v0
        let mut cfg = CFG::new("test");

        let mut block0 = Block::new(0u32);
        let mut insn = InsnNode::const_val(RegisterArg::new(0, ArgType::INT), 1, ArgType::INT);
        insn.set_offset(0);
        block0.push(insn);
        cfg.add_block(block0);

        let mut block1 = Block::new(1u32);
        let mut insn = InsnNode::const_val(RegisterArg::new(0, ArgType::INT), 2, ArgType::INT);
        insn.set_offset(0);
        block1.push(insn);
        cfg.add_block(block1);

        let mut block2 = Block::new(2u32);
        let mut insn = InsnNode::const_val(RegisterArg::new(0, ArgType::INT), 3, ArgType::INT);
        insn.set_offset(0);
        block2.push(insn);
        cfg.add_block(block2);

        let mut block3 = Block::new(3u32);
        let mut insn = InsnNode::new(InsnType::Return, 1);
        insn.add_arg(InsnArg::reg(0, ArgType::INT));
        insn.set_offset(0);
        block3.push(insn);
        cfg.add_block(block3);

        cfg.add_edge(BlockId(0), BlockId(1), EdgeKind::Normal);
        cfg.add_edge(BlockId(0), BlockId(2), EdgeKind::Normal);
        cfg.add_edge(BlockId(1), BlockId(3), EdgeKind::Normal);
        cfg.add_edge(BlockId(2), BlockId(3), EdgeKind::Normal);

        cfg.entry = BlockId(0);

        cfg
    }

    #[test]
    fn test_ssa_transform_diamond() {
        let mut cfg = make_diamond_cfg();

        let mut pass = SSATransform;
        let result = pass.run(&mut cfg);
        assert!(result.is_ok());

        // Block 3 should have a PHI node for v0
        let block3 = cfg.block(BlockId(3)).unwrap();
        let phi_count = block3
            .insns
            .iter()
            .filter(|i| i.insn_type == InsnType::Phi)
            .count();
        assert_eq!(phi_count, 1, "Expected 1 PHI node in block 3");

        // The PHI should be for register 0
        let phi = block3.insns.first().unwrap();
        assert_eq!(phi.insn_type, InsnType::Phi);
        assert_eq!(phi.result.as_ref().unwrap().reg_num, 0);
    }

    #[test]
    fn test_ssa_versioning() {
        let mut cfg = make_diamond_cfg();

        let mut pass = SSATransform;
        pass.run(&mut cfg).unwrap();

        // Check that v0 has different SSA versions in different blocks
        let block0 = cfg.block(BlockId(0)).unwrap();
        let block1 = cfg.block(BlockId(1)).unwrap();

        // Block 0's v0 definition should have version 0
        let v0_in_block0 = &block0.insns[0];
        assert!(v0_in_block0.result.as_ref().unwrap().ssa_version.is_some());

        // Block 1's v0 definition should have a different version
        let v0_in_block1 = &block1.insns[0];
        assert!(v0_in_block1.result.as_ref().unwrap().ssa_version.is_some());
    }

    #[test]
    fn test_input_only_registers_get_ssa_versions() {
        let mut cfg = CFG::new("<init>");
        cfg.registers = 3;
        cfg.ins = 3;
        cfg.set_method(MethodContext::new(
            ArgType::object("ObjectTypeAdapter"),
            "<init>",
            MethodDescriptor {
                parameters: vec![ArgType::object("Gson"), ArgType::object("ToNumberStrategy")],
                return_type: ArgType::VOID,
            },
            false,
        ));

        let mut block = Block::new(0u32);
        block.push(InsnNode::iput(
            InsnArg::reg(1, ArgType::object("Gson")),
            InsnArg::reg(0, ArgType::object("ObjectTypeAdapter")),
            0,
        ));
        block.push(InsnNode::ret(None));
        cfg.add_block(block);
        cfg.entry = BlockId(0);

        let mut pass = SSATransform;
        pass.run(&mut cfg).unwrap();

        let block = cfg.block(BlockId(0)).unwrap();
        let insn = &block.insns[0];
        assert_eq!(
            insn.args[0].as_register().unwrap().ssa_version,
            Some(0),
            "first declared parameter should be seeded as an SSA input"
        );
        assert_eq!(
            insn.args[1].as_register().unwrap().ssa_version,
            Some(0),
            "implicit this should be seeded as an SSA input"
        );
    }

    #[test]
    fn exception_edge_observes_state_before_throwing_result() {
        let context = ArgType::object("android/content/Context");
        let bundle = ArgType::object("android/os/Bundle");
        let mut cfg = CFG::new("exception_result_state");
        cfg.registers = 1;
        cfg.ins = 1;
        cfg.set_method(MethodContext::new(
            ArgType::object("Owner"),
            "call",
            MethodDescriptor {
                parameters: vec![context.clone()],
                return_type: context.clone(),
            },
            true,
        ));

        let mut throwing = Block::new(0u32);
        let mut invoke = InsnNode::new(InsnType::Invoke, 0);
        invoke.set_result(RegisterArg::new(0, bundle.clone()));
        throwing.push(invoke);
        cfg.add_block(throwing);

        let mut handler = Block::new(1u32);
        handler.push(InsnNode::ret(Some(InsnArg::reg(0, context))));
        cfg.add_block(handler);

        let mut normal = Block::new(2u32);
        normal.push(InsnNode::ret(Some(InsnArg::reg(0, bundle))));
        cfg.add_block(normal);

        cfg.add_edge(BlockId(0), BlockId(1), EdgeKind::Exception);
        cfg.add_edge(BlockId(0), BlockId(2), EdgeKind::Normal);
        cfg.entry = BlockId(0);

        SSATransform.run(&mut cfg).unwrap();

        let handler = cfg.block(BlockId(1)).unwrap();
        let phi = handler
            .insns
            .iter()
            .find(|instruction| instruction.insn_type == InsnType::Phi)
            .expect("handler edge-state phi");
        assert_eq!(phi.args[0].as_register().unwrap().ssa_version, Some(0));
        let invoke_result = cfg.block(BlockId(0)).unwrap().insns[0]
            .result
            .as_ref()
            .unwrap()
            .ssa_version;
        assert_ne!(
            phi.args[0].as_register().unwrap().ssa_version,
            invoke_result
        );
    }

    #[test]
    fn exceptional_edge_definitions_reach_handler_join() {
        let prior = ArgType::object("Prior");
        let result = ArgType::object("Result");
        let mut cfg = CFG::new("exception_result_join");
        cfg.registers = 1;
        cfg.ins = 1;
        cfg.set_method(MethodContext::new(
            ArgType::object("Owner"),
            "call",
            MethodDescriptor {
                parameters: vec![prior.clone()],
                return_type: prior.clone(),
            },
            true,
        ));

        let mut throwing = Block::new(0u32);
        let mut invoke = InsnNode::new(InsnType::Invoke, 0);
        invoke.set_result(RegisterArg::new(0, result.clone()));
        throwing.push(invoke);
        cfg.add_block(throwing);

        cfg.add_block(Block::new(1u32));
        cfg.add_block(Block::new(2u32));

        let mut join = Block::new(3u32);
        join.push(InsnNode::ret(Some(InsnArg::reg(0, prior))));
        cfg.add_block(join);

        let mut normal = Block::new(4u32);
        normal.push(InsnNode::ret(Some(InsnArg::reg(0, result))));
        cfg.add_block(normal);

        cfg.add_edge(BlockId(0), BlockId(1), EdgeKind::Exception);
        cfg.add_edge(BlockId(0), BlockId(2), EdgeKind::Exception);
        cfg.add_edge(BlockId(0), BlockId(4), EdgeKind::Normal);
        cfg.add_edge(BlockId(1), BlockId(3), EdgeKind::Normal);
        cfg.add_edge(BlockId(2), BlockId(3), EdgeKind::Normal);
        cfg.entry = BlockId(0);

        SSATransform.run(&mut cfg).unwrap();

        let invoke_result = cfg.block(BlockId(0)).unwrap().insns[0]
            .result
            .as_ref()
            .unwrap()
            .ssa_version;
        let join = cfg.block(BlockId(3)).unwrap();
        let phi = join
            .insns
            .iter()
            .find(|instruction| instruction.insn_type == InsnType::Phi)
            .expect("handler join phi");
        assert!(phi
            .args
            .iter()
            .all(|argument| { argument.as_register().unwrap().ssa_version != invoke_result }));
        assert_eq!(
            join.insns.last().unwrap().args[0]
                .as_register()
                .unwrap()
                .ssa_version,
            phi.result.as_ref().unwrap().ssa_version
        );
    }

    #[test]
    fn throwing_result_preserves_prior_phi_on_exception_edge() {
        let mut cfg = CFG::new("exception_result_after_join");
        cfg.registers = 1;
        cfg.ins = 0;
        cfg.set_method(MethodContext::new(
            ArgType::object("Owner"),
            "call",
            MethodDescriptor {
                parameters: Vec::new(),
                return_type: ArgType::INT,
            },
            true,
        ));

        cfg.add_block(Block::new(0u32));
        let mut left = Block::new(1u32);
        left.push(InsnNode::const_value(RegisterArg::new(0, ArgType::INT), 1));
        cfg.add_block(left);
        let mut right = Block::new(2u32);
        right.push(InsnNode::const_value(RegisterArg::new(0, ArgType::INT), 2));
        cfg.add_block(right);

        let mut throwing = Block::new(3u32);
        let mut invoke = InsnNode::new(InsnType::Invoke, 0);
        invoke.set_result(RegisterArg::new(0, ArgType::INT));
        throwing.push(invoke);
        cfg.add_block(throwing);
        let mut handler = Block::new(4u32);
        handler.push(InsnNode::ret(Some(InsnArg::reg(0, ArgType::INT))));
        cfg.add_block(handler);
        let mut normal = Block::new(5u32);
        normal.push(InsnNode::ret(Some(InsnArg::reg(0, ArgType::INT))));
        cfg.add_block(normal);

        cfg.add_edge(BlockId(0), BlockId(1), EdgeKind::Normal);
        cfg.add_edge(BlockId(0), BlockId(2), EdgeKind::Normal);
        cfg.add_edge(BlockId(1), BlockId(3), EdgeKind::Normal);
        cfg.add_edge(BlockId(2), BlockId(3), EdgeKind::Normal);
        cfg.add_edge(BlockId(3), BlockId(4), EdgeKind::Exception);
        cfg.add_edge(BlockId(3), BlockId(5), EdgeKind::Normal);
        cfg.entry = BlockId(0);

        SSATransform.run(&mut cfg).unwrap();

        let throwing = cfg.block(BlockId(3)).unwrap();
        let prior = throwing
            .insns
            .iter()
            .find(|instruction| instruction.insn_type == InsnType::Phi)
            .and_then(|phi| phi.result.as_ref())
            .and_then(|result| result.ssa_version)
            .expect("joined value before throwing invoke");
        let invoke_result = throwing
            .insns
            .last()
            .and_then(|invoke| invoke.result.as_ref())
            .and_then(|result| result.ssa_version)
            .expect("normal invoke result");
        let exceptional = cfg
            .block(BlockId(4))
            .unwrap()
            .insns
            .iter()
            .find(|instruction| instruction.insn_type == InsnType::Phi)
            .and_then(|phi| phi.args.first())
            .and_then(InsnArg::as_register)
            .and_then(|argument| argument.ssa_version)
            .expect("exception edge value");

        assert_eq!(exceptional, prior);
        assert_ne!(exceptional, invoke_result);
    }
}
