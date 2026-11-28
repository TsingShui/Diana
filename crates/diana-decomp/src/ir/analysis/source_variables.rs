//! Interference-aware allocation of SSA values to Kotlin source variables.
//!
//! SSA remains intact through region construction and sparse value recovery.
//! This analysis first assigns non-interfering values to source variables,
//! then lowers Phi edge copies into identified Semantic IR basic blocks.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ir::{
    BlockId, InsnArg, InstructionTreeError, RegionGraph, RegisterArg, SemanticExpression,
    SemanticFoldError, SemanticInstructions, SemanticMethod, SemanticNode, SemanticOperation,
    SemanticVisitor, SourceSemantics, ValueSemantics, CFG,
};

use super::types::SourceTypeLattice;
use super::{
    CodeVariables, ControlContractions, NormalCopySite, SourceTypeEnvironment, SsaClasses,
    SsaTypeEnvironment, SsaUseSite, SsaValueGraph, SsaVar, TypeConstraintError,
};

mod edge_arguments;
mod interference;
mod lexical_exits;
mod loop_exit_copies;
mod phi;

use interference::{ClassInterference, InterferenceGraph, SsaLiveness};
use lexical_exits::LexicalExitLowering;
use phi::{
    CleanupCatches, ExceptionalCopyPlacement, PhiCopies, PhiLowering, PhiTypeAnalysis,
    SemanticSsaValues,
};

pub struct SourceVariableAllocation {
    variables: CodeVariables,
    liveness: SsaLiveness,
    required_phis: BTreeSet<SsaVar>,
    statement_definitions: BTreeSet<SsaVar>,
    cleanup_catches: CleanupCatches,
    contractions: ControlContractions,
    phi_types: BTreeMap<SsaVar, crate::ir::ArgType>,
}

impl SourceVariableAllocation {
    pub fn analyze(
        cfg: &CFG,
        values: &SsaValueGraph,
        constants: &BTreeMap<SsaVar, InsnArg>,
        recovered_phis: &BTreeSet<SsaVar>,
        root: &SemanticNode,
        types: &SsaTypeEnvironment,
        hierarchy: &dyn super::TypeHierarchy,
        regions: &RegionGraph,
    ) -> Result<Self, SourceVariableError> {
        Self::analyze_with_flow(
            cfg,
            values,
            constants,
            recovered_phis,
            root,
            types,
            hierarchy,
            regions,
            ControlContractions::for_edge_arguments(cfg, regions),
            || std::sync::Arc::new(super::SemanticFlowGraph::analyze(root)),
        )
    }

