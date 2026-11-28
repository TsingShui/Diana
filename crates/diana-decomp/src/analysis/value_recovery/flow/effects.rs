//! Location-sensitive effects used by semantic value scheduling.

use std::{collections::BTreeSet, rc::Rc};

use crate::ir::{
    analysis::InstructionEffects, InsnType, SemanticExpression, SemanticOperation,
    SemanticStatement, SemanticStatementKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MemoryLocation {
    Field(u32),
    Array,
    UnknownHeap,
}

/// A direct operation touches at most one kind of location on each side.
/// Unions need a set only when a compound expression touches distinct fields
/// or both fields and an array. An unknown heap access aliases every location.
#[derive(Debug, Clone, Default)]
#[cfg_attr(test, derive(PartialEq, Eq))]
enum Locations {
    #[default]
    Empty,
    One(MemoryLocation),
    Many(Rc<BTreeSet<MemoryLocation>>),
}

impl Locations {
    fn same_storage(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (Self::One(left), Self::One(right)) => left == right,
            (Self::Many(left), Self::Many(right)) => Rc::ptr_eq(left, right),
            _ => false,
        }
    }

    fn union(self, other: Self) -> Self {
        use Locations::*;
        match (self, other) {
            (Empty, set) | (set, Empty) => set,
            (One(MemoryLocation::UnknownHeap), _) | (_, One(MemoryLocation::UnknownHeap)) => {
                One(MemoryLocation::UnknownHeap)
            }
            (One(a), One(b)) if a == b => One(a),
            (One(a), One(b)) => Many(Rc::new(BTreeSet::from([a, b]))),
            (Many(mut set), One(location)) | (One(location), Many(mut set)) => {
                Rc::make_mut(&mut set).insert(location);
                Many(set)
            }
            (Many(mut a), Many(b)) => {
                if !Rc::ptr_eq(&a, &b) {
                    Rc::make_mut(&mut a).extend(b.iter().copied());
                }
                Many(a)
            }
        }
    }

    fn overlaps(&self, other: &Self) -> bool {
        use Locations::*;
        match (self, other) {
            (Empty, _) | (_, Empty) => false,
            (One(MemoryLocation::UnknownHeap), _) | (_, One(MemoryLocation::UnknownHeap)) => true,
            (One(a), One(b)) => a == b,
            (One(a), Many(set)) | (Many(set), One(a)) => set.contains(a),
            (Many(a), Many(b)) => !a.is_disjoint(b),
        }
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub(super) struct EffectSummary {
    instruction: InstructionEffects,
    reads: Locations,
    writes: Locations,
}

/// Indices live in the same immutable value graph as their use occurrences.
#[derive(Debug, Clone, Copy)]
pub(super) struct EvaluationPrefix(usize);

#[derive(Default)]
pub(super) struct EvaluationPrefixes {
    empty: EffectSummary,
    nonempty: Vec<EffectSummary>,
}

impl EvaluationPrefixes {
    pub(super) fn record(&mut self, prefix: &EffectSummary) -> EvaluationPrefix {
        if prefix.instruction.is_pure()
            && matches!(
                (&prefix.reads, &prefix.writes),
                (Locations::Empty, Locations::Empty)
            )
        {
            return EvaluationPrefix(0);
        }
        // Consecutive operands often have the same prefix. Comparing shared
        // set identities keeps recording constant-time even for many fields.
        if !self.nonempty.last().is_some_and(|last| {
            last.instruction == prefix.instruction
                && last.reads.same_storage(&prefix.reads)
                && last.writes.same_storage(&prefix.writes)
        }) {
            self.nonempty.push(prefix.clone());
        }
        EvaluationPrefix(self.nonempty.len())
    }

    pub(super) fn get(&self, index: EvaluationPrefix) -> &EffectSummary {
        if index.0 == 0 {
            &self.empty
        } else {
            &self.nonempty[index.0 - 1]
        }
    }
}

impl EffectSummary {
    pub(super) fn pure() -> Self {
        Self::default()
    }

    pub(super) fn statement_boundary() -> Self {
        Self {
            instruction: InstructionEffects::CONTROL,
            ..Self::default()
        }
    }

    pub(super) fn synchronization() -> Self {
        Self {
            instruction: InstructionEffects::SYNCHRONIZATION,
            ..Self::default()
        }
    }

    pub(super) fn direct(operation: &SemanticOperation) -> Self {
        Self::with_instruction_effects(operation, operation.direct_effects())
    }

    pub(super) fn include_before_arguments(&mut self, operation: &SemanticOperation) {
        let effects = InstructionEffects::before_arguments(operation.insn_type);
        if !effects.is_pure() {
            *self = std::mem::take(self).join(Self::with_instruction_effects(operation, effects));
        }
    }

    fn with_instruction_effects(
        operation: &SemanticOperation,
        instruction: InstructionEffects,
    ) -> Self {
        let mut summary = Self {
            instruction,
            ..Self::default()
        };
        match operation.insn_type {
            InsnType::Iget | InsnType::Sget => summary.read(Self::field_location(operation)),
            InsnType::Iput | InsnType::Sput => summary.write(Self::field_location(operation)),
            InsnType::ArrayLength | InsnType::Aget => summary.read(MemoryLocation::Array),
            InsnType::Aput | InsnType::FillArray => summary.write(MemoryLocation::Array),
            InsnType::CompoundAssign => {
                summary.read(MemoryLocation::UnknownHeap);
                summary.write(MemoryLocation::UnknownHeap);
            }
            InsnType::StringConcat
            | InsnType::FilledNewArray
            | InsnType::NewArray
            | InsnType::NewInstance
            | InsnType::Invoke
            | InsnType::MoveResult
            | InsnType::Constructor => {
                summary.read(MemoryLocation::UnknownHeap);
                summary.write(MemoryLocation::UnknownHeap);
            }
            _ => {}
        }
        if instruction.reads_memory()
            && matches!(summary.reads, Locations::Empty)
            && instruction.calls()
        {
            summary.read(MemoryLocation::UnknownHeap);
        }
        if instruction.writes_memory()
            && matches!(summary.writes, Locations::Empty)
            && instruction.calls()
        {
            summary.write(MemoryLocation::UnknownHeap);
        }
        summary
    }

    pub(super) fn expression(expression: &SemanticExpression) -> Self {
        let mut summary = Self::pure();
        let mut pending = Vec::new();
        let mut expression = expression;
        loop {
            match expression {
                SemanticExpression::Register(_) | SemanticExpression::Literal(_) => {}
                SemanticExpression::Operation(operation) => {
                    summary = summary.join(Self::direct(operation));
                    pending.extend(operation.operands().iter().rev());
                    pending.extend(operation.compound_target());
                }
                SemanticExpression::Select {
                    condition,
                    when_true,
                    when_false,
                } => {
                    summary = summary.join(Self::predicate(condition).without_control());
                    pending.push(when_false.as_ref());
                    pending.push(when_true.as_ref());
                }
            }
            let Some(next) = pending.pop() else { break };
            expression = next;
        }
        summary
    }

    pub(super) fn operation(operation: &SemanticOperation) -> Self {
        Self::with_operands(operation, Self::direct(operation))
    }

    fn with_operands(operation: &SemanticOperation, mut summary: Self) -> Self {
        for operand in operation.operands() {
            summary = summary.join(Self::expression(operand));
        }
        if let Some(target) = operation.compound_target() {
            summary = summary.join(Self::expression(target));
        }
        summary
    }

    pub(super) fn operation_ignoring_edge_copy(operation: &SemanticOperation) -> Self {
        let direct = Self::with_instruction_effects(
            operation,
            operation.direct_effects_with_edge_copy(false),
        );
        Self::with_operands(operation, direct)
    }

    pub(super) fn statement(statement: &SemanticStatement) -> Self {
        let normal_copy = statement.origin.is_none();
        match &statement.kind {
            SemanticStatementKind::Instruction(operation)
                if operation.payload.edge_copy && normal_copy =>
            {
                Self::operation_ignoring_edge_copy(operation)
            }
            SemanticStatementKind::Instruction(operation) => Self::operation(operation),
            SemanticStatementKind::Definition { value, .. } => {
                match value
                    .as_operation()
                    .filter(|value| value.payload.edge_copy && normal_copy)
                {
                    Some(operation) => Self::operation_ignoring_edge_copy(operation),
                    None => Self::expression(value),
                }
            }
        }
    }

    pub(super) fn predicate(predicate: &crate::ir::SemanticPredicate) -> Self {
        let mut summary = Self::pure();
        let mut pending = Vec::new();
        let mut predicate = predicate;
        loop {
            match predicate {
                crate::ir::SemanticPredicate::Test(operation) => {
                    summary = summary.join(Self::operation(operation));
                }
                crate::ir::SemanticPredicate::Not(inner) => pending.push(inner.as_ref()),
                crate::ir::SemanticPredicate::And(terms)
                | crate::ir::SemanticPredicate::Or(terms) => pending.extend(terms),
                crate::ir::SemanticPredicate::True | crate::ir::SemanticPredicate::False => {}
            }
            let Some(next) = pending.pop() else { break };
            predicate = next;
        }
        summary
    }

    pub(super) fn join(mut self, other: Self) -> Self {
        self.instruction = self.instruction.join(other.instruction);
        self.reads = self.reads.union(other.reads);
        self.writes = self.writes.union(other.writes);
        self
    }

    pub(super) fn without_control(mut self) -> Self {
        self.instruction = self.instruction.without_control();
        self
    }

    pub(super) fn is_pure(&self) -> bool {
        self.instruction.is_pure()
    }

    pub(super) fn is_pure_without_control(&self) -> bool {
        self.instruction.without_control().is_pure()
    }

    pub(super) fn can_relocate(&self) -> bool {
        self.instruction.can_relocate()
    }

    pub(super) fn can_predicate(&self) -> bool {
        self.instruction.can_predicate()
    }

    pub(super) fn conflicts_with(&self, other: &Self) -> bool {
        self.conflicts_with_prefix(other, false)
    }

    pub(super) fn conflicts_with_prefix(&self, prefix: &Self, ignore_control: bool) -> bool {
        let instruction = if ignore_control {
            prefix.instruction.without_control()
        } else {
            prefix.instruction
        };
        if self.is_pure() || instruction.is_pure() {
            return false;
        }
        if self.instruction.controls()
            || instruction.controls()
            || self.instruction.synchronizes()
            || instruction.synchronizes()
            || self.instruction.calls()
            || instruction.calls()
            || (self.instruction.may_throw() && instruction.may_throw())
        {
            return true;
        }
        self.writes.overlaps(&prefix.reads)
            || self.writes.overlaps(&prefix.writes)
            || prefix.writes.overlaps(&self.reads)
    }

    fn read(&mut self, location: MemoryLocation) {
        self.reads = Locations::One(location);
    }

    fn write(&mut self, location: MemoryLocation) {
        self.writes = Locations::One(location);
    }

    fn field_location(operation: &SemanticOperation) -> MemoryLocation {
        operation
            .payload
            .field_index
            .map(MemoryLocation::Field)
            .unwrap_or(MemoryLocation::UnknownHeap)
    }
}
