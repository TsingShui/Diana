//! Expressions owned by structured Semantic IR.
//!
//! Low-level `InsnArg::Wrapped` trees end at `SemanticFactory`.  From that
//! boundary onward every value, including control-dependent selections, uses
//! this representation.

use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};

use crate::ir::{
    analysis::{InstructionEffects, SourceEvaluationError},
    ArgType, InsnArg, InsnNode, InsnPayload, InsnType, InstructionId, LiteralArg, MemberReference,
    RegisterArg,
};

use super::{SemanticFoldError, SemanticPredicate};

#[derive(Debug, Clone)]
pub enum SemanticExpression {
    Register(RegisterArg),
    Literal(LiteralArg),
    Operation(SemanticOperation),
    Select {
        condition: SemanticPredicate,
        when_true: Box<SemanticExpression>,
        when_false: Box<SemanticExpression>,
    },
}

pub(super) enum RegisterSite<'a> {
    Operation(&'a mut SemanticOperation),
    Expression(&'a mut SemanticExpression),
    Predicate(&'a mut SemanticPredicate),
}

impl RegisterSite<'_> {
    pub(super) fn bind(self, lookup: &impl Fn(&RegisterArg) -> Option<u32>) {
        let mut next = self;
        let mut pending = smallvec::SmallVec::<[Self; 8]>::new();
        loop {
            match next {
                Self::Expression(SemanticExpression::Register(register)) => {
                    register.code_var = lookup(register)
                }
                Self::Expression(SemanticExpression::Literal(_)) => {}
                Self::Expression(SemanticExpression::Operation(op))
                | Self::Predicate(SemanticPredicate::Test(op))
                | Self::Operation(op) => {
                    // Variable annotations are independent of cached operand
                    // shape and effects. Detach a shared operation only when
                    // it has a register or child requiring annotation.
                    if op.result.is_some()
                        || !op.operands.is_empty()
                        || op.compound_target.is_some()
                    {
                        let data = Arc::make_mut(&mut op.0);
                        if let Some(result) = &mut data.result {
                            result.code_var = lookup(result);
                        }
                        for value in data
                            .operands
                            .iter_mut()
                            .chain(data.compound_target.as_deref_mut())
                        {
                            match value {
                                SemanticExpression::Register(register) => {
                                    register.code_var = lookup(register)
                                }
                                SemanticExpression::Literal(_) => {}
                                _ => pending.push(Self::Expression(value)),
                            }
                        }
                    }
                }
                Self::Expression(SemanticExpression::Select {
                    condition,
                    when_true,
                    when_false,
                }) => {
                    pending.push(Self::Expression(when_false));
                    pending.push(Self::Expression(when_true));
                    next = Self::Predicate(condition);
                    continue;
                }
                Self::Predicate(SemanticPredicate::False | SemanticPredicate::True) => {}
                Self::Predicate(SemanticPredicate::Not(inner)) => {
                    next = Self::Predicate(inner);
                    continue;
                }
                Self::Predicate(SemanticPredicate::And(terms) | SemanticPredicate::Or(terms)) => {
                    pending.extend(terms.iter_mut().rev().map(Self::Predicate));
                }
            }
            let Some(site) = pending.pop() else { break };
            next = site;
        }
    }
}

/// One source-level operation with semantic expressions as its operands.
///
/// `instruction.args` and `instruction.payload.compound_target` are always
/// empty. Keeping the opcode payload separate from operands makes it
/// impossible for later stages to accidentally reintroduce wrapped CFG trees.
#[derive(Clone)]
pub struct SemanticOperation(Arc<SemanticOperationData>);

/// Region contraction can place one operation at several semantic sites.
/// Those sites share the operation until a transform edits one of them.
#[derive(Clone)]
pub struct SemanticOperationData {
    pub id: InstructionId,
    pub insn_type: InsnType,
    pub result: Option<RegisterArg>,
    pub offset: u32,
    pub payload: InsnPayload,
    operands: Vec<SemanticExpression>,
    compound_target: Option<Box<SemanticExpression>>,
    operand_effects: CachedOperandEffects,
    operand_shape: CachedOperandShape,
}

/// All readers of a shared operation see the same immutable operands. Only
/// the effect bits are published. No other memory depends on their ordering.
struct CachedOperandEffects(AtomicU8);

impl CachedOperandEffects {
    const EMPTY: u8 = u8::MAX;

    fn new() -> Self {
        Self(AtomicU8::new(Self::EMPTY))
    }

    fn get(&self) -> Option<InstructionEffects> {
        let bits = self.0.load(Ordering::Relaxed);
        (bits != Self::EMPTY).then(|| InstructionEffects::from_bits(bits))
    }

    fn set(&self, effects: InstructionEffects) {
        self.0.store(effects.bits(), Ordering::Relaxed);
    }
}

impl Clone for CachedOperandEffects {
    fn clone(&self) -> Self {
        Self(AtomicU8::new(self.0.load(Ordering::Relaxed)))
    }
}

/// Without a selection, an expression cannot introduce any new predicate
/// symbols. Keep invalid evaluation layouts distinct so the ordinary
/// collector can still report the first error in source evaluation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum OperandShape {
    Plain = 0,
    InvalidEvaluation = 1,
    Select = 2,
}