    pub(crate) fn analyze_with_flow(
        cfg: &CFG,
        values: &SsaValueGraph,
        constants: &BTreeMap<SsaVar, InsnArg>,
        recovered_phis: &BTreeSet<SsaVar>,
        root: &SemanticNode,
        types: &SsaTypeEnvironment,
        hierarchy: &dyn super::TypeHierarchy,
        regions: &RegionGraph,
        contractions: ControlContractions,
        flow: impl FnOnce() -> std::sync::Arc<super::SemanticFlowGraph>,
    ) -> Result<Self, SourceVariableError> {
        let semantic_values = SemanticSsaValues::collect(root);
        let cleanup_catches = CleanupCatches::resolve(cfg, root, regions);
        let required_phis =
            semantic_values.required(cfg, values, constants, &contractions, &cleanup_catches)?;
        let (mut retained, statement_definitions) = semantic_values.into_retained_and_definitions();
        let mut cleanup_values = regions
            .cleanup_value_bindings()
            .iter()
            .flat_map(|(handler, normal)| [*handler, *normal])
            .collect::<BTreeSet<_>>();
        let mut pending = cleanup_values.iter().copied().collect::<Vec<_>>();
        while let Some(value) = pending.pop() {
            let Some(phi) = values.phi(value) else {
                continue;
            };
            for input in &phi.inputs {
                if cleanup_values.insert(input.value) {
                    pending.push(input.value);
                }
            }
        }
        retained.extend(required_phis.iter().copied());
        retained.extend(cleanup_values.iter().copied());
        retained.extend(cleanup_catches.inputs(&required_phis));
        let edge_arguments =
            edge_arguments::ContractedEdgeArguments::new(cfg, values, constants, &contractions);
        let inputs = SourceInputs::of(cfg);
        let candidates = CoalescingCandidates::collect(
            cfg,
            values,
            constants,
            regions,
            &cleanup_values,
            &required_phis,
            recovered_phis,
        );
        // Interference also introduces identities that exist only in recovered
        // code. Preserve those edges when the CFG does not own every endpoint.
        let undeclared = |value: SsaVar| values.value(value).is_none() && !inputs.contains(&value);
        let has_unowned = retained
            .iter()
            .chain(&statement_definitions)
            .copied()
            .any(undeclared)
            || candidates
                .0
                .iter()
                .any(|&(left, right, _)| undeclared(left) || undeclared(right));
        let groups = (!has_unowned).then(|| candidates.groups());
        // Copies can require any live name at their placement edge. Without
        // required Phi results the only physical live ranges consumed are
        // the endpoints of possible coalescing conflicts. Each name has an
        // independent gen/kill transfer, so projection commutes with liveness.
        let live_names = if let Some(groups) = groups.as_ref().filter(|_| required_phis.is_empty())
        {
            if groups.is_empty() {
                Box::default()
            } else {
                retained
                    .iter()
                    .filter(|name| groups.contains_key(name))
                    .copied()
                    .collect()
            }
        } else {
            retained.iter().copied().collect()
        };
        let liveness = SsaLiveness::from_names(cfg, live_names)?;
        let mut interference = InterferenceGraph::build(
            cfg,
            root,
            &liveness,
            &retained,
            &required_phis,
            groups,
            flow,
        )?;
        interference.add_exceptional_edge_interference(
            cfg,
            &liveness,
            &retained,
            &edge_arguments,
        )?;
        let phi_types =
            PhiTypeAnalysis::solve(cfg, &required_phis, values, types, constants, hierarchy)?;
        let variables = if has_unowned || !candidates.0.is_empty() {
            let exception_types = CatchVariableTypes::collect(root, hierarchy, SsaVar::from_reg);
            let mut coalescer = SourceClassCoalescer::new(
                &inputs,
                interference,
                types,
                values,
                &exception_types,
                &phi_types,
                hierarchy,
            );
            if !cfg.method().is_static() {
                if let Some(this) = inputs.first().copied() {
                    coalescer.fix(this);
                }
            }
            for &input in &inputs {
                coalescer.pin(input);
            }
            for (left, right, policy) in candidates.0 {
                coalescer.coalesce(left, right, policy);
            }
            CodeVariables::from_classes(values, &inputs, coalescer.finish())
        } else {
            CodeVariables::from_singletons(values, &inputs)
        };
        for value in retained {
            if variables.variable_of(value).is_none() {
                return Err(SourceVariableError::MissingCodeVariable(value));
            }
        }
        Ok(Self {
            variables,
            liveness,
            required_phis,
            statement_definitions,
            cleanup_catches,
            contractions,
            phi_types,
        })
    }

    fn annotate_cfg(&self, cfg: &mut CFG) -> Result<(), SourceVariableError> {
        self.variables
            .annotate(cfg)
            .map_err(SourceVariableError::InstructionTree)?;
        Ok(())
    }

    pub fn apply(
        &self,
        cfg: &mut CFG,
        mut method: SemanticMethod<ValueSemantics>,
        types: SsaTypeEnvironment,
        hierarchy: &dyn super::TypeHierarchy,
    ) -> Result<SemanticMethod<SourceSemantics>, SourceVariableError> {
        self.annotate_cfg(cfg)?;
        let needs_control_rewrite = SourceAnnotation::apply(&self.variables, method.body_mut());
        let copies = PhiCopies::collect(
            cfg,
            &self.variables,
            &self.cleanup_catches,
            &self.required_phis,
            &self.statement_definitions,
            method.state().values(),
            method.state().constants(),
            &self.phi_types,
            &self.contractions,
            method.state().regions(),
            hierarchy,
        )?;
        let copies = ExceptionalCopyPlacement::new(
            cfg,
            &self.variables,
            &self.liveness,
            &self.statement_definitions,
            &self.required_phis,
            method.body(),
            &self.contractions,
            method.state().regions(),
            method.state().values(),
            method.state().constants(),
        )
        .verify(copies)?;
        let mut source_types = SourceTypeEnvironment::from_ssa(
            types,
            &self.variables,
            method.state().values(),
            method.state().constants(),
            hierarchy,
        )?;
        DeclaredInputTypes::apply(cfg, &mut source_types, hierarchy)?;
        let phi_lowering = PhiLowering::new(
            cfg,
            copies,
            self.variables.next_id(),
            &mut source_types,
            hierarchy,
            method.state().regions(),
        )?;
        let next_variable = phi_lowering.next_variable();
        phi_lowering.apply(method.body_mut(), needs_control_rewrite)?;
        LexicalExitLowering::apply(method.body_mut(), next_variable, &mut source_types)?;
        ExceptionTypeBinding::apply(method.body(), &mut source_types, hierarchy)?;
        let (body, state) = method.into_parts();
        Ok(SemanticMethod::from_source(
            body,
            source_types,
            state.into_regions(),
        ))
    }
}

