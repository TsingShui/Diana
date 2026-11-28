//! Source-observable instruction effects shared by dataflow analyses.

use std::fmt;

use crate::ir::{
    ArgType, InsnArg, InsnNode, InsnPayload, InsnType, InvokeType, MemberReference,
    MethodReference, RegisterArg,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrowEffect {
    None,
    SubtypesOf(&'static str),
    Any,
}

impl ThrowEffect {
    pub fn of_tree(instruction: &InsnNode) -> Self {
        let mut effect = Self::None;
        let mut pending = vec![instruction];
        while let Some(instruction) = pending.pop() {
            effect = effect.join(Self::of(instruction));
            pending.extend(
                instruction
                    .args
                    .iter()
                    .chain(instruction.payload.compound_target.as_deref())
                    .filter_map(|argument| match argument {
                        InsnArg::Wrapped(child) => Some(child.as_ref()),
                        InsnArg::Reg(_) | InsnArg::Lit(_) => None,
                    }),
            );
        }
        effect
    }

    fn of(instruction: &InsnNode) -> Self {
        if !instruction.can_throw() {
            Self::None
        } else if instruction.insn_type == InsnType::ConstClass {
            Self::SubtypesOf("java/lang/LinkageError")
        } else {
            Self::Any
        }
    }

    fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::None, effect) | (effect, Self::None) => effect,
            (Self::SubtypesOf(left), Self::SubtypesOf(right)) if left == right => self,
            _ => Self::Any,
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct InstructionEffects(u8);

impl fmt::Debug for InstructionEffects {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InstructionEffects")
            .field("reads", &self.reads_memory())
            .field("writes", &self.writes_memory())
            .field("calls", &self.calls())
            .field("may_throw", &self.may_throw())
            .field("synchronizes", &self.synchronizes())
            .field("controls", &self.controls())
            .finish()
    }
}

impl InstructionEffects {
    // Most significant first, preserving lexicographic ordering of the fields.
    const READS: u8 = 1 << 5;
    const WRITES: u8 = 1 << 4;
    const CALLS: u8 = 1 << 3;
    const MAY_THROW: u8 = 1 << 2;
    const SYNCHRONIZES: u8 = 1 << 1;
    const CONTROLS: u8 = 1;

    pub const PURE: Self = Self(0);
    pub const CONTROL: Self = Self(Self::CONTROLS);
    pub const SYNCHRONIZATION: Self = Self(Self::SYNCHRONIZES | Self::MAY_THROW);
    const UNKNOWN_CALL: Self = Self(Self::READS | Self::WRITES | Self::CALLS | Self::MAY_THROW);

    pub(in crate::ir) const fn bits(self) -> u8 {
        self.0
    }

    /// Reconstitutes the six stored effects, excluding cache metadata.
    pub(in crate::ir) const fn from_bits(bits: u8) -> Self {
        Self(bits & 0b11_1111)
    }

    pub fn of(instruction: &InsnNode) -> Self {
        Self::direct(
            instruction.insn_type,
            instruction.result.as_ref(),
            &instruction.payload,
            instruction.payload.edge_copy,
        )
    }

    /// Effects that precede operand evaluation in source order. Array
    /// initializers allocate before evaluating elements. Sized array
    /// allocations instead evaluate their dimension expressions first.
    pub(crate) fn before_arguments(insn_type: InsnType) -> Self {
        match insn_type {
            InsnType::Constructor | InsnType::FilledNewArray => Self::UNKNOWN_CALL,
            _ => Self::PURE,
        }
    }

    pub(crate) fn direct(
        insn_type: InsnType,
        result: Option<&RegisterArg>,
        payload: &InsnPayload,
        edge_copy: bool,
    ) -> Self {
        if edge_copy {
            return Self(Self::WRITES | Self::CONTROLS);
        }
        if insn_type == InsnType::Invoke {
            if let Some(effects) = IntrinsicMethodEffects::resolve(payload.reference.as_deref()) {
                return effects;
            }
        }
        let mut effects = match insn_type {
            InsnType::Const
            | InsnType::ConstStr
            | InsnType::Move
            | InsnType::Neg
            | InsnType::Not
            | InsnType::Cast
            | InsnType::Cmp
            | InsnType::InstanceOf
            | InsnType::Nop
            | InsnType::Phi
            | InsnType::Arith
            | InsnType::Ternary => Self::PURE,
            InsnType::ConstClass => Self(Self::READS),
            InsnType::CheckCast
            | InsnType::ArrayLength
            | InsnType::Aget
            | InsnType::Iget
            | InsnType::Sget => Self(Self::READS | Self::MAY_THROW),
            InsnType::CompoundAssign
            | InsnType::FillArray
            | InsnType::Aput
            | InsnType::Iput
            | InsnType::Sput => Self(Self::WRITES | Self::MAY_THROW),
            InsnType::Invoke
            | InsnType::Unavailable
            | InsnType::RecordOperation
            | InsnType::StringConcat
            | InsnType::FilledNewArray
            | InsnType::NewArray
            | InsnType::NewInstance
            | InsnType::MoveResult
            | InsnType::Constructor => Self::UNKNOWN_CALL,
            InsnType::MonitorEnter | InsnType::MonitorExit => Self::SYNCHRONIZATION,
            InsnType::Return
            | InsnType::Goto
            | InsnType::Throw
            | InsnType::MoveException
            | InsnType::If
            | InsnType::Switch
            | InsnType::Break
            | InsnType::Continue => Self::CONTROL,
        };
        if insn_type.can_throw(result, payload.arith_op) {
            effects.0 |= Self::MAY_THROW;
        }
        effects
    }

    /// Computes the source-observable effects of an instruction and every
    /// wrapped instruction evaluated as one expression.
    pub fn of_tree(instruction: &InsnNode) -> Self {
        let mut combined = Self::PURE;
        let mut pending = vec![instruction];
        while let Some(instruction) = pending.pop() {
            combined = combined.join(Self::of(instruction));
            pending.extend(
                instruction
                    .args
                    .iter()
                    .chain(instruction.payload.compound_target.as_deref())
                    .filter_map(|argument| match argument {
                        InsnArg::Wrapped(child) => Some(child.as_ref()),
                        InsnArg::Reg(_) | InsnArg::Lit(_) => None,
                    }),
            );
        }
        combined
    }

    pub fn of_argument(argument: &InsnArg) -> Self {
        match argument {
            InsnArg::Wrapped(instruction) => Self::of_tree(instruction),
            InsnArg::Reg(_) | InsnArg::Lit(_) => Self::PURE,
        }
    }

    pub fn join(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn is_pure(self) -> bool {
        self == Self::PURE
    }

    pub fn may_throw(self) -> bool {
        self.0 & Self::MAY_THROW != 0
    }

    pub fn reads_memory(self) -> bool {
        self.0 & Self::READS != 0
    }

    pub fn writes_memory(self) -> bool {
        self.0 & Self::WRITES != 0
    }

    pub fn calls(self) -> bool {
        self.0 & Self::CALLS != 0
    }

    pub fn synchronizes(self) -> bool {
        self.0 & Self::SYNCHRONIZES != 0
    }

    pub fn controls(self) -> bool {
        self.0 & Self::CONTROLS != 0
    }

    pub fn is_ssa_bookkeeping(instruction: &InsnNode) -> bool {
        matches!(
            instruction.insn_type,
            InsnType::Nop
                | InsnType::Goto
                | InsnType::Move
                | InsnType::MoveException
                | InsnType::Phi
        )
    }

    /// Kotlin emits these no-op calls to delimit inlined `finally` bodies.
    /// They carry compiler structure but no source-level execution semantics.
    pub fn is_kotlin_finally_marker(instruction: &InsnNode) -> bool {
        if instruction.insn_type != InsnType::Invoke
            || instruction.payload.invoke_type != Some(InvokeType::Static)
        {
            return false;
        }
        let Some(MemberReference::Method(method)) = instruction.payload.reference.as_deref() else {
            return false;
        };
        method.owner.as_object() == Some("kotlin/jvm/internal/InlineMarker")
            && matches!(method.name.as_str(), "finallyStart" | "finallyEnd")
            && method.descriptor.parameters == [ArgType::INT]
            && method.descriptor.return_type == ArgType::VOID
    }

    pub fn can_relocate(self) -> bool {
        self.0 & (Self::SYNCHRONIZES | Self::CONTROLS) == 0
    }

    /// Whether an instruction can remain inside an explicit execution domain.
    ///
    /// Predication preserves evaluation order and exception timing, so reads
    /// and potentially-throwing expressions are legal. Observable writes,
    /// calls, synchronization, and control transfers still require their
    /// original structured node.
    pub fn can_predicate(self) -> bool {
        self.0 & (Self::WRITES | Self::CALLS | Self::SYNCHRONIZES | Self::CONTROLS) == 0
    }

    pub fn without_control(self) -> Self {
        Self(self.0 & !Self::CONTROLS)
    }

    pub fn is_control_only(self) -> bool {
        self == Self::CONTROL
    }

    pub fn conflicts_with(self, other: Self) -> bool {
        if self.is_pure() || other.is_pure() {
            return false;
        }
        if self.join(other).0 & (Self::CONTROLS | Self::SYNCHRONIZES | Self::CALLS) != 0 {
            return true;
        }
        if (self.writes_memory() && other.0 & (Self::READS | Self::WRITES) != 0)
            || (other.writes_memory() && self.0 & (Self::READS | Self::WRITES) != 0)
        {
            return true;
        }
        self.may_throw() && other.may_throw()
    }
}

struct IntrinsicMethodEffects;

impl IntrinsicMethodEffects {
    fn resolve(reference: Option<&MemberReference>) -> Option<InstructionEffects> {
        let MemberReference::Method(method) = reference? else {
            return None;
        };
        Self::floating_predicate(method).then_some(InstructionEffects::PURE)
    }

    fn floating_predicate(method: &MethodReference) -> bool {
        let operand = match method.owner.as_object() {
            Some("java/lang/Float") => ArgType::FLOAT,
            Some("java/lang/Double") => ArgType::DOUBLE,
            _ => return false,
        };
        matches!(method.name.as_str(), "isNaN" | "isInfinite")
            && method.descriptor.parameters == [operand]
            && method.descriptor.return_type == ArgType::BOOLEAN
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/instruction_effects.rs"]
mod tests;