struct CachedOperandShape(AtomicU8);

impl CachedOperandShape {
    const EMPTY: u8 = u8::MAX;

    fn get(&self) -> Option<OperandShape> {
        match self.0.load(Ordering::Relaxed) {
            0 => Some(OperandShape::Plain),
            1 => Some(OperandShape::InvalidEvaluation),
            2 => Some(OperandShape::Select),
            _ => None,
        }
    }

    fn set(&self, shape: OperandShape) {
        self.0.store(shape as u8, Ordering::Relaxed);
    }
}

impl Clone for CachedOperandShape {
    fn clone(&self) -> Self {
        Self(AtomicU8::new(self.0.load(Ordering::Relaxed)))
    }
}

impl Drop for SemanticOperationData {
    fn drop(&mut self) {
        if !self
            .operands
            .iter()
            .chain(self.compound_target.as_deref())
            .any(|value| {
                matches!(
                    value,
                    SemanticExpression::Operation(_) | SemanticExpression::Select { .. }
                )
            })
        {
            return;
        }

        // Dismantle nested last-owner operations on the heap stack. A shared
        // operation's last owner will release its children in the same way.
        let mut expressions = std::mem::take(&mut self.operands);
        expressions.extend(self.compound_target.take().map(|value| *value));
        let mut predicates = Vec::new();
        while let Some(expression) = expressions.pop() {
            match expression {
                SemanticExpression::Operation(operation) => {
                    Self::release(operation, &mut expressions)
                }
                SemanticExpression::Select {
                    condition,
                    when_true,
                    when_false,
                } => {
                    expressions.push(*when_true);
                    expressions.push(*when_false);
                    predicates.push(condition);
                }
                SemanticExpression::Register(_) | SemanticExpression::Literal(_) => {}
            }
            while let Some(predicate) = predicates.pop() {
                match predicate {
                    SemanticPredicate::Test(operation) => {
                        Self::release(operation, &mut expressions)
                    }
                    SemanticPredicate::Not(inner) => predicates.push(*inner),
                    SemanticPredicate::And(mut terms) | SemanticPredicate::Or(mut terms) => {
                        predicates.append(&mut terms);
                    }
                    SemanticPredicate::True | SemanticPredicate::False => {}
                }
            }
        }
    }
}

impl SemanticOperationData {
    fn release(operation: SemanticOperation, expressions: &mut Vec<SemanticExpression>) {
        if let Some(mut data) = Arc::into_inner(operation.0) {
            expressions.append(&mut data.operands);
            expressions.extend(data.compound_target.take().map(|value| *value));
        }
    }
}

impl std::ops::Deref for SemanticOperation {
    type Target = SemanticOperationData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SemanticOperation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let data = Arc::make_mut(&mut self.0);
        *data.operand_effects.0.get_mut() = CachedOperandEffects::EMPTY;
        *data.operand_shape.0.get_mut() = CachedOperandShape::EMPTY;
        data
    }
}

impl std::fmt::Debug for SemanticOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticOperation")
            .field("id", &self.id)
            .field("insn_type", &self.insn_type)
            .field("result", &self.result)
            .field("offset", &self.offset)
            .field("payload", &self.payload)
            .field("operands", &self.operands)
            .field("compound_target", &self.compound_target)
            .finish()
    }
}