type CoalescingPairs = smallvec::SmallVec<[(SsaVar, SsaVar, CoalescingPolicy); 4]>;

struct CoalescingCandidates(CoalescingPairs);

impl CoalescingCandidates {
    fn collect(
        cfg: &CFG,
        values: &SsaValueGraph,
        constants: &BTreeMap<SsaVar, InsnArg>,
        regions: &RegionGraph,
        cleanup_values: &BTreeSet<SsaVar>,
        required_phis: &BTreeSet<SsaVar>,
        recovered_phis: &BTreeSet<SsaVar>,
    ) -> Self {
        let mut pairs = CoalescingPairs::new();
        for (handler, normal) in regions.cleanup_value_bindings() {
            pairs.push((*handler, *normal, CoalescingPolicy::SemanticCongruence));
        }
        for phi in values
            .phis()
            .iter()
            .filter(|phi| cleanup_values.contains(&phi.result))
        {
            for input in &phi.inputs {
                pairs.push((
                    phi.result,
                    input.value,
                    CoalescingPolicy::SemanticCongruence,
                ));
            }
        }
        let mut coalescing_phis = required_phis
            .iter()
            .chain(recovered_phis)
            .copied()
            .collect::<BTreeSet<_>>();
        let mut pending_phis = coalescing_phis.iter().copied().collect::<Vec<_>>();
        while let Some(result) = pending_phis.pop() {
            let Some(phi) = values.phi(result) else {
                continue;
            };
            for input in &phi.inputs {
                if values.phi(input.value).is_some() && coalescing_phis.insert(input.value) {
                    pending_phis.push(input.value);
                }
            }
        }
        for phi in values
            .phis()
            .iter()
            .filter(|phi| coalescing_phis.contains(&phi.result))
        {
            let acyclic = phi
                .inputs
                .iter()
                .filter(|input| !constants.contains_key(&input.value))
                .all(|input| !regions.semantic_dominates(phi.block, input.predecessor));
            for input in &phi.inputs {
                if !constants.contains_key(&input.value) {
                    let phi_only = values.value(input.value).is_some_and(|value| {
                        value.uses.iter().all(|usage| {
                            matches!(
                                values.use_site(cfg, input.value, *usage),
                                Some(SsaUseSite::Phi(_))
                            )
                        })
                    });
                    let policy = if acyclic {
                        CoalescingPolicy::AcyclicPhi { phi_only }
                    } else {
                        CoalescingPolicy::AllInterference
                    };
                    pairs.push((phi.result, input.value, policy));
                }
            }
        }
        CanonicalCatchParameters::collect(cfg, regions).append_to(&mut pairs);
        for copy in values
            .copies()
            .iter()
            .filter(|copy| !constants.contains_key(&copy.source))
        {
            pairs.push((copy.result, copy.source, CoalescingPolicy::AllInterference));
        }
        Self(pairs)
    }

    /// Every accepted union is one of these pairs. Even after any sequence of
    /// accepted and rejected proposals, two roots in different components
    /// cannot participate in the same interference query.
    fn groups(&self) -> foldhash::HashMap<SsaVar, SsaVar> {
        let mut groups = SsaClasses::default();
        for &(left, right, _) in &self.0 {
            if left != right {
                groups.union(left, right);
            }
        }
        groups.into_roots()
    }
}

struct CanonicalCatchParameters {
    groups: Vec<Vec<SsaVar>>,
}

