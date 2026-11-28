use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{
    analysis::{SsaClasses, SsaValueGraph, SsaVar},
    ArgType, ArithOp, IfOp, InsnArg, InsnNode, InsnType, InstructionTree, InstructionVisitor,
    MemberReference, RegisterArg, CFG,
};

use super::TypeConstraintError;

use super::bounds::{BoundKind, TypeBound, TypeBounds};

/// Constraints use the immutable graph's existing SSA row numbers. Method
/// inputs and observations absent from the graph still own independent rows.
struct ObservedBounds<'a> {
    values: &'a SsaValueGraph,
    indexed: Vec<(SsaVar, TypeBounds)>,
    additional: BTreeMap<SsaVar, TypeBounds>,
}

impl<'a> ObservedBounds<'a> {
    fn new(values: &'a SsaValueGraph) -> Self {
        Self {
            values,
            indexed: values
                .values()
                .map(|fact| (fact.variable, TypeBounds::default()))
                .collect(),
            additional: BTreeMap::new(),
        }
    }

    fn for_value(&mut self, value: SsaVar) -> &mut TypeBounds {
        match self.values.value_position(value) {
            Some(row) => &mut self.indexed[row].1,
            None => self.additional.entry(value).or_default(),
        }
    }

    fn into_sorted(self) -> Vec<(SsaVar, TypeBounds)> {
        if self.additional.is_empty() {
            return self.indexed;
        }
        let rows = self.indexed.into_iter();
        let mut additional = self.additional.into_iter().peekable();
        let mut sorted = Vec::with_capacity(rows.len() + additional.len());
        for (value, bounds) in rows {
            while let Some(row) = additional.next_if(|(key, _)| *key < value) {
                sorted.push(row);
            }
            sorted.push((value, bounds));
        }
        sorted.extend(additional);
        sorted
    }
}

pub(super) struct TypeConstraintGraph<'a> {
    classes: Option<SsaClasses>,
    bounds: ObservedBounds<'a>,
    flows: BTreeSet<(SsaVar, SsaVar, FlowKind)>,
    upper_flows: BTreeSet<(SsaVar, SsaVar)>,
    arrays: BTreeSet<ArrayConstraint>,
}