enum EffectTask<'a> {
    Expression(&'a SemanticExpression),
    Predicate(&'a SemanticPredicate),
    Operands(&'a SemanticOperation),
    Finish(&'a SemanticOperation, InstructionEffects),
}

/// Evaluate operations, selections and predicates on one stack. Completing an
/// operation records only its operands' effects. A predicate suppresses its
/// own control effect, and edge-copy handling can override the root separately.
struct EffectWalk;

impl EffectWalk {
    fn run(first: EffectTask<'_>) -> InstructionEffects {
        let mut tasks = smallvec::SmallVec::<[EffectTask; 8]>::new();
        let mut task = first;
        let mut effects = InstructionEffects::PURE;
        loop {
            match task {
                EffectTask::Expression(SemanticExpression::Operation(operation)) => {
                    effects = effects.join(operation.direct_effects());
                    task = EffectTask::Operands(operation);
                    continue;
                }
                EffectTask::Expression(SemanticExpression::Select {
                    condition,
                    when_true,
                    when_false,
                }) => {
                    tasks.push(EffectTask::Expression(when_false));
                    tasks.push(EffectTask::Expression(when_true));
                    task = EffectTask::Predicate(condition);
                    continue;
                }
                EffectTask::Predicate(SemanticPredicate::Test(operation)) => {
                    effects = effects.join(operation.direct_effects().without_control());
                    task = EffectTask::Operands(operation);
                    continue;
                }
                EffectTask::Predicate(SemanticPredicate::Not(inner)) => {
                    task = EffectTask::Predicate(inner);
                    continue;
                }
                EffectTask::Predicate(
                    SemanticPredicate::And(terms) | SemanticPredicate::Or(terms),
                ) => {
                    tasks.extend(terms.iter().rev().map(EffectTask::Predicate));
                }
                EffectTask::Operands(operation) => {
                    if let Some(cached) = operation.operand_effects.get() {
                        effects = effects.join(cached);
                    } else {
                        let mut nested = operation
                            .operands
                            .iter()
                            .chain(operation.compound_target.as_deref())
                            .filter(|child| {
                                matches!(
                                    child,
                                    SemanticExpression::Operation(_)
                                        | SemanticExpression::Select { .. }
                                )
                            });
                        if let Some(first) = nested.next() {
                            tasks.push(EffectTask::Finish(operation, effects));
                            tasks.extend(nested.rev().map(EffectTask::Expression));
                            effects = InstructionEffects::PURE;
                            task = EffectTask::Expression(first);
                            continue;
                        }
                        operation.operand_effects.set(InstructionEffects::PURE);
                    }
                }
                EffectTask::Finish(operation, outer) => {
                    operation.operand_effects.set(effects);
                    effects = outer.join(effects);
                }
                EffectTask::Predicate(SemanticPredicate::True | SemanticPredicate::False)
                | EffectTask::Expression(
                    SemanticExpression::Register(_) | SemanticExpression::Literal(_),
                ) => {}
            }
            let Some(next) = tasks.pop() else {
                return effects;
            };
            task = next;
        }
    }
}

impl SemanticPredicate {
    pub fn effects(&self) -> InstructionEffects {
        EffectWalk::run(EffectTask::Predicate(self))
    }
}

impl SemanticExpression {
    pub fn from_argument(argument: InsnArg) -> Result<Self, SemanticFoldError> {
        let instruction = match argument {
            InsnArg::Reg(register) => return Ok(Self::Register(register)),
            InsnArg::Lit(literal) => return Ok(Self::Literal(literal)),
            InsnArg::Wrapped(instruction) => Arc::unwrap_or_clone(instruction),
        };
        let mut pending = Vec::new();
        Self::schedule_operation(instruction, &mut pending);
        let mut results = Vec::new();
        while let Some(task) = pending.pop() {
            match task {
                ExpressionTask::Argument(InsnArg::Reg(register)) => {
                    results.push(Self::Register(register));
                }
                ExpressionTask::Argument(InsnArg::Lit(literal)) => {
                    results.push(Self::Literal(literal));
                }
                ExpressionTask::Argument(InsnArg::Wrapped(instruction)) => {
                    Self::schedule_operation(Arc::unwrap_or_clone(instruction), &mut pending);
                }
                ExpressionTask::Operation {
                    instruction,
                    operand_count,
                    has_compound_target,
                } => {
                    let child_count = operand_count + usize::from(has_compound_target);
                    let start = results
                        .len()
                        .checked_sub(child_count)
                        .ok_or(SemanticFoldError::MalformedWorkStack)?;
                    let (operands, compound_target) = {
                        let mut children = results.drain(start..);
                        let operands = children.by_ref().take(operand_count).collect();
                        let compound_target = if has_compound_target {
                            Some(
                                children
                                    .next()
                                    .ok_or(SemanticFoldError::MalformedWorkStack)?,
                            )
                        } else {
                            None
                        };
                        (operands, compound_target)
                    };
                    results.push(Self::Operation(SemanticOperation::from_parts(
                        instruction,
                        operands,
                        compound_target,
                    )));
                }
            }
        }
        if results.len() != 1 {
            return Err(SemanticFoldError::MalformedWorkStack);
        }
        results.pop().ok_or(SemanticFoldError::MalformedWorkStack)
    }

    pub fn operation(instruction: InsnNode) -> Result<Self, SemanticFoldError> {
        SemanticOperation::from_instruction(instruction).map(Self::Operation)
    }

    fn schedule_operation(instruction: InsnNode, pending: &mut Vec<ExpressionTask>) {
        let mut instruction = instruction;
        let operands = std::mem::take(&mut instruction.args);
        let compound_target = instruction.payload.compound_target.take();
        pending.push(ExpressionTask::Operation {
            instruction,
            operand_count: operands.len(),
            has_compound_target: compound_target.is_some(),
        });
        pending.extend(
            operands
                .into_iter()
                .chain(compound_target.map(|target| *target))
                .rev()
                .map(ExpressionTask::Argument),
        );
    }

    pub fn select(
        condition: SemanticPredicate,
        when_true: SemanticExpression,
        when_false: SemanticExpression,
    ) -> Self {
        Self::Select {
            condition,
            when_true: Box::new(when_true),
            when_false: Box::new(when_false),
        }
    }

    pub fn declared_type(&self) -> Option<&ArgType> {
        match self {
            Self::Register(register) => Some(&register.ty),
            Self::Literal(literal) => Some(&literal.ty),
            Self::Operation(operation) => operation.result.as_ref().map(|result| &result.ty),
            Self::Select { when_true, .. } => when_true.declared_type(),
        }
    }

    pub fn as_register(&self) -> Option<&RegisterArg> {
        match self {
            Self::Register(register) => Some(register),
            Self::Literal(_) | Self::Operation(_) | Self::Select { .. } => None,
        }
    }

    pub fn as_operation(&self) -> Option<&SemanticOperation> {
        match self {
            Self::Operation(operation) => Some(operation),
            Self::Register(_) | Self::Literal(_) | Self::Select { .. } => None,
        }
    }

    pub fn literal_value(&self) -> Option<i64> {
        let mut current = self;
        loop {
            match current {
                Self::Literal(literal) => return Some(literal.value),
                Self::Operation(operation)
                    if matches!(
                        operation.insn_type,
                        crate::ir::InsnType::Const | crate::ir::InsnType::Move
                    ) && operation.operands.len() == 1 =>
                {
                    current = &operation.operands[0];
                }
                Self::Register(_) | Self::Operation(_) | Self::Select { .. } => return None,
            }
        }
    }

    fn stable_leaf(&self) -> &Self {
        let mut expression = self;
        while let Self::Operation(operation) = expression {
            if !matches!(
                operation.insn_type,
                crate::ir::InsnType::Const | crate::ir::InsnType::Move
            ) || operation.operands.len() != 1
            {
                break;
            }
            expression = &operation.operands[0];
        }
        expression
    }

    pub fn same_stable_value(&self, other: &Self) -> bool {
        match (self.stable_leaf(), other.stable_leaf()) {
            (Self::Register(left), Self::Register(right)) => {
                match (left.code_var, right.code_var) {
                    (Some(left), Some(right)) => left == right,
                    _ => left.reg_num == right.reg_num && left.ssa_version == right.ssa_version,
                }
            }
            (Self::Literal(left), Self::Literal(right)) => left == right,
            _ => false,
        }
    }

    pub fn effects(&self) -> InstructionEffects {
        match self {
            Self::Register(_) | Self::Literal(_) => InstructionEffects::PURE,
            Self::Operation(operation) => operation.effects(),
            Self::Select { .. } => EffectWalk::run(EffectTask::Expression(self)),
        }
    }
}

impl SemanticOperation {
    pub(crate) fn string_literal(value: impl Into<crate::ir::Utf16String>) -> Self {
        let mut instruction = InsnNode::new(InsnType::ConstStr, 0);
        instruction.payload.string_value = Some(Box::new(value.into()));
        Self::from_parts(instruction, Vec::new(), None)
    }
    pub fn from_instruction(mut instruction: InsnNode) -> Result<Self, SemanticFoldError> {
        let args = std::mem::take(&mut instruction.args);
        Self::from_instruction_args(instruction, args.into_iter())
    }

    pub(crate) fn from_borrowed_instruction(
        instruction: &InsnNode,
    ) -> Result<Self, SemanticFoldError> {
        let header = InsnNode {
            id: instruction.id,
            insn_type: instruction.insn_type,
            result: instruction.result.clone(),
            args: Vec::new(),
            offset: instruction.offset,
            payload: instruction.payload.clone(),
        };
        Self::from_instruction_args(header, instruction.args.iter().cloned())
    }

    fn from_instruction_args(
        mut instruction: InsnNode,
        args: impl ExactSizeIterator<Item = InsnArg>,
    ) -> Result<Self, SemanticFoldError> {
        let mut operands = Vec::with_capacity(args.len());
        for argument in args {
            operands.push(SemanticExpression::from_argument(argument)?);
        }
        let target = instruction
            .payload
            .compound_target
            .take()
            .map(|target| SemanticExpression::from_argument(*target))
            .transpose()?;
        Ok(Self::from_parts(instruction, operands, target))
    }

    fn shape_from_children(&self) -> Option<OperandShape> {
        let mut valid = self.evaluation_operands_iter().is_ok();
        let mut complete = true;
        for child in self.operands.iter().chain(self.compound_target.as_deref()) {
            match child {
                SemanticExpression::Register(_) | SemanticExpression::Literal(_) => {}
                SemanticExpression::Select { .. } => return Some(OperandShape::Select),
                SemanticExpression::Operation(operation) => match operation.operand_shape.get() {
                    Some(OperandShape::Select) => return Some(OperandShape::Select),
                    Some(OperandShape::InvalidEvaluation) => valid = false,
                    Some(OperandShape::Plain) => {}
                    None => complete = false,
                },
            }
        }
        complete.then_some(if valid {
            OperandShape::Plain
        } else {
            OperandShape::InvalidEvaluation
        })
    }

    pub(crate) fn operand_shape(&self) -> OperandShape {
        if let Some(shape) = self.operand_shape.get() {
            return shape;
        }
        let mut parents = smallvec::SmallVec::<[&SemanticOperation; 8]>::new();
        let mut current = self;
        loop {
            if let Some(shape) = current
                .operand_shape
                .get()
                .or_else(|| current.shape_from_children())
            {
                current.operand_shape.set(shape);
                let Some(parent) = parents.pop() else {
                    return shape;
                };
                current = parent;
            } else {
                // Process all missing children before the parent's next visit.
                // A shared child is evaluated at most once for an immutable tree.
                parents.push(current);
                let mut missing = current
                    .operands
                    .iter()
                    .chain(current.compound_target.as_deref())
                    .filter_map(|child| match child {
                        SemanticExpression::Operation(child)
                            if child.operand_shape.get().is_none() =>
                        {
                            Some(child)
                        }
                        _ => None,
                    });
                let Some(first) = missing.next() else {
                    // Another reader may have published the last child.
                    parents.pop();
                    continue;
                };
                parents.extend(missing);
                current = first;
            }
        }
    }

    pub(crate) fn contains_select(&self) -> bool {
        self.operand_shape() == OperandShape::Select
    }

    pub fn operands(&self) -> &[SemanticExpression] {
        &self.operands
    }

    pub(crate) fn children_mut(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = &mut SemanticExpression> {
        let data = std::ops::DerefMut::deref_mut(self);
        data.operands
            .iter_mut()
            .chain(data.compound_target.as_deref_mut())
    }

    pub fn operands_mut(&mut self) -> &mut [SemanticExpression] {
        &mut self.operands
    }

    pub fn compound_target(&self) -> Option<&SemanticExpression> {
        self.compound_target.as_deref()
    }

    pub fn compound_target_mut(&mut self) -> Option<&mut SemanticExpression> {
        self.compound_target.as_deref_mut()
    }

    pub fn operation_equivalent(&self, other: &Self) -> bool {
        self.insn_type == other.insn_type && self.payload.operation_equivalent(&other.payload)
    }

    pub fn discard_result(&mut self) {
        self.result = None;
    }

    pub(crate) fn rewrite_kind(
        mut self,
        kind: InsnType,
        operands: Vec<SemanticExpression>,
    ) -> Self {
        self.insn_type = kind;
        self.operands = operands;
        self.compound_target = None;
        self
    }

    pub(crate) fn set_result_type(&mut self, ty: ArgType) -> bool {
        let Some(result) = &mut self.result else {
            return false;
        };
        result.ty = ty;
        true
    }

    pub(crate) fn set_unary_operator(&mut self, operator: crate::ir::UnaryOp) {
        self.payload.unary_op = Some(operator);
    }

    pub(crate) fn set_comparison_operator(&mut self, operator: crate::ir::IfOp) {
        self.payload.if_op = Some(operator);
    }

    pub fn effects(&self) -> InstructionEffects {
        self.direct_effects().join(self.nested_effects())
    }

    pub fn effects_ignoring_edge_copy(&self) -> InstructionEffects {
        self.direct_effects_with_edge_copy(false)
            .join(self.nested_effects())
    }

    fn nested_effects(&self) -> InstructionEffects {
        self.operand_effects
            .get()
            .unwrap_or_else(|| EffectWalk::run(EffectTask::Operands(self)))
    }

    pub fn direct_effects(&self) -> InstructionEffects {
        self.direct_effects_with_edge_copy(self.payload.edge_copy)
    }

    pub(crate) fn direct_effects_with_edge_copy(&self, edge_copy: bool) -> InstructionEffects {
        InstructionEffects::direct(
            self.insn_type,
            self.result.as_ref(),
            &self.payload,
            edge_copy,
        )
    }

    pub fn conversion_type(&self) -> Option<&ArgType> {
        match self.insn_type {
            InsnType::Cast => self.payload.cast_type.as_deref(),
            InsnType::CheckCast => self
                .payload
                .class_type
                .as_deref()
                .or(self.payload.cast_type.as_deref()),
            _ => None,
        }
        .or_else(|| self.result.as_ref().map(|result| &result.ty))
    }

    /// Runtime class allocated by a recovered constructor operation.
    ///
    /// Optimizers may replace an empty subclass initializer with a direct call
    /// to its superclass initializer. The invoked method owner therefore does
    /// not necessarily identify the object produced by `new-instance`.
    pub fn allocation_type(&self) -> Option<&ArgType> {
        if self.insn_type != InsnType::Constructor {
            return None;
        }
        self.payload
            .class_type
            .as_deref()
            .or_else(|| self.result.as_ref().map(|result| &result.ty))
            .or(match self.payload.reference.as_deref() {
                Some(MemberReference::Method(method)) => Some(&method.owner),
                _ => None,
            })
    }

    pub fn evaluation_operands(&self) -> Result<Vec<&SemanticExpression>, SourceEvaluationError> {
        Ok(self.evaluation_operands_iter()?.collect())
    }

    pub(crate) fn evaluation_operands_iter(
        &self,
    ) -> Result<impl DoubleEndedIterator<Item = &SemanticExpression> + Clone, SourceEvaluationError>
    {
        let arity = || SourceEvaluationError::InvalidArity {
            instruction: self.insn_type,
            offset: self.offset,
            actual: self.operands.len(),
        };
        let (first, last) = match self.insn_type {
            InsnType::Constructor => (
                self.operands
                    .get(1..)
                    .ok_or(SourceEvaluationError::MalformedConstructor(self.offset))?,
                None,
            ),
            InsnType::Iput | InsnType::Aput => {
                let expected = if self.insn_type == InsnType::Iput {
                    2
                } else {
                    3
                };
                if self.operands.len() != expected {
                    return Err(arity());
                }
                (&self.operands[1..], self.operands.first())
            }
            InsnType::CompoundAssign => (
                std::slice::from_ref(self.compound_target.as_deref().ok_or_else(arity)?),
                Some(self.operands.last().ok_or_else(arity)?),
            ),
            _ => (self.operands.as_slice(), self.compound_target.as_deref()),
        };
        Ok(first.iter().chain(last))
    }

    pub fn effects_before(
        &self,
        target: crate::ir::InstructionId,
    ) -> Result<Option<InstructionEffects>, SourceEvaluationError> {
        if self.id == target {
            return Ok(Some(InstructionEffects::PURE));
        }
        let mut effects = InstructionEffects::before_arguments(self.insn_type);
        let mut pending = vec![EvaluationTask::Effect(self)];
        pending.extend(
            self.evaluation_operands_iter()?
                .rev()
                .map(EvaluationTask::Expression),
        );
        while let Some(task) = pending.pop() {
            match task {
                EvaluationTask::Expression(SemanticExpression::Operation(operation)) => {
                    if operation.id == target {
                        return Ok(Some(effects));
                    }
                    effects =
                        effects.join(InstructionEffects::before_arguments(operation.insn_type));
                    pending.push(EvaluationTask::Effect(operation));
                    pending.extend(
                        operation
                            .evaluation_operands_iter()?
                            .rev()
                            .map(EvaluationTask::Expression),
                    );
                }
                EvaluationTask::Expression(SemanticExpression::Select { .. }) => return Ok(None),
                EvaluationTask::Expression(
                    SemanticExpression::Register(_) | SemanticExpression::Literal(_),
                ) => {}
                EvaluationTask::Effect(operation) => {
                    effects = effects.join(operation.direct_effects());
                }
            }
        }
        Ok(None)
    }

    pub(crate) fn from_parts(
        instruction: InsnNode,
        operands: Vec<SemanticExpression>,
        compound_target: Option<SemanticExpression>,
    ) -> Self {
        debug_assert!(instruction.args.is_empty());
        debug_assert!(instruction.payload.compound_target.is_none());
        Self(Arc::new(SemanticOperationData {
            id: instruction.id,
            insn_type: instruction.insn_type,
            result: instruction.result,
            offset: instruction.offset,
            payload: instruction.payload,
            operands,
            compound_target: compound_target.map(Box::new),
            operand_effects: CachedOperandEffects::new(),
            operand_shape: CachedOperandShape(AtomicU8::new(CachedOperandShape::EMPTY)),
        }))
    }

    /// Takes the operation's instruction metadata, ordered semantic operands,
    /// and optional compound-assignment target. The instruction contains the
    /// original ID, opcode, result, offset and payload. Its raw argument list
    /// is empty because the operands are already semantic expressions.
    ///
    /// The last owner moves these values directly. If other sites still share
    /// the operation, they retain their operands and instruction metadata.
    pub fn into_parts(
        self,
    ) -> (
        InsnNode,
        Vec<SemanticExpression>,
        Option<SemanticExpression>,
    ) {
        let mut data = Arc::unwrap_or_clone(self.0);
        (
            InsnNode {
                id: data.id,
                insn_type: data.insn_type,
                result: data.result.take(),
                args: Vec::new(),
                offset: data.offset,
                payload: std::mem::take(&mut data.payload),
            },
            std::mem::take(&mut data.operands),
            data.compound_target.take().map(|target| *target),
        )
    }

    pub(crate) fn instruction_with_arguments(&self, arguments: Vec<InsnArg>) -> InsnNode {
        InsnNode {
            id: self.id,
            insn_type: self.insn_type,
            result: self.result.clone(),
            args: arguments,
            offset: self.offset,
            payload: self.payload.clone(),
        }
    }
}

enum ExpressionTask {
    Argument(InsnArg),
    Operation {
        instruction: InsnNode,
        operand_count: usize,
        has_compound_target: bool,
    },
}

enum EvaluationTask<'a> {
    Expression(&'a SemanticExpression),
    Effect(&'a SemanticOperation),
}

#[cfg(test)]
#[path = "../../../tests/internal/operation_effects.rs"]
mod effect_tests;