impl CanonicalCatchParameters {
    fn collect(cfg: &CFG, regions: &RegionGraph) -> Self {
        let mut groups = BTreeMap::<BlockId, Vec<SsaVar>>::new();
        for (&adapter, &canonical_entry) in regions.handler_adapters() {
            let values = cfg
                .block(adapter)
                .into_iter()
                .flat_map(|block| &block.insns)
                .filter(|instruction| instruction.insn_type == crate::ir::InsnType::MoveException)
                .filter_map(|instruction| instruction.result.as_ref())
                .filter_map(SsaVar::from_reg);
            groups.entry(canonical_entry).or_default().extend(values);
        }
        Self {
            groups: groups.into_values().collect(),
        }
    }

    fn append_to(self, pairs: &mut CoalescingPairs) {
        for group in self.groups {
            let Some((&representative, rest)) = group.split_first() else {
                continue;
            };
            for &value in rest {
                pairs.push((representative, value, CoalescingPolicy::SemanticCongruence));
            }
        }
    }
}

struct DeclaredInputTypes;

impl DeclaredInputTypes {
    fn apply(
        cfg: &CFG,
        types: &mut SourceTypeEnvironment,
        hierarchy: &dyn super::TypeHierarchy,
    ) -> Result<(), TypeConstraintError> {
        if let Some(variable) = cfg.this_code_variable() {
            types.bind_declared_variable(variable, cfg.method().owner().clone(), hierarchy)?;
        }
        for (variable, ty) in cfg
            .parameter_code_variables()
            .iter()
            .zip(&cfg.method().descriptor().parameters)
        {
            if let Some(variable) = variable {
                types.bind_declared_variable(*variable, ty.clone(), hierarchy)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ExceptionDefinition {
    Null,
    Copy(u32),
    Other,
}

struct ExceptionTypeBinding<'a> {
    exception_types: ExceptionTypes<'a>,
    seeds: BTreeMap<u32, crate::ir::ArgType>,
    definitions: BTreeMap<u32, Vec<ExceptionDefinition>>,
}

impl<'a> ExceptionTypeBinding<'a> {
    fn apply(
        root: &SemanticNode,
        types: &'a mut SourceTypeEnvironment,
        hierarchy: &'a dyn super::TypeHierarchy,
    ) -> Result<(), TypeConstraintError> {
        let seeds = CatchVariableTypes::collect(root, hierarchy, |register| register.code_var);
        if seeds.is_empty() {
            return Ok(());
        }
        let mut binding = Self {
            exception_types: ExceptionTypes::new(hierarchy),
            seeds,
            definitions: BTreeMap::new(),
        };
        binding.visit_node(root);
        for (variable, ty) in binding.solve() {
            types.bind_exception_type(variable, ty)?;
        }
        Ok(())
    }

    fn solve(&self) -> BTreeMap<u32, crate::ir::ArgType> {
        let mut inferred = self.seeds.clone();
        loop {
            let mut changed = false;
            for (&variable, definitions) in &self.definitions {
                if self.seeds.contains_key(&variable) {
                    continue;
                }
                let mut ty = None;
                let mut complete = true;
                for definition in definitions {
                    match definition {
                        ExceptionDefinition::Null => {}
                        ExceptionDefinition::Copy(source) if *source == variable => {}
                        ExceptionDefinition::Copy(source) => {
                            let Some(source_type) = inferred.get(source) else {
                                complete = false;
                                break;
                            };
                            ty = Some(match ty {
                                Some(current) => {
                                    self.exception_types.merge_pair(&current, source_type)
                                }
                                None => source_type.clone(),
                            });
                        }
                        ExceptionDefinition::Other => {
                            complete = false;
                            break;
                        }
                    }
                }
                let Some(ty) = complete.then_some(ty).flatten() else {
                    continue;
                };
                if inferred.get(&variable) != Some(&ty) {
                    inferred.insert(variable, ty);
                    changed = true;
                }
            }
            if !changed {
                return inferred;
            }
        }
    }

    fn classify(mut value: &SemanticExpression) -> ExceptionDefinition {
        loop {
            match value {
                SemanticExpression::Literal(literal)
                    if literal.value == 0 && literal.ty.is_reference() =>
                {
                    return ExceptionDefinition::Null;
                }
                SemanticExpression::Register(register) => {
                    return register
                        .code_var
                        .map(ExceptionDefinition::Copy)
                        .unwrap_or(ExceptionDefinition::Other);
                }
                SemanticExpression::Operation(operation)
                    if operation.insn_type == crate::ir::InsnType::Move
                        && operation.operands().len() == 1 =>
                {
                    value = &operation.operands()[0];
                }
                SemanticExpression::Literal(_)
                | SemanticExpression::Operation(_)
                | SemanticExpression::Select { .. } => return ExceptionDefinition::Other,
            }
        }
    }
}

impl SemanticVisitor for ExceptionTypeBinding<'_> {
    fn visit_statement(&mut self, statement: &crate::ir::SemanticStatement) {
        let (result, definition) = match &statement.kind {
            crate::ir::SemanticStatementKind::Definition { result, value, .. } => {
                (result, Self::classify(value))
            }
            crate::ir::SemanticStatementKind::Instruction(operation) => {
                let Some(result) = operation.result.as_ref() else {
                    return;
                };
                let definition = match operation.operands() {
                    [operand] if operation.insn_type == crate::ir::InsnType::Move => {
                        Self::classify(operand)
                    }
                    _ => ExceptionDefinition::Other,
                };
                (result, definition)
            }
        };
        if let Some(variable) = result.code_var {
            self.definitions
                .entry(variable)
                .or_default()
                .push(definition);
        }
    }

    fn visit_predicate(&mut self, _: &crate::ir::SemanticPredicate) {}

    fn visit_expression(&mut self, _: &SemanticExpression) {}
}

struct CatchVariableTypes<'a, K> {
    types: ExceptionTypes<'a>,
    key: fn(&RegisterArg) -> Option<K>,
    values: BTreeMap<K, crate::ir::ArgType>,
}

impl<'a, K: Ord> CatchVariableTypes<'a, K> {
    fn collect(
        root: &SemanticNode,
        hierarchy: &'a dyn super::TypeHierarchy,
        key: fn(&RegisterArg) -> Option<K>,
    ) -> BTreeMap<K, crate::ir::ArgType> {
        let mut collector = Self {
            types: ExceptionTypes::new(hierarchy),
            key,
            values: BTreeMap::new(),
        };
        collector.visit_node(root);
        collector.values
    }
}

impl<K: Ord> SemanticVisitor for CatchVariableTypes<'_, K> {
    fn visits_values(&self) -> bool {
        false
    }