pub(super) struct NormalizedTypeConstraints {
    /// Maps observed values to roots when copies merge identities. Otherwise the
    /// original bounds keys are precisely the observed singleton values.
    pub(super) members: Option<Vec<(SsaVar, SsaVar)>>,
    pub(super) bounds: Vec<(SsaVar, TypeBounds)>,
    pub(super) flows: BTreeSet<(SsaVar, SsaVar, FlowKind)>,
    pub(super) upper_flows: BTreeSet<(SsaVar, SsaVar)>,
    pub(super) arrays: BTreeSet<ArrayConstraint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum FlowKind {
    Phi,
    Bitwise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ArrayConstraint {
    Read { array: SsaVar, result: SsaVar },
    Write { array: SsaVar, value: SsaVar },
}

impl<'a> TypeConstraintGraph<'a> {
    fn new(values: &'a SsaValueGraph) -> Self {
        // An empty row records a known SSA identity even before type evidence
        // reaches it. Bounds from synthetic inputs extend the same index.
        Self {
            classes: None,
            bounds: ObservedBounds::new(values),
            flows: BTreeSet::new(),
            upper_flows: BTreeSet::new(),
            arrays: BTreeSet::new(),
        }
    }

    pub(super) fn collect(
        cfg: &CFG,
        values: &'a SsaValueGraph,
        constants: &BTreeMap<SsaVar, InsnArg>,
    ) -> Result<Self, TypeConstraintError> {
        let mut graph = Self::new(values);
        for copy in values.copies() {
            if !constants.contains_key(&copy.source) {
                graph
                    .classes
                    .get_or_insert_with(SsaClasses::default)
                    .union(copy.result, copy.source);
            }
        }
        for (&value, constant) in constants {
            if let Some(ty) = Self::constant_fallback(constant) {
                graph.add_value_bound(value, BoundKind::Fallback, ty);
            }
        }
        for phi in values.phis() {
            for input in &phi.inputs {
                if let Some(ty) = constants
                    .get(&input.value)
                    .and_then(Self::constant_fallback)
                {
                    graph.add_value_bound(phi.result, BoundKind::Fallback, ty);
                    graph.upper_flows.insert((input.value, phi.result));
                } else {
                    graph.flows.insert((input.value, phi.result, FlowKind::Phi));
                }
            }
        }
        graph.collect_method_inputs(cfg)?;
        let mut catch_types = BTreeMap::<u32, BTreeSet<ArgType>>::new();
        for handler in &cfg.handlers {
            catch_types.entry(handler.handler).or_default().insert(
                handler
                    .catch_type
                    .clone()
                    .unwrap_or_else(ArgType::throwable),
            );
        }
        for block in cfg.blocks.values() {
            for instruction in &block.insns {
                graph.constrain_instruction(instruction, cfg, &catch_types)?;
            }
        }
        Ok(graph)
    }

    pub(super) fn normalize(mut self) -> NormalizedTypeConstraints {
        let mut rows = self.bounds.into_sorted();
        let Some(classes) = self.classes else {
            self.flows.retain(|(source, target, _)| source != target);
            self.upper_flows.retain(|(source, target)| source != target);
            return NormalizedTypeConstraints {
                members: None,
                bounds: rows,
                flows: self.flows,
                upper_flows: self.upper_flows,
                arrays: self.arrays,
            };
        };
        let copies = classes.into_roots();
        let members: Vec<_> = rows
            .iter()
            .map(|&(value, _)| (value, copies.get(&value).copied().unwrap_or(value)))
            .collect();
        let positions = super::RegisterRows::new(&members, |row| row.0);
        let position = |value| positions.position(&members, value, |row| row.0);
        let root_of = |value| position(value).map_or(value, |index| members[index].1);
        // A component root is its least identity. Accumulate evidence in that
        // row, visiting the members in their original SSA order. A root absent
        // from the observations can still collect evidence from its members.
        let mut unobserved = BTreeMap::<SsaVar, TypeBounds>::new();
        for (index, &(value, root)) in members.iter().enumerate() {
            if value != root {
                let bounds = std::mem::take(&mut rows[index].1);
                if let Some(owner) = position(root) {
                    rows[owner].1.merge(bounds);
                } else {
                    unobserved.entry(root).or_default().merge(bounds);
                }
            }
        }
        rows.retain(|(value, _)| copies.get(value).is_none_or(|root| root == value));
        if !unobserved.is_empty() {
            rows.extend(unobserved);
            rows.sort_unstable_by_key(|row| row.0);
        }
        let flows = self
            .flows
            .into_iter()
            .filter_map(|(source, target, kind)| {
                let source = root_of(source);
                let target = root_of(target);
                (source != target).then_some((source, target, kind))
            })
            .collect();
        let upper_flows = self
            .upper_flows
            .into_iter()
            .filter_map(|(source, target)| {
                let source = root_of(source);
                let target = root_of(target);
                (source != target).then_some((source, target))
            })
            .collect();
        let arrays = self
            .arrays
            .into_iter()
            .map(|constraint| match constraint {
                ArrayConstraint::Read { array, result } => ArrayConstraint::Read {
                    array: root_of(array),
                    result: root_of(result),
                },
                ArrayConstraint::Write { array, value } => ArrayConstraint::Write {
                    array: root_of(array),
                    value: root_of(value),
                },
            })
            .collect();
        NormalizedTypeConstraints {
            members: Some(members),
            bounds: rows,
            flows,
            upper_flows,
            arrays,
        }
    }

    fn collect_method_inputs(&mut self, cfg: &CFG) -> Result<(), TypeConstraintError> {
        let mut register = cfg.registers.saturating_sub(cfg.ins);
        if !cfg.method().is_static() {
            self.add_value_bound(
                SsaVar::new(register, 0),
                BoundKind::Exact,
                cfg.method().owner().clone(),
            );
            register += 1;
        }
        for ty in &cfg.method().descriptor().parameters {
            self.add_value_bound(SsaVar::new(register, 0), BoundKind::Exact, ty.clone());
            register += if ty.is_wide() { 2 } else { 1 };
        }
        Ok(())
    }

    fn constrain_instruction(
        &mut self,
        instruction: &InsnNode,
        cfg: &CFG,
        catch_types: &BTreeMap<u32, BTreeSet<ArgType>>,
    ) -> Result<(), TypeConstraintError> {
        if let Some(result) = &instruction.result {
            self.observe_register_type(result);
        }
        self.observe_arguments(instruction);

        match instruction.insn_type {
            InsnType::Invoke | InsnType::Constructor => self.constrain_invoke(instruction)?,
            InsnType::Iget | InsnType::Iput | InsnType::Sget | InsnType::Sput => {
                self.constrain_field(instruction)?
            }
            InsnType::Return => {
                if let Some(value) = instruction.args.first() {
                    let ty = cfg.method().descriptor().return_type.clone();
                    let kind = if ty.is_primitive() {
                        BoundKind::Exact
                    } else {
                        BoundKind::Upper
                    };
                    self.add_argument_bound(value, kind, ty);
                }
            }
            InsnType::Throw => {
                if let Some(value) = instruction.args.first() {
                    self.add_argument_bound(value, BoundKind::Upper, ArgType::throwable());
                }
            }
            InsnType::CheckCast | InsnType::Cast => {
                if let (Some(result), Some(ty)) =
                    (&instruction.result, instruction.conversion_type())
                {
                    self.add_register_bound(result, BoundKind::Exact, ty.clone());
                }
                if instruction.insn_type == InsnType::CheckCast {
                    if let Some(value) = instruction.args.first() {
                        self.add_argument_bound(value, BoundKind::Upper, ArgType::unknown_object());
                    }
                }
            }
            InsnType::MoveException => {
                if let Some(result) = &instruction.result {
                    self.add_register_bound(result, BoundKind::Upper, ArgType::throwable());
                    // The ordered handler table supplies a producer type for
                    // each real landing pad. Shared entries carry the union
                    // of their clauses, so exception values can flow into Phi
                    // arguments even with no typed register use at the catch.
                    for ty in catch_types.get(&instruction.offset).into_iter().flatten() {
                        self.add_register_bound(result, BoundKind::Lower, ty.clone());
                    }
                }
            }
            InsnType::NewInstance => {
                if let (Some(result), Some(ty)) = (
                    &instruction.result,
                    instruction.payload.class_type.as_deref(),
                ) {
                    self.add_register_bound(result, BoundKind::Exact, ty.clone());
                }
            }
            InsnType::ConstClass => {
                if let Some(result) = &instruction.result {
                    self.add_register_bound(
                        result,
                        BoundKind::Exact,
                        ArgType::object("java/lang/Class"),
                    );
                }
            }
            InsnType::InstanceOf => {
                if let Some(result) = &instruction.result {
                    self.add_register_bound(result, BoundKind::PredicateWord, ArgType::BOOLEAN);
                }
                if let Some(value) = instruction.args.first() {
                    self.add_argument_bound(value, BoundKind::Upper, ArgType::unknown_object());
                }
            }
            InsnType::ArrayLength => {
                if let Some(result) = &instruction.result {
                    self.add_register_bound(result, BoundKind::Exact, ArgType::INT);
                }
                if let Some(array) = instruction.args.first() {
                    self.add_argument_bound(array, BoundKind::Upper, ArgType::unknown_object());
                }
            }
            InsnType::Aget => self.constrain_array_get(instruction),
            InsnType::Aput => self.constrain_array_put(instruction),
            InsnType::NewArray | InsnType::FilledNewArray => {
                if let Some(result) = &instruction.result {
                    if let Some(array_type) = instruction.payload.class_type.as_deref() {
                        self.add_register_bound(result, BoundKind::Exact, array_type.clone());
                    }
                }
                if instruction.insn_type == InsnType::NewArray {
                    for size in &instruction.args {
                        self.add_argument_bound(size, BoundKind::Upper, ArgType::INT);
                    }
                } else if let Some(array) = instruction.result.as_ref().and_then(SsaVar::from_reg) {
                    for value in &instruction.args {
                        if let Some(value) = Self::argument_value(value) {
                            self.arrays.insert(ArrayConstraint::Write { array, value });
                        }
                    }
                }
            }
            InsnType::Cmp => {
                if let Some(result) = &instruction.result {
                    self.add_register_bound(result, BoundKind::Exact, ArgType::INT);
                }
                self.constrain_peer_arguments(instruction);
            }
            InsnType::If => self.constrain_peer_arguments(instruction),
            InsnType::Switch => {
                if let Some(selector) = instruction.args.first() {
                    self.add_argument_bound(selector, BoundKind::Upper, ArgType::INT);
                }
            }
            InsnType::Arith | InsnType::CompoundAssign => self.constrain_arithmetic(instruction),
            InsnType::Neg | InsnType::Not => self.constrain_unary(instruction),
            InsnType::MonitorEnter | InsnType::MonitorExit => {
                if let Some(lock) = instruction.args.first() {
                    self.add_argument_bound(lock, BoundKind::Upper, ArgType::unknown_object());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn constrain_invoke(&mut self, instruction: &InsnNode) -> Result<(), TypeConstraintError> {
        let reference = instruction.payload.reference.as_deref().ok_or(
            TypeConstraintError::MissingReference {
                offset: instruction.offset,
                instruction: instruction.insn_type,
            },
        )?;
        let MemberReference::Method(method) = reference else {
            return Err(TypeConstraintError::InvalidReferenceKind {
                offset: instruction.offset,
                instruction: instruction.insn_type,
            });
        };
        let invoke_type = instruction
            .payload
            .invoke_type
            .ok_or(TypeConstraintError::MissingInvokeType(instruction.offset))?;
        let is_static = !invoke_type.has_receiver();
        let expected = method.descriptor.parameters.len() + usize::from(!is_static);
        if instruction.args.len() != expected {
            return Err(TypeConstraintError::InvokeArity {
                offset: instruction.offset,
                expected,
                actual: instruction.args.len(),
            });
        }
        let mut arguments = instruction.args.iter();
        if !is_static {
            if let Some(receiver) = arguments.next() {
                self.add_argument_bound(receiver, BoundKind::Upper, method.owner.clone());
            }
        }
        for (argument, parameter) in arguments.zip(&method.descriptor.parameters) {
            self.add_argument_bound(argument, BoundKind::Upper, parameter.clone());
        }
        if let Some(result) = &instruction.result {
            let ty = if instruction.insn_type == InsnType::Constructor {
                if result.ty.is_known() {
                    result.ty.clone()
                } else {
                    method.owner.clone()
                }
            } else {
                method.descriptor.return_type.clone()
            };
            self.add_register_bound(result, BoundKind::Exact, ty);
        }
        Ok(())
    }

    fn constrain_field(&mut self, instruction: &InsnNode) -> Result<(), TypeConstraintError> {
        let reference = instruction.payload.reference.as_deref().ok_or(
            TypeConstraintError::MissingReference {
                offset: instruction.offset,
                instruction: instruction.insn_type,
            },
        )?;
        let MemberReference::Field(field) = reference else {
            return Err(TypeConstraintError::InvalidReferenceKind {
                offset: instruction.offset,
                instruction: instruction.insn_type,
            });
        };
        let field_type = field.field_type.clone();
        match instruction.insn_type {
            InsnType::Iget => {
                if let Some(result) = &instruction.result {
                    self.add_register_bound(result, BoundKind::Exact, field_type);
                }
                if let Some(owner) = instruction.args.first() {
                    self.add_argument_bound(owner, BoundKind::Upper, field.owner.clone());
                }
            }
            InsnType::Iput => {
                if let Some(value) = instruction.args.first() {
                    self.add_argument_bound(value, BoundKind::Upper, field_type);
                }
                if let Some(owner) = instruction.args.get(1) {
                    self.add_argument_bound(owner, BoundKind::Upper, field.owner.clone());
                }
            }
            InsnType::Sget => {
                if let Some(result) = &instruction.result {
                    self.add_register_bound(result, BoundKind::Exact, field_type);
                }
            }
            InsnType::Sput => {
                if let Some(value) = instruction.args.first() {
                    self.add_argument_bound(value, BoundKind::Upper, field_type);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn constrain_array_get(&mut self, instruction: &InsnNode) {
        let Some(result) = &instruction.result else {
            return;
        };
        if result.ty.is_known() {
            if let Some(array) = instruction.args.first() {
                self.add_argument_bound(
                    array,
                    BoundKind::Fallback,
                    ArgType::array(result.ty.clone()),
                );
            }
        }
        if let Some(index) = instruction.args.get(1) {
            self.add_argument_bound(index, BoundKind::Upper, ArgType::INT);
        }
        if let (Some(array), Some(result)) = (
            instruction.args.first().and_then(Self::argument_value),
            SsaVar::from_reg(result),
        ) {
            self.arrays.insert(ArrayConstraint::Read { array, result });
        }
    }

    fn constrain_array_put(&mut self, instruction: &InsnNode) {
        let Some(value) = instruction.args.first() else {
            return;
        };
        if let Some(ty) = value.declared_type().filter(|ty| ty.is_known()) {
            if let Some(array) = instruction.args.get(1) {
                self.add_argument_bound(array, BoundKind::Fallback, ArgType::array(ty.clone()));
            }
        }
        if let Some(index) = instruction.args.get(2) {
            self.add_argument_bound(index, BoundKind::Upper, ArgType::INT);
        }
        if let (Some(array), Some(value)) = (
            instruction.args.get(1).and_then(Self::argument_value),
            Self::argument_value(value),
        ) {
            self.arrays.insert(ArrayConstraint::Write { array, value });
        }
    }

    fn constrain_peer_arguments(&mut self, instruction: &InsnNode) {
        let Some(left) = instruction.args.first() else {
            return;
        };
        let Some(right) = instruction.args.get(1) else {
            return;
        };
        let reference_equality = matches!(instruction.payload.if_op, Some(IfOp::Eq | IfOp::Ne));
        if let Some(ty) = left.declared_type().filter(|ty| ty.is_known()) {
            self.add_peer_bound(right, ty, reference_equality);
        }
        if let Some(ty) = right.declared_type().filter(|ty| ty.is_known()) {
            self.add_peer_bound(left, ty, reference_equality);
        }
    }

    fn add_peer_bound(&mut self, argument: &InsnArg, peer: &ArgType, reference_equality: bool) {
        if reference_equality {
            let domain = if peer.is_reference() {
                ArgType::unknown_object()
            } else {
                peer.clone()
            };
            self.add_argument_bound(argument, BoundKind::Domain, domain);
        } else {
            self.add_argument_bound(argument, BoundKind::Upper, peer.clone());
        }
    }

    fn constrain_arithmetic(&mut self, instruction: &InsnNode) {
        let result_type = instruction
            .result
            .as_ref()
            .map(|result| result.ty.clone())
            .filter(ArgType::is_known)
            .or_else(|| {
                instruction
                    .args
                    .iter()
                    .filter_map(InsnArg::declared_type)
                    .find(|ty| ty.is_known())
                    .cloned()
            });
        let Some(result_type) = result_type else {
            return;
        };
        if result_type == ArgType::INT
            && matches!(
                instruction.payload.arith_op,
                Some(ArithOp::And | ArithOp::Or | ArithOp::Xor)
            )
        {
            self.constrain_narrow_bitwise(instruction);
            return;
        }
        if let Some(result) = &instruction.result {
            self.add_register_bound(result, BoundKind::Lower, result_type.clone());
        }
        for (index, argument) in instruction.args.iter().enumerate() {
            let expected = if index == 1
                && matches!(
                    instruction.payload.arith_op,
                    Some(ArithOp::Shl | ArithOp::Shr | ArithOp::Ushr)
                ) {
                ArgType::INT
            } else {
                result_type.clone()
            };
            self.add_argument_bound(argument, BoundKind::Upper, expected);
        }
    }

    fn constrain_narrow_bitwise(&mut self, instruction: &InsnNode) {
        let Some(result) = instruction.result.as_ref().and_then(SsaVar::from_reg) else {
            return;
        };
        let mut has_value_source = false;
        for argument in &instruction.args {
            if let Some(source) = Self::argument_value(argument) {
                self.flows.insert((source, result, FlowKind::Bitwise));
                has_value_source = true;
                continue;
            }
            if matches!(argument, InsnArg::Lit(literal) if !matches!(literal.value, 0 | 1)) {
                self.add_value_bound(result, BoundKind::Lower, ArgType::INT);
            }
        }
        if !has_value_source {
            self.add_value_bound(result, BoundKind::Lower, ArgType::INT);
        }
    }

    fn constrain_unary(&mut self, instruction: &InsnNode) {
        let Some(result) = &instruction.result else {
            return;
        };
        self.add_register_bound(result, BoundKind::Lower, result.ty.clone());
        if let Some(source) = instruction.args.first() {
            self.add_argument_bound(source, BoundKind::Upper, result.ty.clone());
        }
    }

    fn observe_arguments(&mut self, instruction: &InsnNode) {
        InstructionTree::visit_args(instruction, &mut TypeObservation { graph: self });
    }

    fn add_argument_bound(&mut self, argument: &InsnArg, kind: BoundKind, ty: ArgType) {
        if let Some(register) = argument.as_register() {
            self.add_register_bound(register, kind, ty);
        }
    }

    fn add_register_bound(&mut self, register: &RegisterArg, kind: BoundKind, ty: ArgType) {
        if let Some(value) = SsaVar::from_reg(register) {
            self.add_value_bound(value, kind, ty);
        }
    }

    fn observe_register_type(&mut self, register: &RegisterArg) {
        if let Some(value) = SsaVar::from_reg(register) {
            self.bounds.for_value(value).observe(&register.ty);
        }
    }

    fn add_value_bound(&mut self, value: SsaVar, kind: BoundKind, ty: ArgType) {
        self.bounds
            .for_value(value)
            .insert(TypeBound::new(kind, ty));
    }

    fn argument_value(argument: &InsnArg) -> Option<SsaVar> {
        match argument {
            InsnArg::Reg(register) => SsaVar::from_reg(register),
            InsnArg::Wrapped(instruction) => instruction.result.as_ref().and_then(SsaVar::from_reg),
            InsnArg::Lit(_) => None,
        }
    }

    fn constant_fallback(value: &InsnArg) -> Option<ArgType> {
        let ty = match value {
            InsnArg::Lit(literal) => &literal.ty,
            InsnArg::Wrapped(instruction) if instruction.insn_type == InsnType::Const => {
                &instruction.result.as_ref()?.ty
            }
            InsnArg::Wrapped(instruction) if instruction.insn_type == InsnType::ConstStr => {
                return Some(ArgType::string());
            }
            InsnArg::Reg(_) | InsnArg::Wrapped(_) => return None,
        };
        if ty.is_known() {
            return Some(ty.clone());
        }
        match ty {
            ArgType::Unknown(categories)
                if categories.iter().any(|category| {
                    matches!(
                        category,
                        crate::ir::PrimitiveType::Boolean
                            | crate::ir::PrimitiveType::Byte
                            | crate::ir::PrimitiveType::Short
                            | crate::ir::PrimitiveType::Char
                            | crate::ir::PrimitiveType::Int
                            | crate::ir::PrimitiveType::Float
                    )
                }) =>
            {
                Some(ArgType::INT)
            }
            ArgType::Unknown(categories)
                if categories.iter().any(|category| {
                    matches!(
                        category,
                        crate::ir::PrimitiveType::Long | crate::ir::PrimitiveType::Double
                    )
                }) =>
            {
                Some(ArgType::LONG)
            }
            ArgType::Unknown(categories)
                if categories.iter().any(|category| {
                    matches!(
                        category,
                        crate::ir::PrimitiveType::Object | crate::ir::PrimitiveType::Array
                    )
                }) =>
            {
                Some(ArgType::object("java/lang/Object"))
            }
            _ => None,
        }
    }
}

struct TypeObservation<'a, 'graph> {
    graph: &'a mut TypeConstraintGraph<'graph>,
}

impl InstructionVisitor for TypeObservation<'_, '_> {
    fn visit_register(&mut self, register: &RegisterArg) {
        self.graph.observe_register_type(register);
    }

    fn visit_instruction(&mut self, instruction: &InsnNode) {
        if let Some(result) = &instruction.result {
            self.graph.observe_register_type(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concrete_register_observations_are_weak_type_evidence() {
        let value = SsaVar::new(2, 1);
        let values = SsaValueGraph::default();
        let mut graph = TypeConstraintGraph::new(&values);
        graph.bounds.for_value(value);
        let register = RegisterArg::new_ssa(2, 1, ArgType::INT);

        graph.observe_register_type(&register);

        let bounds = graph
            .bounds
            .additional
            .get(&value)
            .expect("observed value bounds");
        assert!(bounds.contains(&TypeBound::new(BoundKind::Domain, ArgType::INT)));
        assert!(bounds.contains(&TypeBound::new(BoundKind::Fallback, ArgType::INT)));
    }

    #[test]
    fn narrow_bitwise_operations_preserve_boolean_type_flow() {
        let left = SsaVar::new(0, 1);
        let right = SsaVar::new(1, 1);
        let result = SsaVar::new(2, 1);
        let values = SsaValueGraph::default();
        let mut graph = TypeConstraintGraph::new(&values);
        for value in [left, right, result] {
            graph.bounds.for_value(value);
        }
        let instruction = InsnNode::arith(
            ArithOp::Or,
            RegisterArg::new_ssa(2, 1, ArgType::INT),
            InsnArg::reg_ssa(0, 1, ArgType::BOOLEAN),
            InsnArg::reg_ssa(1, 1, ArgType::BOOLEAN),
            ArgType::INT,
        );

        graph.constrain_arithmetic(&instruction);

        assert!(graph.flows.contains(&(left, result, FlowKind::Bitwise)));
        assert!(graph.flows.contains(&(right, result, FlowKind::Bitwise)));
        assert!(!graph.bounds.additional.get(&result).is_some_and(|bounds| {
            bounds.contains(&TypeBound::new(BoundKind::Lower, ArgType::INT))
        }));
    }

    #[test]
    fn physical_array_write_type_is_fallback_evidence() {
        let array = SsaVar::new(4, 1);
        let value_type = ArgType::object("android/content/pm/ActivityInfo");
        let values = SsaValueGraph::default();
        let mut graph = TypeConstraintGraph::new(&values);
        graph.bounds.for_value(array);
        let instruction = InsnNode::aput(
            InsnArg::reg_ssa(1, 0, value_type.clone()),
            InsnArg::reg_ssa(4, 1, ArgType::unknown_object()),
            InsnArg::lit(0, ArgType::INT),
        );

        graph.constrain_array_put(&instruction);

        let bounds = graph.bounds.additional.get(&array).expect("array bounds");
        assert!(bounds.contains(&TypeBound::new(
            BoundKind::Fallback,
            ArgType::array(value_type.clone()),
        )));
        assert!(!bounds.contains(&TypeBound::new(
            BoundKind::Upper,
            ArgType::array(value_type),
        )));
    }

    #[test]
    fn move_exception_is_constrained_to_throwable() {
        let value = SsaVar::new(3, 14);
        let values = SsaValueGraph::default();
        let mut graph = TypeConstraintGraph::new(&values);
        graph.bounds.for_value(value);
        let instruction = InsnNode::move_exception(RegisterArg::new_ssa(
            value.reg_num,
            value.version,
            ArgType::unknown(),
        ));

        graph
            .constrain_instruction(&instruction, &CFG::new("handler"), &BTreeMap::new())
            .expect("move-exception constraint");

        assert!(graph.bounds.additional.get(&value).is_some_and(|bounds| {
            bounds.contains(&TypeBound::new(BoundKind::Upper, ArgType::throwable()))
        }));
    }
}
