//! Ordered predicate atoms for the proofs used by guard merging and distribution.

use foldhash::HashMap;
use smallvec::SmallVec;

use crate::ir::{
    analysis::SsaVar, IfOp, InsnType, InstructionId, RegisterArg, SemanticExpression,
    SemanticOperation,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AtomIdentity {
    Source,
    Distribution,
}

impl AtomIdentity {
    pub(super) fn equivalent(self, left: &SemanticOperation, right: &SemanticOperation) -> bool {
        if self == Self::Distribution && left.id == right.id {
            return true;
        }
        let mut pending = SmallVec::new();
        if !Self::pair_operands(left, right, &mut pending) {
            return false;
        }
        while let Some((left, right)) = pending.pop() {
            match (left, right) {
                (SemanticExpression::Register(left), SemanticExpression::Register(right))
                    if self.register(left) == self.register(right) => {}
                (SemanticExpression::Literal(left), SemanticExpression::Literal(right))
                    if left == right => {}
                (SemanticExpression::Operation(left), SemanticExpression::Operation(right))
                    if Self::pair_operands(left, right, &mut pending) => {}
                _ => return false,
            }
        }
        true
    }

    fn pair_operands<'a>(
        left: &'a SemanticOperation,
        right: &'a SemanticOperation,
        pending: &mut SmallVec<[(&'a SemanticExpression, &'a SemanticExpression); 2]>,
    ) -> bool {
        if !left.operation_equivalent(right) || left.operands().len() != right.operands().len() {
            return false;
        }
        pending.extend(left.operands().iter().zip(right.operands()));
        match (left.compound_target(), right.compound_target()) {
            (Some(left), Some(right)) => pending.push((left, right)),
            (None, None) => {}
            _ => return false,
        }
        true
    }

    fn register(self, register: &RegisterArg) -> OperandShape {
        if self == Self::Source {
            if let Some(variable) = register.code_var {
                return OperandShape::Source(variable);
            }
        }
        match SsaVar::from_reg(register) {
            Some(variable) => OperandShape::Ssa(variable),
            None => OperandShape::Register(register.reg_num, register.code_var),
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
enum OperandShape {
    Missing,
    Source(u32),
    Ssa(SsaVar),
    Register(u32, Option<u32>),
    Literal(i64),
    Operation(InsnType),
    Select,
}

impl OperandShape {
    fn of(mode: AtomIdentity, expression: Option<&SemanticExpression>) -> Self {
        match expression {
            None => Self::Missing,
            Some(SemanticExpression::Register(register)) => mode.register(register),
            Some(SemanticExpression::Literal(literal)) => Self::Literal(literal.value),
            Some(SemanticExpression::Operation(operation)) => Self::Operation(operation.insn_type),
            Some(SemanticExpression::Select { .. }) => Self::Select,
        }
    }
}

/// Necessary conditions for operand equivalence. Each bucket is searched in
/// discovery order using the complete comparison.
#[derive(PartialEq, Eq, Hash)]
struct AtomShape {
    opcode: InsnType,
    condition: Option<IfOp>,
    arity: usize,
    first: OperandShape,
    second: OperandShape,
}

impl AtomShape {
    fn of(mode: AtomIdentity, operation: &SemanticOperation) -> Self {
        let operands = operation.operands();
        Self {
            opcode: operation.insn_type,
            condition: operation.payload.if_op,
            arity: operands.len(),
            first: OperandShape::of(mode, operands.first()),
            second: OperandShape::of(mode, operands.get(1)),
        }
    }
}

pub(super) struct AtomClasses<'a> {
    mode: AtomIdentity,
    representatives: Vec<&'a SemanticOperation>,
    by_id: HashMap<InstructionId, usize>,
    by_shape: HashMap<AtomShape, SmallVec<[usize; 1]>>,
}

impl<'a> AtomClasses<'a> {
    const INDEX_AT: usize = 8;

    pub(super) fn new(mode: AtomIdentity) -> Self {
        Self {
            mode,
            representatives: Vec::new(),
            by_id: HashMap::default(),
            by_shape: HashMap::default(),
        }
    }

    pub(super) fn len(&self) -> usize {
        self.representatives.len()
    }

    pub(super) fn intern(&mut self, operation: &'a SemanticOperation) -> usize {
        let next = self.len();
        if next < Self::INDEX_AT {
            if let Some(index) = self
                .representatives
                .iter()
                .position(|candidate| self.mode.equivalent(candidate, operation))
            {
                return index;
            }
            self.representatives.push(operation);
            if self.len() == Self::INDEX_AT {
                for (index, candidate) in self.representatives.iter().enumerate() {
                    if self.mode == AtomIdentity::Distribution {
                        self.by_id.entry(candidate.id).or_insert(index);
                    }
                    self.by_shape
                        .entry(AtomShape::of(self.mode, candidate))
                        .or_default()
                        .push(index);
                }
            }
            return next;
        }
        // Distribution also identifies operations by original instruction ID,
        // even when their operands differ. An earlier structural representative
        // still takes precedence over the first same-ID representative.
        let by_id = self.by_id.get(&operation.id).copied().unwrap_or(next);
        let candidates = self
            .by_shape
            .entry(AtomShape::of(self.mode, operation))
            .or_default();
        for &index in candidates.iter().take_while(|&&index| index < by_id) {
            if self.mode.equivalent(self.representatives[index], operation) {
                return index;
            }
        }
        if by_id == next {
            candidates.push(next);
            self.representatives.push(operation);
            if self.mode == AtomIdentity::Distribution {
                self.by_id.insert(operation.id, next);
            }
        }
        by_id
    }
}