    fn enter_node(&mut self, node: &SemanticNode) {
        let SemanticNode::Try { catches, .. } = node else {
            return;
        };
        for catch in catches {
            let Some(key) = catch.exception_value.as_ref().and_then(self.key) else {
                continue;
            };
            let ty = self.types.merge(&catch.exception_types);
            self.values
                .entry(key)
                .and_modify(|current| *current = self.types.merge_pair(current, &ty))
                .or_insert(ty);
        }
    }
}

struct ExceptionTypes<'a> {
    lattice: SourceTypeLattice<'a>,
}

impl<'a> ExceptionTypes<'a> {
    fn new(hierarchy: &'a dyn super::TypeHierarchy) -> Self {
        Self {
            lattice: SourceTypeLattice::new(hierarchy),
        }
    }

    fn merge(&self, types: &[crate::ir::ArgType]) -> crate::ir::ArgType {
        match types.split_first() {
            Some((first, rest)) => rest
                .iter()
                .fold(first.clone(), |left, right| self.merge_pair(&left, right)),
            None => crate::ir::ArgType::throwable(),
        }
    }

    fn merge_pair(
        &self,
        left: &crate::ir::ArgType,
        right: &crate::ir::ArgType,
    ) -> crate::ir::ArgType {
        self.lattice
            .join(left, right)
            .filter(|ty| ty.as_object() != Some("java/lang/Object"))
            .unwrap_or_else(crate::ir::ArgType::throwable)
    }
}

struct SourceClassCoalescer<'a> {
    classes: SsaClasses,
    interference: ClassInterference,
    inputs: &'a [SsaVar],
    types: &'a SsaTypeEnvironment,
    values: &'a SsaValueGraph,
    exception_types: &'a BTreeMap<SsaVar, crate::ir::ArgType>,
    phi_types: &'a BTreeMap<SsaVar, crate::ir::ArgType>,
    merged_types: BTreeMap<SsaVar, crate::ir::ArgType>,
    type_lattice: SourceTypeLattice<'a>,
    pinned: BTreeSet<SsaVar>,
    fixed: BTreeSet<SsaVar>,
}

