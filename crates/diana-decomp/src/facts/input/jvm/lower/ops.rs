//! Transfer checked JVM instruction operands to the shared register IR.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use smallvec::SmallVec;

use super::super::bytecode::{ConstantOperand, Instruction, OperandProblem, Operands, Reference};
use super::super::{ClassFile, Constant, JvmString, ReferenceKind};
use crate::ir::{
    ArgType, ArithOp, CmpBias, FieldReference, IfOp, InsnArg, InsnNode, InsnType, InvokeType,
    MemberReference, MethodDescriptor, MethodReference, PrimitiveType, RegisterArg, UnaryOp,
    Utf16String,
};
use crate::FailureKind;

#[path = "bootstrap.rs"]
mod bootstrap;

const SCRATCH_WORDS: u32 = 4;

#[derive(Clone, Copy, Debug)]
pub(super) struct Registers {
    locals: u16,
    stack: u16,
    input: u16,
}

impl Registers {
    pub(super) fn new(locals: u16, stack: u16, input: u16) -> Result<Self, OpError> {
        if input > locals {
            return Err(OpError::Local);
        }
        Ok(Self {
            locals,
            stack,
            input,
        })
    }

    pub(super) fn count(self) -> u32 {
        u32::from(self.locals) + u32::from(self.stack) + SCRATCH_WORDS + self.input()
    }

    pub(super) fn input(self) -> u32 {
        u32::from(self.input)
    }

    /// The shared backend describes all incoming parameters in a high range.
    /// The producer copies those parameters into the original JVM local window.
    pub(super) fn input_register(self, slot: u16) -> Result<u32, OpError> {
        if slot >= self.input {
            return Err(OpError::Local);
        }
        Ok(self.count() - self.input() + u32::from(slot))
    }

    pub(super) fn local(self, slot: u16) -> Result<u32, OpError> {
        self.local_width(slot, 1)
    }

    fn local_width(self, slot: u16, words: u16) -> Result<u32, OpError> {
        if slot.checked_add(words).is_none_or(|end| end > self.locals) {
            return Err(OpError::Local);
        }
        Ok(u32::from(slot))
    }

    pub(super) fn stack(self, word: u16) -> u32 {
        u32::from(self.locals) + u32::from(word)
    }

