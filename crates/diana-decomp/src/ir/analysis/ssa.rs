//! Canonical SSA value graph and source-variable congruence classes.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{
    ArgType, BlockId, InsnArg, InsnNode, InsnType, InstructionTree, InstructionVisitor,
    RegisterArg, CFG,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SsaVar {
    pub reg_num: u32,
    pub version: u32,
}

impl SsaVar {
    pub fn new(reg_num: u32, version: u32) -> Self {
        Self { reg_num, version }
    }

    pub fn from_reg(register: &RegisterArg) -> Option<Self> {
        Some(Self::new(register.reg_num, register.ssa_version?))
    }

    pub fn apply_to(self, register: &mut RegisterArg) {
        register.reg_num = self.reg_num;
        register.ssa_version = Some(self.version);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InsnPosition {
    pub block: BlockId,
    pub index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UsePosition {
    pub instruction: InsnPosition,
    pub argument: usize,
    phi: Option<SsaVar>,
}

impl UsePosition {
    pub(super) fn phi(instruction: InsnPosition, argument: usize, result: SsaVar) -> Self {
        Self {
            instruction,
            argument,
            phi: Some(result),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SsaUseSite<'a> {
    Instruction(&'a InsnNode),
    Phi(&'a PhiMerge),
}

#[derive(Debug, Clone)]
pub struct SsaValue {
    pub variable: SsaVar,
    pub definition: Option<InsnPosition>,
    pub uses: Vec<UsePosition>,
    pub observed_types: BTreeSet<ArgType>,
}

impl SsaValue {
    fn new(variable: SsaVar) -> Self {
        Self {
            variable,
            definition: None,
            uses: Vec::new(),
            observed_types: BTreeSet::new(),
        }
    }
    fn observe_type(&mut self, ty: &ArgType) {
        if !self.observed_types.contains(ty) {
            self.observed_types.insert(ty.clone());
        }
    }
}

#[derive(Debug, Clone)]
pub struct PhiInput {
    pub predecessor: BlockId,
    pub edge_kind: crate::ir::EdgeKind,
    pub value: SsaVar,
}

#[derive(Debug, Clone)]
pub struct PhiMerge {
    pub block: BlockId,
    pub instruction: crate::ir::InstructionId,
    pub result: SsaVar,
    pub inputs: Vec<PhiInput>,
}

#[derive(Debug, Clone, Copy)]
pub struct ValueCopy {
    pub result: SsaVar,
    pub source: SsaVar,
}

#[derive(Debug, Clone, Default)]
pub struct SsaValueGraph {
    values: Box<[SsaValue]>,
    registers: RegisterRows,
    phis: Vec<PhiMerge>,
    phi_indices: foldhash::HashMap<SsaVar, usize>,
    copies: Vec<ValueCopy>,
}

#[derive(Debug, Clone)]
pub enum SsaInvariantError {
    DefinitionWithoutIdentity(InsnPosition),
    MultipleDefinitions {
        value: SsaVar,
        first: InsnPosition,
        second: InsnPosition,
    },
    UseWithoutIdentity(InsnPosition),
    PhiWithoutResult(BlockId),
    PhiArity {
        block: BlockId,
        values: usize,
        edges: usize,
    },
    InvalidPhiInput {
        predecessor: BlockId,
        block: BlockId,
    },
    MissingPhiPredecessor(BlockId),
    PhiTopology {
        block: BlockId,
        actual: Vec<(BlockId, crate::ir::EdgeKind)>,
        expected: Vec<(BlockId, crate::ir::EdgeKind)>,
    },
    Dominance(super::DominanceError),
    UndefinedValue(SsaVar),
    NonDominatingDefinition {
        value: SsaVar,
        definition: InsnPosition,
        usage: InsnPosition,
    },
}

impl std::fmt::Display for SsaInvariantError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DefinitionWithoutIdentity(position) => {
                write!(formatter, "definition without SSA identity at {position:?}")
            }
            Self::MultipleDefinitions {
                value,
                first,
                second,
            } => write!(
                formatter,
                "{value:?} is defined at both {first:?} and {second:?}"
            ),
            Self::UseWithoutIdentity(position) => {
                write!(formatter, "use without SSA identity at {position:?}")
            }
            Self::PhiWithoutResult(block) => write!(formatter, "Phi without result in {block}"),
            Self::PhiArity {
                block,
                values,
                edges,
            } => write!(
                formatter,
                "Phi in {block} has {values} values for {edges} edges"
            ),
            Self::InvalidPhiInput { predecessor, block } => write!(
                formatter,
                "unresolved Phi input on edge {predecessor} -> {block}"
            ),
            Self::MissingPhiPredecessor(block) => {
                write!(formatter, "Phi predecessor block {block} is missing")
            }
            Self::PhiTopology {
                block,
                actual,
                expected,
            } => write!(
                formatter,
                "Phi predecessors in {block} are {actual:?}, expected {expected:?}"
            ),
            Self::Dominance(error) => error.fmt(formatter),
            Self::UndefinedValue(value) => {
                write!(formatter, "{value:?} is used without a definition")
            }
            Self::NonDominatingDefinition {
                value,
                definition,
                usage,
            } => write!(
                formatter,
                "definition of {value:?} at {definition:?} does not dominate use at {usage:?}"
            ),
        }
    }
}

impl std::error::Error for SsaInvariantError {}

impl SsaValueGraph {
    pub fn build(cfg: &CFG) -> Result<Self, SsaInvariantError> {
        Self::build_with_dominators(cfg, None).map(|(graph, _)| graph)
    }

    pub(crate) fn build_with_dominators(
        cfg: &CFG,
        dominators: Option<super::DominatorTree>,
    ) -> Result<(Self, super::DominatorTree), SsaInvariantError> {
        let mut builder = SsaValueBuilder::default();
        for (&block, body) in &cfg.blocks {
            for (index, instruction) in body.insns.iter().enumerate() {
                let position = InsnPosition { block, index };
                if let Some(result) = &instruction.result {
                    let variable = SsaVar::from_reg(result)
                        .ok_or(SsaInvariantError::DefinitionWithoutIdentity(position))?;
                    let value = builder.value_mut(variable);
                    if let Some(first) = value.definition.replace(position) {
                        return Err(SsaInvariantError::MultipleDefinitions {
                            value: variable,
                            first,
                            second: position,
                        });
                    }
                    value.observe_type(&result.ty);
                }
                let arguments: &[InsnArg] = if instruction.insn_type == InsnType::Phi {
                    &[]
                } else if instruction.insn_type == InsnType::Constructor {
                    instruction.args.get(1..).unwrap_or_default()
                } else {
                    &instruction.args
                };
                for (argument, value) in arguments.iter().enumerate() {
                    builder.collect_use(
                        value,
                        UsePosition {
                            instruction: position,
                            argument,
                            phi: None,
                        },
                    )?;
                }
                if let Some(target) = &instruction.payload.compound_target {
                    builder.collect_use(
                        target,
                        UsePosition {
                            instruction: position,
                            argument: instruction.args.len(),
                            phi: None,
                        },
                    )?;
                }
                if instruction.insn_type == InsnType::Phi {
                    builder.collect_phi(cfg, block, instruction)?;
                } else if instruction.insn_type == InsnType::Move {
                    if let (Some(result), Some(source)) = (
                        instruction.result.as_ref().and_then(SsaVar::from_reg),
                        instruction
                            .args
                            .first()
                            .and_then(InsnArg::as_register)
                            .and_then(SsaVar::from_reg),
                    ) {
                        builder.graph.copies.push(ValueCopy { result, source });
                    }
                }
            }
        }
        let graph = builder.finish();
        graph.validate_phi_edges(cfg)?;
        let dominators = match dominators {
            Some(tree) => tree,
            None => super::DominatorTree::compute(cfg).map_err(SsaInvariantError::Dominance)?,
        };
        graph.validate_dominance(cfg, &dominators)?;
        Ok((graph, dominators))
    }

    // Freezing instruction identities only numbers the existing instructions;
    // Phi records and their source instructions share the original walk order.
    pub(crate) fn refresh_phi_identities(&mut self, cfg: &CFG) {
        if self.phis.is_empty() {
            return;
        }
        let ids = cfg
            .blocks
            .values()
            .flat_map(|block| &block.insns)
            .filter(|instruction| instruction.insn_type == InsnType::Phi)
            .map(|instruction| instruction.id);
        for (phi, id) in self.phis.iter_mut().zip(ids) {
            phi.instruction = id;
        }
    }

    pub fn values(&self) -> impl Iterator<Item = &SsaValue> {
        self.values.iter()
    }

    pub fn value(&self, variable: SsaVar) -> Option<&SsaValue> {
        self.registers
            .find(&self.values, variable, |value| value.variable)
    }

    pub fn use_site<'a>(
        &'a self,
        cfg: &'a CFG,
        value: SsaVar,
        usage: UsePosition,
    ) -> Option<SsaUseSite<'a>> {
        if let Some(result) = usage.phi {
            let phi = self.phi(result)?;
            let input = phi.inputs.get(usage.argument)?;
            return (input.value == value && input.predecessor == usage.instruction.block)
                .then_some(SsaUseSite::Phi(phi));
        }
        cfg.block(usage.instruction.block)
            .and_then(|block| block.insns.get(usage.instruction.index))
            .map(SsaUseSite::Instruction)
    }

    pub fn phis(&self) -> &[PhiMerge] {
        &self.phis
    }

    pub(crate) fn phi(&self, result: SsaVar) -> Option<&PhiMerge> {
        self.phis.get(*self.phi_indices.get(&result)?)
    }

    #[cfg(test)]
    pub(crate) fn with_phis_for_test(phis: Vec<PhiMerge>) -> Self {
        Self {
            phi_indices: phis
                .iter()
                .enumerate()
                .map(|(i, phi)| (phi.result, i))
                .collect(),
            phis,
            ..Self::default()
        }
    }

    pub fn copies(&self) -> &[ValueCopy] {
        &self.copies
    }

    pub fn copy_classes(&self) -> SsaClasses {
        let mut classes = SsaClasses::new(self.values.iter().map(|value| value.variable));
        for copy in &self.copies {
            classes.union(copy.result, copy.source);
        }
        classes
    }

    fn validate_phi_edges(&self, cfg: &CFG) -> Result<(), SsaInvariantError> {
        for phi in &self.phis {
            let expected = cfg.predecessors_with_kind(phi.block);
            let edges = phi
                .inputs
                .iter()
                .map(|input| (input.predecessor, input.edge_kind));
            if !edges.clone().eq(expected.iter().copied()) {
                return Err(SsaInvariantError::PhiTopology {
                    block: phi.block,
                    actual: edges.collect(),
                    expected: expected.to_vec(),
                });
            }
        }
        Ok(())
    }

    fn validate_dominance(
        &self,
        cfg: &CFG,
        dominators: &super::DominatorTree,
    ) -> Result<(), SsaInvariantError> {
        let first_input = cfg.registers.saturating_sub(cfg.ins);
        for value in &self.values {
            let Some(definition) = value.definition else {
                let method_input = value.variable.version == 0
                    && first_input <= value.variable.reg_num
                    && value.variable.reg_num < cfg.registers;
                if method_input {
                    continue;
                }
                return Err(SsaInvariantError::UndefinedValue(value.variable));
            };
            for usage in &value.uses {
                let dominated = if definition.block == usage.instruction.block {
                    definition.index < usage.instruction.index
                } else {
                    dominators.dominates(definition.block, usage.instruction.block)
                };
                if !dominated {
                    return Err(SsaInvariantError::NonDominatingDefinition {
                        value: value.variable,
                        definition,
                        usage: usage.instruction,
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Default)]
struct SsaValueBuilder {
    values: BTreeMap<SsaVar, SsaValue>,
    graph: SsaValueGraph,
}

impl SsaValueBuilder {
    fn finish(mut self) -> SsaValueGraph {
        let values: Box<[_]> = self.values.into_values().collect();
        self.graph.registers = RegisterRows::new(&values, |value| value.variable);
        self.graph.values = values;
        self.graph
    }

    fn value_mut(&mut self, variable: SsaVar) -> &mut SsaValue {
        self.values
            .entry(variable)
            .or_insert_with(|| SsaValue::new(variable))
    }

    fn collect_use(
        &mut self,
        argument: &InsnArg,
        position: UsePosition,
    ) -> Result<(), SsaInvariantError> {
        let mut collector = UseCollector {
            graph: self,
            position,
            error: None,
        };
        InstructionTree::visit_arg(argument, &mut collector);
        collector.error.map_or(Ok(()), Err)
    }

    fn collect_phi(
        &mut self,
        cfg: &CFG,
        block: BlockId,
        instruction: &crate::ir::InsnNode,
    ) -> Result<(), SsaInvariantError> {
        let result = instruction
            .result
            .as_ref()
            .and_then(SsaVar::from_reg)
            .ok_or(SsaInvariantError::PhiWithoutResult(block))?;
        if instruction.payload.phi_edges.len() != instruction.args.len() {
            return Err(SsaInvariantError::PhiArity {
                block,
                values: instruction.args.len(),
                edges: instruction.payload.phi_edges.len(),
            });
        }
        let mut inputs = Vec::with_capacity(instruction.args.len());
        for (&(predecessor, edge_kind), argument) in
            instruction.payload.phi_edges.iter().zip(&instruction.args)
        {
            let value = argument
                .as_register()
                .and_then(SsaVar::from_reg)
                .ok_or(SsaInvariantError::InvalidPhiInput { predecessor, block })?;
            let predecessor_len = cfg
                .block(predecessor)
                .ok_or(SsaInvariantError::MissingPhiPredecessor(predecessor))?
                .insns
                .len();
            let position = UsePosition::phi(
                InsnPosition {
                    block: predecessor,
                    index: predecessor_len,
                },
                inputs.len(),
                result,
            );
            let input = self.value_mut(value);
            input.uses.push(position);
            if let Some(register) = argument.as_register() {
                input.observe_type(&register.ty);
            }
            inputs.push(PhiInput {
                predecessor,
                edge_kind,
                value,
            });
        }
        self.graph.phi_indices.insert(result, self.graph.phis.len());
        self.graph.phis.push(PhiMerge {
            block,
            instruction: instruction.id,
            result,
            inputs,
        });
        Ok(())
    }
}

/// Register offsets into an immutable table ordered by (register, version).
/// Sparse register identities keep the ordinary sorted lookup. Dense version
/// rows also support direct addressing without filling missing versions.
#[derive(Debug, Clone, Default)]
struct RegisterRows(Box<[usize]>);

impl RegisterRows {
    fn new<T>(values: &[T], key: impl Fn(&T) -> SsaVar) -> Self {
        let Some(last) = values.last() else {
            return Self::default();
        };
        let max = key(last).reg_num as usize;
        if values.len() < 8 || max >= values.len() {
            return Self::default();
        }
        let mut starts = Vec::with_capacity(max + 2);
        for (index, value) in values.iter().enumerate() {
            let register = key(value).reg_num as usize;
            if register >= starts.len() {
                starts.resize(register + 1, index);
            }
        }
        starts.push(values.len());
        Self(starts.into_boxed_slice())
    }

    fn find<'a, T>(
        &self,
        values: &'a [T],
        variable: SsaVar,
        key: impl Fn(&T) -> SsaVar,
    ) -> Option<&'a T> {
        let row = if self.0.is_empty() {
            values
        } else {
            let starts = self.0.get(variable.reg_num as usize..)?.get(..2)?;
            let row = &values[starts[0]..starts[1]];
            let direct = variable.version.checked_sub(key(row.first()?).version)? as usize;
            if let Some(value) = row.get(direct).filter(|value| key(value) == variable) {
                return Some(value);
            }
            row
        };
        row.binary_search_by_key(&variable, key)
            .ok()
            .map(|index| &row[index])
    }
}

#[derive(Debug, Clone)]
pub struct CodeVariables {
    registers: RegisterRows,
    assignments: Vec<(SsaVar, u32)>,
    count: u32,
}

impl CodeVariables {
    pub(super) fn from_singletons(values: &SsaValueGraph, inputs: &[SsaVar]) -> Self {
        // SSA values and declared inputs each arrive in register/version order.
        let mut values = values.values.iter().map(|value| value.variable).peekable();
        let mut inputs = inputs.iter().copied().peekable();
        let mut assignments = Vec::with_capacity(values.len());
        loop {
            let variable = match (values.peek(), inputs.peek()) {
                (Some(left), Some(right)) if left == right => {
                    inputs.next();
                    values.next()
                }
                (Some(left), Some(right)) if right < left => inputs.next(),
                (Some(_), _) => values.next(),
                (None, _) => inputs.next(),
            };
            let Some(variable) = variable else { break };
            assignments.push((variable, assignments.len() as u32));
        }
        Self::from_assignments(assignments.len() as u32, assignments)
    }

    pub(super) fn from_classes(mut classes: SsaClasses) -> Self {
        let variables = classes.values();
        let mut roots = foldhash::HashMap::default();
        let mut assignments = Vec::with_capacity(variables.len());
        for variable in variables {
            let root = classes.root(variable);
            let next = roots.len() as u32;
            let id = *roots.entry(root).or_insert(next);
            assignments.push((variable, id));
        }
        Self::from_assignments(roots.len() as u32, assignments)
    }

    fn from_assignments(count: u32, assignments: Vec<(SsaVar, u32)>) -> Self {
        let registers = RegisterRows::new(&assignments, |entry| entry.0);
        Self {
            registers,
            count,
            assignments,
        }
    }

    pub fn annotate(&self, cfg: &mut CFG) -> Result<(), crate::ir::InstructionTreeError> {
        for block in cfg.blocks.values_mut() {
            for instruction in &mut block.insns {
                if let Some(result) = &mut instruction.result {
                    result.code_var = self.class_of(result);
                }
                InstructionTree::edit_register_uses(instruction, |register| {
                    register.code_var = self.class_of(register);
                });
            }
        }
        self.bind_inputs(cfg);
        Ok(())
    }

    pub(super) fn next_id(&self) -> u32 {
        self.count
    }

    pub(super) fn variable_of(&self, value: SsaVar) -> Option<u32> {
        self.registers
            .find(&self.assignments, value, |entry| entry.0)
            .map(|entry| entry.1)
    }

    pub(super) fn assignments(&self) -> &[(SsaVar, u32)] {
        &self.assignments
    }

    fn class_of(&self, register: &RegisterArg) -> Option<u32> {
        SsaVar::from_reg(register).and_then(|variable| self.variable_of(variable))
    }

    fn bind_inputs(&self, cfg: &mut CFG) {
        let mut register = cfg.registers.saturating_sub(cfg.ins);
        let this_variable = if !cfg.method().is_static() && register < cfg.registers {
            let variable = self.variable_of(SsaVar::new(register, 0));
            register += 1;
            variable
        } else {
            None
        };
        let parameter_variables = cfg
            .method()
            .descriptor()
            .parameters
            .iter()
            .map(|parameter| {
                let value = self.variable_of(SsaVar::new(register, 0));
                register += parameter_slots(parameter);
                value
            })
            .collect();
        cfg.set_source_variables(this_variable, parameter_variables);
    }
}

struct UseCollector<'a> {
    graph: &'a mut SsaValueBuilder,
    position: UsePosition,
    error: Option<SsaInvariantError>,
}

impl InstructionVisitor for UseCollector<'_> {
    fn visit_register(&mut self, register: &RegisterArg) {
        if self.error.is_some() {
            return;
        }
        let Some(variable) = SsaVar::from_reg(register) else {
            self.error = Some(SsaInvariantError::UseWithoutIdentity(
                self.position.instruction,
            ));
            return;
        };
        let value = self.graph.value_mut(variable);
        value.uses.push(self.position);
        value.observe_type(&register.ty);
    }
}

#[derive(Debug, Clone)]
pub struct SsaClasses {
    parents: foldhash::HashMap<SsaVar, SsaVar>,
}

impl Default for SsaClasses {
    fn default() -> Self {
        Self::new([])
    }
}

impl SsaClasses {
    pub fn new(values: impl IntoIterator<Item = SsaVar>) -> Self {
        Self {
            parents: values.into_iter().map(|value| (value, value)).collect(),
        }
    }

    pub fn root(&mut self, value: SsaVar) -> SsaVar {
        let mut parent = *self.parents.entry(value).or_insert(value);
        if parent == value {
            return value;
        }
        let mut current = value;
        loop {
            let grandparent = self.parents[&parent];
            if grandparent == parent {
                return parent;
            }
            self.parents.insert(current, grandparent);
            current = parent;
            parent = grandparent;
        }
    }

    pub fn union(&mut self, left: SsaVar, right: SsaVar) {
        let left = self.root(left);
        let right = self.root(right);
        if left == right {
            return;
        }
        let (root, child) = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        self.parents.insert(child, root);
    }

    pub fn values(&self) -> Vec<SsaVar> {
        let mut values: Vec<_> = self.parents.keys().copied().collect();
        values.sort_unstable();
        values
    }

    pub(super) fn into_roots(mut self) -> foldhash::HashMap<SsaVar, SsaVar> {
        let keys: Vec<_> = self.parents.keys().copied().collect();
        for value in keys {
            let root = self.root(value);
            self.parents.insert(value, root);
        }
        self.parents
    }

    pub fn groups(&mut self) -> BTreeMap<SsaVar, BTreeSet<SsaVar>> {
        let mut groups = BTreeMap::<SsaVar, BTreeSet<SsaVar>>::new();
        for value in self.parents.keys().copied().collect::<Vec<_>>() {
            groups.entry(self.root(value)).or_default().insert(value);
        }
        groups
    }
}

fn parameter_slots(parameter: &ArgType) -> u32 {
    if parameter.is_wide() {
        2
    } else {
        1
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/ssa_value_layout.rs"]
mod ssa_value_layout;