impl<'a> SourceClassCoalescer<'a> {
    fn new(
        inputs: &'a [SsaVar],
        interference: InterferenceGraph,
        types: &'a SsaTypeEnvironment,
        values: &'a SsaValueGraph,
        exception_types: &'a BTreeMap<SsaVar, crate::ir::ArgType>,
        phi_types: &'a BTreeMap<SsaVar, crate::ir::ArgType>,
        hierarchy: &'a dyn super::TypeHierarchy,
    ) -> Self {
        let mut classes = SsaClasses::default();
        let interference = ClassInterference::from_singletons(interference, &mut classes);
        Self {
            classes,
            interference,
            inputs,
            types,
            values,
            exception_types,
            phi_types,
            merged_types: BTreeMap::new(),
            type_lattice: SourceTypeLattice::new(hierarchy),
            pinned: BTreeSet::new(),
            fixed: BTreeSet::new(),
        }
    }

    fn pin(&mut self, value: SsaVar) {
        let root = self.classes.root(value);
        self.pinned.insert(root);
    }

    fn fix(&mut self, value: SsaVar) {
        let root = self.classes.root(value);
        self.fixed.insert(root);
    }

    fn coalesce(&mut self, left: SsaVar, right: SsaVar, policy: CoalescingPolicy) {
        let left = self.classes.root(left);
        let right = self.classes.root(right);
        if left == right
            || self.fixed.contains(&left)
            || self.fixed.contains(&right)
            || (self.pinned.contains(&left) && self.pinned.contains(&right))
            || policy.interferes(&self.interference, left, right)
        {
            return;
        }
        let Ok(merged_type) = policy.merged_type(self, left, right) else {
            return;
        };
        self.classes.union(left, right);
        let root = self.classes.root(left);
        let absorbed = if root == left { right } else { left };
        self.merged_types.remove(&absorbed);
        if let Some(ty) = merged_type {
            self.merged_types.insert(root, ty);
        }
        if self.pinned.remove(&absorbed) {
            self.pinned.insert(root);
        }
        if self.fixed.remove(&absorbed) {
            self.fixed.insert(root);
        }
        self.interference.merge(root, absorbed);
    }

    fn class_type(&self, variable: SsaVar) -> Option<Cow<'_, crate::ir::ArgType>> {
        if let Some(ty) = self
            .merged_types
            .get(&variable)
            .or_else(|| self.phi_types.get(&variable))
            .or_else(|| self.exception_types.get(&variable))
        {
            return Some(Cow::Borrowed(ty));
        }
        let value = self.values.value(variable);
        if value.is_none() && !self.inputs.contains(&variable) {
            return None;
        }
        self.types
            .value_type(variable)
            .map(Cow::Borrowed)
            .or_else(|| {
                self.type_lattice
                    .select_observed(&value?.observed_types)
                    .map(Cow::Owned)
            })
    }

    fn finish(self) -> SsaClasses {
        self.classes
    }
}

#[derive(Debug, Clone, Copy)]
enum CoalescingPolicy {
    AllInterference,
    AcyclicPhi { phi_only: bool },
    SemanticCongruence,
}

impl CoalescingPolicy {
    fn interferes(self, graph: &ClassInterference, left: SsaVar, right: SsaVar) -> bool {
        match self {
            Self::AllInterference => graph.interferes(left, right),
            Self::AcyclicPhi { .. } => graph.hard_interferes(left, right),
            // Cleanup equivalence and its Phi closure describe one logical
            // source state across mutually exclusive normal/exceptional
            // domains. Physical exceptional liveness overlaps those domains
            // by construction and is not a valid conflict for this class.
            Self::SemanticCongruence => false,
        }
    }