    fn scratch(self, word: u16) -> u32 {
        self.stack(self.stack) + u32::from(word)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Category {
    Int,
    Long,
    Float,
    Double,
    Reference,
}

impl Category {
    fn of(ty: &ArgType) -> Result<Self, OpError> {
        use PrimitiveType::*;
        Ok(match ty.as_primitive() {
            Some(Boolean | Byte | Short | Char | Int) => Self::Int,
            Some(Long) => Self::Long,
            Some(Float) => Self::Float,
            Some(Double) => Self::Double,
            Some(Object | Array) | None => Self::Reference,
            Some(Void) => return Err(OpError::Category),
        })
    }

    fn default_type(self) -> ArgType {
        match self {
            Self::Int => ArgType::INT,
            Self::Long => ArgType::LONG,
            Self::Float => ArgType::FLOAT,
            Self::Double => ArgType::DOUBLE,
            Self::Reference => ArgType::unknown_object(),
        }
    }

    fn words(self) -> u16 {
        if matches!(self, Self::Long | Self::Double) {
            2
        } else {
            1
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Value {
    ty: ArgType,
    category: Category,
    available: bool,
    runtime: bool,
    input_parameter: Option<u16>,
}

impl Value {
    fn new(ty: ArgType, available: bool) -> Result<Self, OpError> {
        Ok(Self {
            category: Category::of(&ty)?,
            ty,
            available,
            runtime: false,
            input_parameter: None,
        })
    }

    fn widen_same_category(&mut self, other: &Self) -> bool {
        let mut changed = false;
        if self.ty != other.ty {
            let widened = self.category.default_type();
            changed |= self.ty != widened;
            self.ty = widened;
        }
        if self.available && !other.available {
            self.available = false;
            changed = true;
        }
        if self.runtime && !other.runtime {
            self.runtime = false;
            changed = true;
        }
        if self.input_parameter.is_some() && self.input_parameter != other.input_parameter {
            self.input_parameter = None;
            changed = true;
        }
        changed
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct Stack {
    slots: SmallVec<[Value; 8]>,
    words: u16,
    unknown: bool,
    locals: HashMap<u16, Value>,
}

impl Stack {
    pub(super) fn is_unknown(&self) -> bool {
        self.unknown
    }

    pub(super) fn frame_values(&self) -> usize {
        self.slots.len() + self.locals.len()
    }

    pub(super) fn invalidate(&mut self) {
        self.slots.clear();
        self.locals.clear();
        self.words = 0;
        self.unknown = true;
    }

    pub(super) fn seed_local(
        &mut self,
        slot: u16,
        ty: ArgType,
        input_parameter: Option<u16>,
    ) -> Result<(), OpError> {
        let mut value = Value::new(ty, true)?;
        value.runtime = true;
        value.input_parameter = input_parameter;
        self.set_local(slot, value)
    }

    pub(super) fn push_exception(
        &mut self,
        ty: ArgType,
        registers: Registers,
    ) -> Result<(), OpError> {
        self.slots.clear();
        self.words = 0;
        self.unknown = false;
        let mut value = Value::new(ty, true)?;
        value.runtime = true;
        self.push(value, registers)?;
        Ok(())
    }

    pub(super) fn join(&mut self, other: &Self) -> Result<bool, OpError> {
        if self.unknown {
            return Ok(false);
        }
        if other.unknown {
            self.invalidate();
            return Ok(true);
        }
        if self.words != other.words || self.slots.len() != other.slots.len() {
            return Err(OpError::MergeHeight);
        }
        let mut changed = false;
        for (current, incoming) in self.slots.iter_mut().zip(&other.slots) {
            if current.category != incoming.category {
                return Err(OpError::Category);
            }
            changed |= current.widen_same_category(incoming);
        }
        self.locals.retain(|slot, current| {
            let incoming = other
                .locals
                .get(slot)
                .filter(|value| value.category == current.category);
            if let Some(incoming) = incoming {
                changed |= current.widen_same_category(incoming);
                true
            } else {
                // An uninitialized local, a category-2 tail and an
                // incompatible category are all Top at this join.
                changed = true;
                false
            }
        });
        Ok(changed)
    }

    fn local_value(&self, slot: u16, category: Category) -> Option<Value> {
        self.locals
            .get(&slot)
            .filter(|value| value.category == category)
            .cloned()
    }

    fn set_local(&mut self, slot: u16, value: Value) -> Result<(), OpError> {
        // Overwriting either word of an existing wide local invalidates the
        // old value. A wide tail has no independently readable value.
        if let Some(previous) = slot.checked_sub(1).filter(|slot| {
            self.locals
                .get(slot)
                .is_some_and(|value| value.category.words() == 2)
        }) {
            self.locals.remove(&previous);
        }
        if let (2, Some(tail)) = (value.category.words(), slot.checked_add(1)) {
            self.locals.remove(&tail);
        }
        self.locals
            .try_reserve(1)
            .map_err(|_| OpError::Allocation)?;
        self.locals.insert(slot, value);
        Ok(())
    }

    fn push(&mut self, value: Value, registers: Registers) -> Result<RegisterArg, OpError> {
        let end = self
            .words
            .checked_add(value.category.words())
            .ok_or(OpError::Overflow)?;
        if end > registers.stack {
            return Err(OpError::Overflow);
        }
        self.slots.try_reserve(1).map_err(|_| OpError::Allocation)?;
        let register = RegisterArg::new(registers.stack(self.words), value.ty.clone());
        self.slots.push(value);
        self.words = end;
        Ok(register)
    }

    fn pop(&mut self, registers: Registers, expected: Option<Category>) -> Result<Read, OpError> {
        let value = self.slots.last().ok_or(OpError::Underflow)?;
        if expected.is_some_and(|expected| expected != value.category) {
            return Err(OpError::Category);
        }
        let value = self.slots.pop().ok_or(OpError::Underflow)?;
        self.words -= value.category.words();
        Ok(Read {
            register: registers.stack(self.words),
            value,
        })
    }

    fn group(
        &mut self,
        registers: Registers,
        mut words: u16,
    ) -> Result<SmallVec<[Read; 2]>, OpError> {
        let mut out = SmallVec::new();
        while words != 0 {
            let item = self.pop(registers, None)?;
            words = words
                .checked_sub(item.value.category.words())
                .ok_or(OpError::Category)?;
            out.push(item);
        }
        out.reverse();
        Ok(out)
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum OpError {
    #[error("JVM local index exceeds max_locals")]
    Local,
    #[error("JVM operand stack underflow")]
    Underflow,
    #[error("JVM operand stack exceeds max_stack")]
    Overflow,
    #[error("incompatible JVM operand stack categories")]
    Category,
    #[error("inconsistent JVM operand stack heights at control-flow join")]
    MergeHeight,
    #[error("allocation failed while lowering JVM instruction")]
    Allocation,
    #[error("unavailable incoming JVM operand stack")]
    UnknownStack,
    #[error("instruction depends on an unavailable JVM operand")]
    Dependency,
    #[error("unsupported JVM instruction {0}")]
    Unsupported(&'static str),
    #[error("{reason}")]
    UnsupportedEffect { kind: FailureKind, reason: Box<str> },
    #[error("invalid JVM instruction operand: {0:?}")]
    Operand(OperandProblem),
    #[error("invalid reference pool operand for JVM instruction {0}")]
    Reference(&'static str),
    #[error("JVM reference identity contains an isolated UTF-16 surrogate")]
    NonUnicodeName,
    #[error(transparent)]
    Class(#[from] super::super::ClassError),
    #[error(transparent)]
    Constant(#[from] super::super::bytecode::ReferenceError),
    #[error(transparent)]
    Descriptor(#[from] crate::ir::DescriptorParseError),
}

impl OpError {
    pub(super) fn kind(&self) -> FailureKind {
        match self {
            Self::Allocation => FailureKind::ResourceExhausted,
            Self::UnsupportedEffect { kind, .. } => *kind,
            Self::Unsupported(_) | Self::UnknownStack | Self::Dependency | Self::NonUnicodeName => {
                FailureKind::Unsupported
            }
            Self::Class(error) => error.kind(),
            Self::Constant(error) => error.kind(),
            Self::Local
            | Self::Underflow
            | Self::Overflow
            | Self::Category
            | Self::MergeHeight
            | Self::Operand(_)
            | Self::Reference(_)
            | Self::Descriptor(_) => FailureKind::Malformed,
        }
    }

    pub(super) fn retains_stack(&self) -> bool {
        matches!(self, Self::Dependency | Self::UnsupportedEffect { .. })
    }
}

#[derive(Clone)]
struct Read {
    register: u32,
    value: Value,
}

impl Read {
    fn arg(&self) -> InsnArg {
        InsnArg::reg(self.register, self.value.ty.clone())
    }
}

struct Lower<'a> {
    stack: &'a mut Stack,
    registers: Registers,
    output: Option<&'a mut Vec<InsnNode>>,
    pc: u16,
}

impl Lower<'_> {
    fn pop(&mut self, category: Category) -> Result<Read, OpError> {
        self.stack.pop(self.registers, Some(category))
    }

    fn node(
        &mut self,
        kind: InsnType,
        result: Option<RegisterArg>,
        args: &[Read],
        payload: impl FnOnce(&mut InsnNode),
    ) -> Result<(), OpError> {
        if args.iter().any(|arg| !arg.value.available) {
            return Err(OpError::Dependency);
        }
        if let Some(output) = &mut self.output {
            output.try_reserve(1).map_err(|_| OpError::Allocation)?;
            let mut insn = InsnNode::new(kind, args.len());
            insn.offset = u32::from(self.pc);
            insn.result = result;
            insn.args.extend(args.iter().map(Read::arg));
            payload(&mut insn);
            output.push(insn);
        }
        Ok(())
    }

    fn expr(
        &mut self,
        kind: InsnType,
        ty: ArgType,
        args: &[Read],
        payload: impl FnOnce(&mut InsnNode),
    ) -> Result<(), OpError> {
        let available = args.iter().all(|arg| arg.value.available);
        let mut value = Value::new(ty, available)?;
        value.runtime = match kind {
            InsnType::Const | InsnType::ConstStr => false,
            InsnType::Move | InsnType::Arith | InsnType::Neg | InsnType::Not | InsnType::Cast => {
                args.iter().any(|arg| arg.value.runtime)
            }
            // These operations are not Java constant expressions.
            _ => true,
        };
        if let (InsnType::Move, [operand]) = (kind, args) {
            value.input_parameter = operand.value.input_parameter;
        }
        let dest = self.stack.push(value, self.registers)?;
        self.node(kind, Some(dest), args, payload)
    }

    fn effect(
        &mut self,
        instruction: Instruction<'_>,
        result: ArgType,
        args: &[Read],
        kind: FailureKind,
        detail: impl fmt::Display,
    ) -> Result<(), OpError> {
        let dest = if result == ArgType::VOID {
            None
        } else {
            // The bytecode proves that the operation produces this type. The
            // IR keeps the definition and marks its value as unknown.
            let mut value = Value::new(result, true)?;
            value.runtime = true;
            Some(self.stack.push(value, self.registers)?)
        };
        self.opaque(instruction, dest, args, kind, detail)
    }

    fn opaque(
        &mut self,
        instruction: Instruction<'_>,
        dest: Option<RegisterArg>,
        args: &[Read],
        kind: FailureKind,
        detail: impl fmt::Display,
    ) -> Result<(), OpError> {
        let mnemonic = instruction
            .modified_mnemonic()
            .unwrap_or(instruction.mnemonic());
        let index = instruction
            .constant_index()
            .map_or_else(String::new, |index| format!(" #{index}"));
        let reason = format!("JVM byte {}: {mnemonic}{index}: {detail}", self.pc).into_boxed_str();
        if let Some(output) = &mut self.output {
            output.try_reserve(1).map_err(|_| OpError::Allocation)?;
            let mut node = InsnNode::unavailable(reason.clone());
            node.offset = u32::from(self.pc);
            node.result = dest;
            node.args
                .try_reserve(args.len())
                .map_err(|_| OpError::Allocation)?;
            node.args
                .extend(args.iter().filter(|arg| arg.value.available).map(Read::arg));
            output.push(node);
        }
        Err(OpError::UnsupportedEffect { kind, reason })
    }

    fn literal(&mut self, value: i64, ty: ArgType) -> Result<(), OpError> {
        self.expr(InsnType::Const, ty.clone(), &[], |node| {
            node.add_arg(InsnArg::lit(value, ty));
        })
    }

    fn load(
        &mut self,
        instruction: Instruction<'_>,
        slot: u16,
        category: Category,
    ) -> Result<(), OpError> {
        let register = self.registers.local_width(slot, category.words())?;
        let Some(value) = self.stack.local_value(slot, category) else {
            return self.effect(
                instruction,
                category.default_type(),
                &[],
                FailureKind::Malformed,
                format_args!("local slot {slot} has no value of the required JVM category"),
            );
        };
        let ty = value.ty.clone();
        self.expr(InsnType::Move, ty, &[Read { register, value }], |_| {})
    }

    fn store(&mut self, slot: u16, category: Category) -> Result<(), OpError> {
        let register = self.registers.local_width(slot, category.words())?;
        let read = self.pop(category)?;
        let dest = RegisterArg::new(register, read.value.ty.clone());
        self.stack.set_local(slot, read.value.clone())?;
        self.node(InsnType::Move, Some(dest), &[read], |_| {})
    }

    fn increment(
        &mut self,
        instruction: Instruction<'_>,
        slot: u16,
        value: i16,
    ) -> Result<(), OpError> {
        let register = self.registers.local(slot)?;
        let dest = RegisterArg::new(register, ArgType::INT);
        let Some(old) = self.stack.local_value(slot, Category::Int) else {
            let mut next = Value::new(ArgType::INT, true)?;
            next.runtime = true;
            self.stack.set_local(slot, next)?;
            return self.opaque(
                instruction,
                Some(dest),
                &[],
                FailureKind::Malformed,
                format_args!("local slot {slot} has no integer value to increment by {value}"),
            );
        };
        let mut next = Value::new(ArgType::INT, old.available)?;
        next.runtime = old.runtime;
        self.stack.set_local(slot, next)?;
        self.node(
            InsnType::Arith,
            Some(dest),
            &[Read {
                register,
                value: old,
            }],
            |node| {
                node.payload.arith_op = Some(ArithOp::Add);
                node.add_arg(InsnArg::lit(i64::from(value), ArgType::INT));
            },
        )
    }

    fn discard(&mut self, words: u16) -> Result<(), OpError> {
        self.stack.group(self.registers, words)?;
        Ok(())
    }

    fn permute(&mut self, top_words: u16, underneath: u16, duplicate: bool) -> Result<(), OpError> {
        let mut top = self.stack.group(self.registers, top_words)?;
        let mut below = self.stack.group(self.registers, underneath)?;
        let mut scratch = 0;
        if underneath != 0 {
            for source in below.iter_mut().chain(&mut top) {
                let register = self.registers.scratch(scratch);
                scratch += source.value.category.words();
                if source.value.available {
                    let dest = RegisterArg::new(register, source.value.ty.clone());
                    self.node(
                        InsnType::Move,
                        Some(dest),
                        std::slice::from_ref(source),
                        |_| {},
                    )?;
                }
                source.register = register;
            }
        }
        let copies = if duplicate { top.as_slice() } else { &[] };
        for source in top.iter().chain(&below).chain(copies) {
            let dest = self.stack.push(source.value.clone(), self.registers)?;
            if source.value.available && dest.reg_num != source.register {
                self.node(
                    InsnType::Move,
                    Some(dest),
                    std::slice::from_ref(source),
                    |_| {},
                )?;
            }
        }
        Ok(())
    }
}

pub(super) fn apply(
    class: &ClassFile<'_>,
    instruction: Instruction<'_>,
    stack: &mut Stack,
    registers: Registers,
    output: Option<&mut Vec<InsnNode>>,
) -> Result<(), OpError> {
    if let Some(problem) = instruction.operand_problem() {
        return Err(OpError::Operand(problem));
    }
    let encoded = instruction.opcode();
    let opcode = instruction.wide_opcode().unwrap_or(encoded);
    if stack.is_unknown() && !matches!(opcode, 0x00 | 0xa7 | 0xb1 | 0xc8) {
        return Err(OpError::UnknownStack);
    }
    let mut lower = Lower {
        stack,
        registers,
        output,
        pc: instruction.offset(),
    };
    match opcode {
        0x00 => lower.node(InsnType::Nop, None, &[], |_| {}),
        0x01 => lower.literal(0, ArgType::object("java/lang/Object")),
        0x02..=0x08 => lower.literal(i64::from(opcode) - 3, ArgType::INT),
        0x09..=0x0a => lower.literal(i64::from(opcode - 0x09), ArgType::LONG),
        0x0b..=0x0d => lower.literal(
            i64::from(f32::from(opcode - 0x0b).to_bits()),
            ArgType::FLOAT,
        ),
        0x0e..=0x0f => lower.literal(f64::from(opcode - 0x0e).to_bits() as i64, ArgType::DOUBLE),
        0x10..=0x11 => {
            let Operands::Immediate(value) = instruction.operands() else {
                return Err(OpError::Reference(instruction.mnemonic()));
            };
            lower.literal(i64::from(value), ArgType::INT)
        }
        0x12..=0x14 => load_constant(&mut lower, class, instruction),
        0x15..=0x2d => access_local(&mut lower, instruction, opcode, true),
        0x2e..=0x35 => array_access(&mut lower, opcode - 0x2e, true),
        0x36..=0x4e => access_local(&mut lower, instruction, opcode, false),
        0x4f..=0x56 => array_access(&mut lower, opcode - 0x4f, false),
        0x57..=0x58 => lower.discard(u16::from(opcode - 0x57) + 1),
        0x59 => lower.permute(1, 0, true),
        0x5a => lower.permute(1, 1, true),
        0x5b => lower.permute(1, 2, true),
        0x5c => lower.permute(2, 0, true),
        0x5d => lower.permute(2, 1, true),
        0x5e => lower.permute(2, 2, true),
        0x5f => lower.permute(1, 1, false),
        0x60..=0x83 => arithmetic(&mut lower, opcode),
        0x84 => {
            let Operands::Increment { local, value } = instruction.operands() else {
                return Err(OpError::Local);
            };
            lower.increment(instruction, local, value)
        }
        0x85..=0x93 => cast(&mut lower, opcode),
        0x94..=0x98 => compare(&mut lower, opcode),
        0x99..=0xa7 | 0xc6..=0xc8 => branch(&mut lower, instruction),
        0xaa..=0xab => switch(&mut lower, instruction),
        0xac..=0xb0 => {
            let category = LOCAL_CATEGORIES[usize::from(opcode - 0xac)];
            let value = lower.pop(category)?;
            lower.node(InsnType::Return, None, &[value], |_| {})
        }
        0xb1 => lower.node(InsnType::Return, None, &[], |_| {}),
        0xb2..=0xb5 => field(&mut lower, class, instruction),
        0xb6..=0xba => invoke(&mut lower, class, instruction),
        0xbb..=0xbd | 0xc0..=0xc1 | 0xc5 => typed(&mut lower, class, instruction),
        0xbe => {
            let array = lower.pop(Category::Reference)?;
            lower.expr(InsnType::ArrayLength, ArgType::INT, &[array], |node| {
                node.payload.is_length = Some(true);
            })
        }
        0xbf | 0xc2..=0xc3 => {
            let value = lower.pop(Category::Reference)?;
            let kind = match opcode {
                0xbf => InsnType::Throw,
                0xc2 => InsnType::MonitorEnter,
                _ => InsnType::MonitorExit,
            };
            lower.node(kind, None, &[value], |node| {
                if opcode != 0xbf {
                    node.payload.is_enter = Some(opcode == 0xc2);
                }
            })
        }
        _ => Err(OpError::Unsupported(instruction.mnemonic())),
    }
}

const LOCAL_CATEGORIES: [Category; 5] = [
    Category::Int,
    Category::Long,
    Category::Float,
    Category::Double,
    Category::Reference,
];

fn access_local(
    lower: &mut Lower<'_>,
    instruction: Instruction<'_>,
    opcode: u8,
    load: bool,
) -> Result<(), OpError> {
    let (slot, category) = local_access(instruction, opcode, load)?;
    if load {
        lower.load(instruction, slot, category)
    } else {
        lower.store(slot, category)
    }
}

fn local_access(
    instruction: Instruction<'_>,
    opcode: u8,
    load: bool,
) -> Result<(u16, Category), OpError> {
    let base = if load { 0x15 } else { 0x36 };
    let offset = opcode - base;
    let (slot, kind) = if offset < 5 {
        let Operands::Local(slot) = instruction.operands() else {
            return Err(OpError::Local);
        };
        (slot, offset)
    } else {
        let short = offset - 5;
        (u16::from(short % 4), short / 4)
    };
    let category = LOCAL_CATEGORIES[usize::from(kind)];
    Ok((slot, category))
}

/// The receiver and JVM method parameters occupy at most 255 local words.
/// Instruction coverage must be complete before absence of a store is evidence.
pub(super) fn written_parameter_slots(
    instructions: &[Instruction<'_>],
) -> Result<[bool; 256], OpError> {
    let mut written = [false; 256];
    for &instruction in instructions {
        let opcode = instruction
            .wide_opcode()
            .unwrap_or_else(|| instruction.opcode());
        let (slot, words) = match opcode {
            0x36..=0x4e => {
                let (slot, category) = local_access(instruction, opcode, false)?;
                (slot, category.words())
            }
            0x84 => {
                let Operands::Increment { local, .. } = instruction.operands() else {
                    return Err(OpError::Local);
                };
                (local, 1)
            }
            _ => continue,
        };
        for word in usize::from(slot)..usize::from(slot) + usize::from(words) {
            if let Some(mark) = written.get_mut(word) {
                *mark = true;
            }
        }
    }
    Ok(written)
}

fn array_access(lower: &mut Lower<'_>, kind: u8, load: bool) -> Result<(), OpError> {
    let ty = match kind {
        0 => ArgType::INT,
        1 => ArgType::LONG,
        2 => ArgType::FLOAT,
        3 => ArgType::DOUBLE,
        4 => ArgType::unknown_object(),
        5 => ArgType::BYTE,
        6 => ArgType::CHAR,
        7 => ArgType::SHORT,
        _ => return Err(OpError::Category),
    };
    let category = Category::of(&ty)?;
    let value = if load {
        None
    } else {
        Some(lower.pop(category)?)
    };
    let index = lower.pop(Category::Int)?;
    let array = lower.pop(Category::Reference)?;
    if let Some(value) = value {
        return lower.node(InsnType::Aput, None, &[value, array, index], |node| {
            node.payload.is_get = Some(false);
        });
    }
    let actual = array
        .value
        .ty
        .as_array_element()
        .filter(|element| match kind {
            4 => element.is_reference(),
            5 => **element == ArgType::BOOLEAN,
            _ => false,
        });
    let result = actual.cloned().unwrap_or(ty);
    lower.expr(InsnType::Aget, result, &[array, index], |node| {
        node.payload.is_get = Some(true);
    })
}

fn arithmetic(lower: &mut Lower<'_>, op: u8) -> Result<(), OpError> {
    const BASIC: [ArithOp; 5] = [
        ArithOp::Add,
        ArithOp::Sub,
        ArithOp::Mul,
        ArithOp::Div,
        ArithOp::Rem,
    ];
    const INTEGRAL: [ArithOp; 6] = [
        ArithOp::Shl,
        ArithOp::Shr,
        ArithOp::Ushr,
        ArithOp::And,
        ArithOp::Or,
        ArithOp::Xor,
    ];

    if (0x74..=0x77).contains(&op) {
        let category = LOCAL_CATEGORIES[usize::from(op - 0x74)];
        let value = lower.pop(category)?;
        return lower.expr(InsnType::Neg, category.default_type(), &[value], |node| {
            node.payload.unary_op = Some(UnaryOp::Neg);
        });
    }
    let (arith, category, right_type) = if op < 0x74 {
        let offset = usize::from(op - 0x60);
        let category = LOCAL_CATEGORIES[offset % 4];
        (BASIC[offset / 4], category, category)
    } else {
        let offset = usize::from(op - 0x78);
        let category = LOCAL_CATEGORIES[offset % 2];
        let right = if offset < 6 { Category::Int } else { category };
        (INTEGRAL[offset / 2], category, right)
    };
    let right = lower.pop(right_type)?;
    let left = lower.pop(category)?;
    lower.expr(
        InsnType::Arith,
        category.default_type(),
        &[left, right],
        |node| {
            node.payload.arith_op = Some(arith);
        },
    )
}

fn cast(lower: &mut Lower<'_>, op: u8) -> Result<(), OpError> {
    use Category::{Double, Float, Int, Long};
    use UnaryOp::*;
    let (before, target, unary) = match op {
        0x85 => (Int, ArgType::LONG, IntToLong),
        0x86 => (Int, ArgType::FLOAT, IntToFloat),
        0x87 => (Int, ArgType::DOUBLE, IntToDouble),
        0x88 => (Long, ArgType::INT, LongToInt),
        0x89 => (Long, ArgType::FLOAT, LongToFloat),
        0x8a => (Long, ArgType::DOUBLE, LongToDouble),
        0x8b => (Float, ArgType::INT, FloatToInt),
        0x8c => (Float, ArgType::LONG, FloatToLong),
        0x8d => (Float, ArgType::DOUBLE, FloatToDouble),
        0x8e => (Double, ArgType::INT, DoubleToInt),
        0x8f => (Double, ArgType::LONG, DoubleToLong),
        0x90 => (Double, ArgType::FLOAT, DoubleToFloat),
        0x91 => (Int, ArgType::BYTE, IntToByte),
        0x92 => (Int, ArgType::CHAR, IntToChar),
        0x93 => (Int, ArgType::SHORT, IntToShort),
        _ => return Err(OpError::Category),
    };
    let operand = lower.pop(before)?;
    lower.expr(InsnType::Cast, target.clone(), &[operand], |node| {
        node.payload.unary_op = Some(unary);
        node.payload.cast_type = Some(Box::new(target));
    })
}

fn compare(lower: &mut Lower<'_>, op: u8) -> Result<(), OpError> {
    let (category, bias) = match op {
        0x94 => (Category::Long, CmpBias::None),
        0x95 => (Category::Float, CmpBias::Lt),
        0x96 => (Category::Float, CmpBias::Gt),
        0x97 => (Category::Double, CmpBias::Lt),
        0x98 => (Category::Double, CmpBias::Gt),
        _ => return Err(OpError::Category),
    };
    let right = lower.pop(category)?;
    let left = lower.pop(category)?;
    lower.expr(InsnType::Cmp, ArgType::INT, &[left, right], |node| {
        node.payload.cmp_bias = Some(bias);
    })
}

fn branch(lower: &mut Lower<'_>, instruction: Instruction<'_>) -> Result<(), OpError> {
    const OPS: [IfOp; 6] = [IfOp::Eq, IfOp::Ne, IfOp::Lt, IfOp::Ge, IfOp::Gt, IfOp::Le];
    let relative = instruction
        .relative_branch()
        .ok_or(OpError::Reference(instruction.mnemonic()))?;
    let target = i32::from(instruction.target(relative).map_err(OpError::Operand)?);
    let opcode = instruction.opcode();
    let configure = |node: &mut InsnNode, condition| {
        node.payload.if_op = Some(condition);
        node.payload.target = Some(target);
    };
    match opcode {
        0xa7 | 0xc8 => lower.node(InsnType::Goto, None, &[], |node| {
            node.payload.target = Some(target)
        }),
        0x99..=0x9e | 0xc6..=0xc7 => {
            let (category, test, literal_type) = if opcode >= 0xc6 {
                (
                    Category::Reference,
                    OPS[usize::from(opcode - 0xc6)],
                    ArgType::object("java/lang/Object"),
                )
            } else {
                (Category::Int, OPS[usize::from(opcode - 0x99)], ArgType::INT)
            };
            let value = lower.pop(category)?;
            lower.node(InsnType::If, None, &[value], |node| {
                configure(node, test);
                node.add_arg(InsnArg::lit(0, literal_type));
            })
        }
        0x9f..=0xa6 => {
            let (category, test) = if opcode >= 0xa5 {
                (Category::Reference, OPS[usize::from(opcode - 0xa5)])
            } else {
                (Category::Int, OPS[usize::from(opcode - 0x9f)])
            };
            let right = lower.pop(category)?;
            let left = lower.pop(category)?;
            lower.node(InsnType::If, None, &[left, right], |node| {
                configure(node, test)
            })
        }
        _ => Err(OpError::Unsupported(instruction.mnemonic())),
    }
}

fn switch(lower: &mut Lower<'_>, instruction: Instruction<'_>) -> Result<(), OpError> {
    let switch = instruction
        .switch()
        .ok_or(OpError::Reference(instruction.mnemonic()))?;
    let key = lower.pop(Category::Int)?;
    if lower.output.is_none() {
        return if key.value.available {
            Ok(())
        } else {
            Err(OpError::Dependency)
        };
    }
    let target = |offset| {
        instruction
            .target(offset)
            .map(i32::from)
            .map_err(OpError::Operand)
    };
    let default = target(switch.default)?;
    let mut cases = Vec::new();
    cases
        .try_reserve_exact(switch.cases().len())
        .map_err(|_| OpError::Allocation)?;
    for (value, relative) in switch.cases() {
        cases.push((value, target(relative)?));
    }
    lower.node(InsnType::Switch, None, &[key], |node| {
        node.payload.switch_cases = Some(Box::new(cases));
        node.payload.switch_default = Some(default);
    })
}

fn text(value: JvmString<'_>) -> Result<std::borrow::Cow<'_, str>, OpError> {
    if let Some(value) = value.utf8() {
        return Ok(std::borrow::Cow::Borrowed(value));
    }
    String::from_utf16(&value.to_utf16()?)
        .map(std::borrow::Cow::Owned)
        .map_err(|_| OpError::NonUnicodeName)
}

fn class_type(value: JvmString<'_>) -> Result<ArgType, OpError> {
    let name = text(value)?;
    Ok(if name.starts_with('[') {
        name.parse()?
    } else {
        ArgType::object(&name)
    })
}

fn load_constant(
    lower: &mut Lower<'_>,
    class: &ClassFile<'_>,
    instruction: Instruction<'_>,
) -> Result<(), OpError> {
    let index = instruction
        .constant_index()
        .ok_or(OpError::Reference(instruction.mnemonic()))?;
    let wide = instruction.opcode() == 0x14;
    match class.constant(index)? {
        Constant::Integer(value) if !wide => return lower.literal(i64::from(value), ArgType::INT),
        Constant::Float(value) if !wide => return lower.literal(i64::from(value), ArgType::FLOAT),
        Constant::Long(value) if wide => return lower.literal(value, ArgType::LONG),
        Constant::Double(value) if wide => return lower.literal(value as i64, ArgType::DOUBLE),
        _ => {}
    }
    match instruction.reference(class)? {
        Some(Reference::String(value)) => {
            let units = if lower.output.is_some() {
                value.to_utf16()?
            } else {
                Vec::new()
            };
            lower.expr(InsnType::ConstStr, ArgType::string(), &[], |node| {
                node.payload.string_value = Some(Box::new(Utf16String::from_utf16(units)));
            })
        }
        Some(Reference::Class(value)) => {
            let ty = class_type(value)?;
            lower.expr(InsnType::ConstClass, ArgType::class(), &[], |node| {
                node.payload.class_type = Some(Box::new(ty));
            })
        }
        Some(Reference::Dynamic {
            name,
            descriptor,
            bootstrap_index,
            ..
        }) => lower.effect(
            instruction,
            text(descriptor)?.parse()?,
            &[],
            FailureKind::Unsupported,
            format_args!(
                "dynamic constant {} {} using bootstrap #{bootstrap_index}",
                bootstrap::OriginalText(name),
                bootstrap::OriginalText(descriptor),
            ),
        ),
        Some(Reference::MethodHandle { kind, member, .. }) => lower.effect(
            instruction,
            ArgType::object("java/lang/invoke/MethodHandle"),
            &[],
            FailureKind::Unsupported,
            format_args!(
                "method handle kind {kind} for {}",
                bootstrap::OriginalMember(member)
            ),
        ),
        Some(Reference::MethodType(descriptor)) => lower.effect(
            instruction,
            ArgType::object("java/lang/invoke/MethodType"),
            &[],
            FailureKind::Unsupported,
            format_args!("method type {}", bootstrap::OriginalText(descriptor)),
        ),
        _ => Err(OpError::Reference(instruction.mnemonic())),
    }
}

fn field(
    lower: &mut Lower<'_>,
    class: &ClassFile<'_>,
    instruction: Instruction<'_>,
) -> Result<(), OpError> {
    let Some(Reference::Member(reference)) = instruction.reference(class)? else {
        return Err(OpError::Reference(instruction.mnemonic()));
    };
    let opcode = instruction.opcode();
    let field_type: ArgType = text(reference.descriptor)?.parse()?;
    let static_field = opcode < 0xb4;
    let get = opcode == 0xb2 || opcode == 0xb4;
    let mut operands: SmallVec<[Read; 2]> = SmallVec::new();
    if !get {
        operands.push(lower.pop(Category::of(&field_type)?)?);
    }
    if !static_field {
        operands.push(lower.pop(Category::Reference)?);
    }
    let owner = class_type(reference.owner)?;
    let name = text(reference.name)?;
    let payload = |node: &mut InsnNode| {
        node.payload.reference = Some(Arc::new(MemberReference::Field(FieldReference {
            owner,
            name: name.into_owned(),
            field_type: field_type.clone(),
        })));
        node.payload.is_static = Some(static_field);
        node.payload.is_get = Some(get);
    };
    match opcode {
        0xb2 => lower.expr(InsnType::Sget, field_type.clone(), &operands, payload),
        0xb3 => lower.node(InsnType::Sput, None, &operands, payload),
        0xb4 => lower.expr(InsnType::Iget, field_type.clone(), &operands, payload),
        _ => lower.node(InsnType::Iput, None, &operands, payload),
    }
}

fn invoke(
    lower: &mut Lower<'_>,
    class: &ClassFile<'_>,
    instruction: Instruction<'_>,
) -> Result<(), OpError> {
    let opcode = instruction.opcode();
    let reference = instruction
        .reference(class)?
        .ok_or(OpError::Reference(instruction.mnemonic()))?;
    let descriptor: MethodDescriptor = match &reference {
        Reference::Member(member) => text(member.descriptor)?.parse()?,
        Reference::Dynamic { descriptor, .. } => text(*descriptor)?.parse()?,
        _ => return Err(OpError::Reference(instruction.mnemonic())),
    };
    let mut operands: SmallVec<[Read; 8]> = SmallVec::new();
    let receiver = opcode != 0xb8 && opcode != 0xba;
    operands
        .try_reserve(descriptor.parameters.len() + usize::from(receiver))
        .map_err(|_| OpError::Allocation)?;
    let mut words = u16::from(receiver);
    for parameter in descriptor.parameters.iter().rev() {
        let category = Category::of(parameter)?;
        words += category.words();
        operands.push(lower.pop(category)?);
    }
    if matches!(
        instruction.operands(),
        Operands::Constant { extra: Some(ConstantOperand::InterfaceSlots(count)), .. }
            if u16::from(count) != words
    ) {
        return Err(OpError::Operand(OperandProblem::InvokeInterfaceCount));
    }
    if receiver {
        operands.push(lower.pop(Category::Reference)?);
    }
    operands.reverse();
    let reference = match reference {
        Reference::Member(reference) => reference,
        Reference::Dynamic {
            bootstrap_index,
            name,
            descriptor: original_descriptor,
            ..
        } => {
            return bootstrap::dynamic(
                lower,
                class,
                instruction,
                bootstrap::DynamicSite {
                    bootstrap: bootstrap_index,
                    name,
                    descriptor: original_descriptor,
                },
                descriptor,
                &operands,
            );
        }
        _ => return Err(OpError::Reference(instruction.mnemonic())),
    };
    let dispatch = match opcode {
        0xb6 => InvokeType::Virtual,
        0xb7 if reference.name.bytes() == b"<init>" => InvokeType::Direct,
        0xb7 if reference.owner == class.name()? => InvokeType::Direct,
        0xb7 if reference.kind == ReferenceKind::InterfaceMethod => {
            return lower.effect(
                instruction,
                descriptor.return_type,
                &operands,
                FailureKind::Unsupported,
                format_args!(
                    "interface special dispatch for {}",
                    bootstrap::OriginalMember(reference)
                ),
            );
        }
        0xb7 => InvokeType::Super,
        0xb8 => InvokeType::Static,
        0xb9 => InvokeType::Interface,
        _ => return Err(OpError::Reference(instruction.mnemonic())),
    };
    let owner = class_type(reference.owner)?;
    let name = text(reference.name)?;
    let result = descriptor.return_type.clone();
    let payload = |node: &mut InsnNode| {
        node.payload.invoke_type = Some(dispatch);
        node.payload.reference = Some(Arc::new(MemberReference::Method(MethodReference {
            owner,
            name: name.into_owned(),
            descriptor,
        })));
    };
    if result == ArgType::VOID {
        lower.node(InsnType::Invoke, None, &operands, payload)
    } else {
        lower.expr(InsnType::Invoke, result, &operands, payload)
    }
}

fn typed(
    lower: &mut Lower<'_>,
    class: &ClassFile<'_>,
    instruction: Instruction<'_>,
) -> Result<(), OpError> {
    let opcode = instruction.opcode();
    let ty = if let Operands::ArrayType(kind) = instruction.operands() {
        match kind {
            4 => ArgType::BOOLEAN,
            5 => ArgType::CHAR,
            6 => ArgType::FLOAT,
            7 => ArgType::DOUBLE,
            8 => ArgType::BYTE,
            9 => ArgType::SHORT,
            10 => ArgType::INT,
            11 => ArgType::LONG,
            _ => return Err(OpError::Operand(OperandProblem::NewArrayType)),
        }
    } else {
        let Some(Reference::Class(class_name)) = instruction.reference(class)? else {
            return Err(OpError::Reference(instruction.mnemonic()));
        };
        class_type(class_name)?
    };
    let payload =
        |node: &mut InsnNode, class_type| node.payload.class_type = Some(Box::new(class_type));
    match opcode {
        0xbb if ty.is_object() => lower.expr(InsnType::NewInstance, ty.clone(), &[], |node| {
            payload(node, ty)
        }),
        0xbc | 0xbd => {
            let size = lower.pop(Category::Int)?;
            let array = ArgType::array(ty);
            lower.expr(InsnType::NewArray, array.clone(), &[size], |node| {
                payload(node, array)
            })
        }
        0xc0 => {
            let value = lower.pop(Category::Reference)?;
            lower.expr(InsnType::CheckCast, ty.clone(), &[value], |node| {
                payload(node, ty)
            })
        }
        0xc1 => {
            let value = lower.pop(Category::Reference)?;
            lower.expr(InsnType::InstanceOf, ArgType::BOOLEAN, &[value], |node| {
                payload(node, ty)
            })
        }
        0xc5 => {
            let Operands::Constant {
                extra: Some(ConstantOperand::Dimensions(dimensions)),
                ..
            } = instruction.operands()
            else {
                return Err(OpError::Operand(OperandProblem::MultiArrayDimensions));
            };
            if dimensions == 0 {
                return Err(OpError::Operand(OperandProblem::MultiArrayDimensions));
            }
            let mut element = &ty;
            let mut sizes: SmallVec<[Read; 4]> = SmallVec::new();
            sizes
                .try_reserve(usize::from(dimensions))
                .map_err(|_| OpError::Allocation)?;
            for _ in 0..dimensions {
                element = element
                    .as_array_element()
                    .ok_or(OpError::Operand(OperandProblem::MultiArrayDimensions))?;
                sizes.push(lower.pop(Category::Int)?);
            }
            sizes.reverse();
            lower.expr(InsnType::NewArray, ty.clone(), &sizes, |node| {
                payload(node, ty)
            })
        }
        _ => Err(OpError::Reference(instruction.mnemonic())),
    }
}

#[cfg(test)]
#[path = "../../../../../tests/internal/jvm_ops.rs"]
mod tests;