    fn merged_type(
        self,
        coalescer: &SourceClassCoalescer<'_>,
        left: SsaVar,
        right: SsaVar,
    ) -> Result<Option<crate::ir::ArgType>, ()> {
        match (coalescer.class_type(left), coalescer.class_type(right)) {
            (Some(left), Some(right)) => match self {
                Self::AllInterference | Self::SemanticCongruence => coalescer
                    .type_lattice
                    .meet(&left, &right)
                    .map(Some)
                    .ok_or(()),
                Self::AcyclicPhi { .. } if left == right => Ok(Some(left.into_owned())),
                Self::AcyclicPhi { phi_only: true } => coalescer
                    .type_lattice
                    .join(&left, &right)
                    .map(Some)
                    .ok_or(()),
                Self::AcyclicPhi { phi_only: false } => Err(()),
            },
            (Some(ty), None) | (None, Some(ty)) => Ok(Some(ty.into_owned())),
            (None, None) => Ok(None),
        }
    }
}

struct SourceInputs;

impl SourceInputs {
    fn of(cfg: &CFG) -> Vec<SsaVar> {
        let mut register = cfg.registers.saturating_sub(cfg.ins);
        let mut inputs = Vec::new();
        if !cfg.method().is_static() && register < cfg.registers {
            inputs.push(SsaVar::new(register, 0));
            register += 1;
        }
        for parameter in &cfg.method().descriptor().parameters {
            if register < cfg.registers {
                inputs.push(SsaVar::new(register, 0));
            }
            register += if parameter.is_wide() { 2 } else { 1 };
        }
        inputs
    }
}

struct SourceAnnotation;

impl SourceAnnotation {
    fn apply(variables: &CodeVariables, root: &mut SemanticNode) -> bool {
        let mut needs_control_rewrite = false;
        crate::ir::semantic::for_each_node_mut(root, |node| {
            needs_control_rewrite |= PhiLowering::rewrites_without_copies(node);
            SemanticInstructions::bind_code_variables(node, |register| {
                SsaVar::from_reg(register).and_then(|value| variables.variable_of(value))
            });
            if let SemanticNode::BasicBlock(block) = node {
                block.statements.retain(|statement| {
                    statement
                        .instruction_ref()
                        .is_none_or(|op| !Self::is_identity_copy(op))
                });
            }
        });
        needs_control_rewrite
    }

    fn is_identity_copy(instruction: &SemanticOperation) -> bool {
        if instruction.insn_type != crate::ir::InsnType::Move {
            return false;
        }
        let Some(destination) = instruction
            .result
            .as_ref()
            .and_then(|result| result.code_var)
        else {
            return false;
        };
        instruction
            .operands()
            .first()
            .and_then(SemanticExpression::as_register)
            .and_then(|source| source.code_var)
            == Some(destination)
    }
}

#[derive(Debug)]
pub enum SourceVariableError {
    InstructionTree(InstructionTreeError),
    SemanticTransform(SemanticFoldError),
    Type(TypeConstraintError),
    Dominance(super::DominanceError),
    MissingPhiResult(BlockId),
    NonRegisterPhiInput {
        predecessor: BlockId,
        successor: BlockId,
    },
    UnsplitNormalEdge {
        predecessor: BlockId,
        successor: BlockId,
        targets: Vec<BlockId>,
    },
    MissingBlock(BlockId),
    MissingThrowSite(BlockId),
    MissingSemanticSite(&'static str),
    IncompleteSemanticFlow,
    MissingRegisterIdentity,
    MissingCodeVariable(SsaVar),
    LivenessMismatch(BlockId),
    ExceptionalCopyClobbersNormalValue {
        predecessor: BlockId,
        variable: u32,
    },
    ConflictingExceptionalCopies {
        predecessor: BlockId,
        variable: u32,
        left_handler: BlockId,
        right_handler: BlockId,
    },
    ConflictingPhiVariable {
        variable: u32,
        left_value: SsaVar,
        left_block: BlockId,
        left_type: crate::ir::ArgType,
        right_value: SsaVar,
        right_block: BlockId,
        right_type: crate::ir::ArgType,
    },
    EmptyParallelCopyCycle,
    MissingSemanticBlock(BlockId),
    MissingSemanticEdge(crate::ir::RegionEdge),
    MissingExceptionalCopySite {
        block: BlockId,
        instruction: crate::ir::InstructionId,
    },
    MissingHandlerEntry(BlockId),
    UnresolvedContractedPhi {
        predecessor: BlockId,
        successor: BlockId,
        value: SsaVar,
    },
    DuplicateSemanticBlock(BlockId),
    MissingSemanticThrowSite(BlockId),
    SyntheticIdentityExhausted,
}

impl fmt::Display for SourceVariableError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InstructionTree(error) => error.fmt(formatter),
            Self::SemanticTransform(error) => error.fmt(formatter),
            Self::Type(error) => error.fmt(formatter),
            Self::Dominance(error) => error.fmt(formatter),
            Self::MissingPhiResult(block) => write!(formatter, "Phi in {block} has no result"),
            Self::NonRegisterPhiInput {
                predecessor,
                successor,
            } => write!(
                formatter,
                "Phi edge {predecessor} -> {successor} has a non-register input"
            ),
            Self::UnsplitNormalEdge {
                predecessor,
                successor,
                targets,
            } => write!(
                formatter,
                "normal Phi edge {predecessor} -> {successor} is not split: {targets:?}"
            ),
            Self::MissingBlock(block) => write!(formatter, "missing Phi predecessor {block}"),
            Self::MissingThrowSite(block) => {
                write!(
                    formatter,
                    "exceptional Phi predecessor {block} has no unique terminal throw site"
                )
            }
            Self::MissingSemanticSite(kind) => {
                write!(formatter, "structured {kind} has no numbered semantic site")
            }
            Self::IncompleteSemanticFlow => {
                formatter.write_str("structured semantic flow is incomplete")
            }
            Self::MissingRegisterIdentity => {
                formatter.write_str("parallel copy register has no SSA identity")
            }
            Self::MissingCodeVariable(variable) => {
                write!(formatter, "SSA value {variable:?} has no source variable")
            }
            Self::LivenessMismatch(block) => {
                write!(
                    formatter,
                    "SSA liveness transfer is inconsistent in {block}"
                )
            }
            Self::ExceptionalCopyClobbersNormalValue {
                predecessor,
                variable,
            } => write!(
                formatter,
                "exceptional copy in {predecessor} clobbers normal-live variable v{variable}"
            ),
            Self::ConflictingExceptionalCopies {
                predecessor,
                variable,
                left_handler,
                right_handler,
            } => write!(
                formatter,
                "exceptional edges from {predecessor} to {left_handler} and {right_handler} assign different values to v{variable}"
            ),
            Self::ConflictingPhiVariable {
                variable,
                left_value,
                left_block,
                left_type,
                right_value,
                right_block,
                right_type,
            } => write!(
                formatter,
                "source variable v{variable} combines Phi {left_value:?} in {left_block} ({left_type:?}) with {right_value:?} in {right_block} ({right_type:?})"
            ),
            Self::EmptyParallelCopyCycle => formatter.write_str("empty parallel-copy cycle"),
            Self::MissingSemanticBlock(block) => {
                write!(formatter, "Phi lowering cannot find semantic block {block}")
            }
            Self::MissingSemanticEdge(edge) => {
                write!(
                    formatter,
                    "Phi lowering cannot find semantic edge {} -> {} ({:?})",
                    edge.source, edge.target, edge.kind
                )
            }
            Self::MissingExceptionalCopySite { block, instruction } => write!(
                formatter,
                "exceptional Phi copy for instruction {instruction:?} has no semantic site in {block}"
            ),
            Self::MissingHandlerEntry(block) => {
                write!(
                    formatter,
                    "exceptional Phi copy has no handler entry {block}"
                )
            }
            Self::UnresolvedContractedPhi {
                predecessor,
                successor,
                value,
            } => write!(
                formatter,
                "SSA value {value:?} on contracted edge {predecessor} -> {successor} is not representable"
            ),
            Self::DuplicateSemanticBlock(block) => {
                write!(formatter, "semantic block {block} occurs more than once")
            }
            Self::MissingSemanticThrowSite(block) => {
                write!(
                    formatter,
                    "semantic block {block} has no throwing instruction for exceptional Phi copies"
                )
            }
            Self::SyntheticIdentityExhausted => {
                formatter.write_str("source control lowering exhausted method-local identities")
            }
        }
    }
}

impl std::error::Error for SourceVariableError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InstructionTree(error) => Some(error),
            Self::SemanticTransform(error) => Some(error),
            Self::Type(error) => Some(error),
            Self::Dominance(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SemanticFoldError> for SourceVariableError {
    fn from(source: SemanticFoldError) -> Self {
        Self::SemanticTransform(source)
    }
}

impl From<TypeConstraintError> for SourceVariableError {
    fn from(source: TypeConstraintError) -> Self {
        Self::Type(source)
    }
}
