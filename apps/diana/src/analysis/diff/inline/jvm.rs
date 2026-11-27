//! A straight-line JVM fragment must replace one original local static call.
//! Byte subsequences alone do not establish its parameters or its continuation.

use std::collections::{HashMap, HashSet};

use diana_decomp::ir::{ArgType, MethodDescriptor, PrimitiveType};

use crate::analysis::diff::index::{Body, Method};

use super::{InlineSite, is_call};

const STATIC: u32 = 0x0008;
const SYNCHRONIZED: u32 = 0x0020;
const STRICT: u32 = 0x0800;

/// Positional evidence outside the original call. Derive it once per caller
/// pair so repeated calls never trigger repeated whole-method comparisons.
pub(in crate::analysis::diff) struct Outside {
    prefix: usize,
    suffix: usize,
}

impl Outside {
    pub(super) fn new(old: &Body, new: &Body) -> Self {
        let common = old.code.len().min(new.code.len());
        let prefix = (0..common).take_while(|&i| same(old, i, new, i)).count();
        let suffix = (0..common)
            .take_while(|&delta| {
                same(
                    old,
                    old.code.len() - 1 - delta,
                    new,
                    new.code.len() - 1 - delta,
                )
            })
            .count();
        Self { prefix, suffix }
    }
}

fn same(a: &Body, i: usize, b: &Body, j: usize) -> bool {
    a.code[i].known_operands
        && b.code[j].known_operands
        && a.code[i].opcode == b.code[j].opcode
        && a.operand(i) == b.operand(j)
}

pub(super) struct Eligible {
    pub(super) count: usize,
    pub(super) descriptor: MethodDescriptor,
}

pub(super) fn eligible(method: &Method, body: &Body) -> Option<Eligible> {
    let (last, prefix) = body.code.split_last()?;
    if method.access & (STATIC | SYNCHRONIZED) != STATIC
        || matches!(method.name.as_str(), "<init>" | "<clinit>")
        || prefix.len() < 4
        || !complete(method, body)
    {
        return None;
    }
    let descriptor: MethodDescriptor = method.prototype.parse().ok()?;
    (last.opcode == return_opcode(&descriptor)?).then_some(Eligible {
        count: prefix.len(),
        descriptor,
    })
}

fn complete(method: &Method, body: &Body) -> bool {
    method.issues.is_empty() && body.complete_control && body.protection.is_empty()
}

pub(super) fn find(
    before: &Method,
    helper: &Method,
    after: &Method,
    invoke: usize,
    outside: &Outside,
) -> Option<InlineSite> {
    let caller = before.body.as_ref()?;
    let callee = helper.body.as_ref()?;
    let target = after.body.as_ref()?;
    let eligible = eligible(helper, callee)?;
    let count = eligible.count;
    // A static call across a different class can initiate that class. A
    // synchronized helper also introduces effects outside its instruction list.
    let original_class = before.at.rsplit_once(":m")?.0;
    let helper_class = helper.at.rsplit_once(":m")?.0;
    if original_class != helper_class
        || before.owner != helper.owner
        || after.owner != helper.owner
        || before.prototype != after.prototype
        || caller.jvm_major != target.jvm_major
        || caller.jvm_major != callee.jvm_major
        || matches!(before.name.as_str(), "<init>" | "<clinit>")
        || matches!(after.name.as_str(), "<init>" | "<clinit>")
        || (before.access ^ after.access) & (STATIC | SYNCHRONIZED | STRICT) != 0
        || (before.access ^ helper.access) & STRICT != 0
        || !complete(before, caller)
        || !complete(after, target)
        || !is_call(caller, invoke)
    {
        return None;
    }
    let identity = helper.identity();
    if caller.callee(invoke) != Some(identity.as_str())
        || callee.call_names().any(|name| name == identity)
        || target.call_names().any(|name| name == identity)
    {
        return None;
    }

    let parameters = &eligible.descriptor.parameters;
    let start = invoke.checked_sub(parameters.len())?;
    let successor = invoke + 1;
    let end = start.checked_add(count)?;
    let suffix = caller.code.len().checked_sub(successor)?;
    if start > outside.prefix
        || suffix > outside.suffix
        || target.code.len().checked_sub(end)? != suffix
    {
        return None;
    }

    let mut arguments = HashMap::with_capacity(parameters.len());
    let mut argument_slot = 0u16;
    for (index, ty) in (start..invoke).zip(parameters) {
        let value = Kind::of(ty)?;
        if caller.code[index].opcode != value.load() {
            return None;
        }
        arguments.insert(argument_slot, local(caller, index)?);
        argument_slot = argument_slot.checked_add(u16::from(value.width()))?;
    }

    for (i, j) in (0..count).zip(start..end) {
        let source = callee.code.get(i)?;
        let expanded = target.code.get(j)?;
        if !source.known_operands || !expanded.known_operands || source.opcode != expanded.opcode {
            return None;
        }
        if matches!(source.opcode, 0x15..=0x19) {
            if arguments.get(&local(callee, i)?).copied()? != local(target, j)? {
                return None;
            }
        } else if !callee.registers(i).is_empty() || !same(callee, i, target, j) {
            // A callee-private local store needs a separate lifetime proof.
            return None;
        }
    }

    let inner_calls: HashSet<_> = callee.call_names().collect();
    let independently_called = |point| {
        caller
            .callee(point)
            .is_some_and(|call| call != identity && !inner_calls.contains(call))
    };
    if !(0..start).any(independently_called)
        || !(successor..caller.code.len()).any(independently_called)
    {
        return None;
    }

    let caller_descriptor: MethodDescriptor = before.prototype.parse().ok()?;
    certify(callee, &eligible.descriptor, true)?;
    certify(caller, &caller_descriptor, before.access & STATIC != 0)?;
    certify(target, &caller_descriptor, after.access & STATIC != 0)?;

    Some(InlineSite {
        start,
        count,
        corroborates_outer: true,
    })
}

fn local(body: &Body, i: usize) -> Option<u16> {
    let [local] = body.registers(i) else {
        return None;
    };
    Some(*local)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Kind {
    Int,
    Long,
    Float,
    Double,
    Reference,
}

impl Kind {
    fn of(ty: &ArgType) -> Option<Self> {
        use PrimitiveType::*;
        Some(match ty.as_primitive() {
            Some(Boolean | Byte | Char | Short | Int) => Self::Int,
            Some(Long) => Self::Long,
            Some(Float) => Self::Float,
            Some(Double) => Self::Double,
            Some(Object | Array) => Self::Reference,
            None if ty.is_reference() => Self::Reference,
            _ => return None,
        })
    }

    fn load(self) -> u8 {
        match self {
            Self::Int => 0x15,
            Self::Long => 0x16,
            Self::Float => 0x17,
            Self::Double => 0x18,
            Self::Reference => 0x19,
        }
    }

    fn width(self) -> u8 {
        if matches!(self, Self::Long | Self::Double) {
            2
        } else {
            1
        }
    }
}

const LANES: [Kind; 5] = [
    Kind::Int,
    Kind::Long,
    Kind::Float,
    Kind::Double,
    Kind::Reference,
];

fn return_opcode(descriptor: &MethodDescriptor) -> Option<u8> {
    Some(if descriptor.return_type == ArgType::VOID {
        0xb1
    } else {
        Kind::of(&descriptor.return_type)?.load() - 0x15 + 0xac
    })
}

/// Stack categories are used solely to establish this diff witness. The
/// proof accepts a sequence of original straight-line operations with known
/// operands, rejects reads of another frame's stack or uninitialized locals,
/// and checks the return's category. It does not resolve a classpath.
struct Frame {
    locals: Vec<Option<Kind>>,
    stack: Vec<Kind>,
    words: usize,
    max_stack: usize,
}

fn certify(body: &Body, descriptor: &MethodDescriptor, static_method: bool) -> Option<()> {
    let (last, prefix) = body.code.split_last()?;
    if last.opcode != return_opcode(descriptor)? {
        return None;
    }
    let mut frame = Frame {
        locals: vec![None; usize::from(body.frame[0])],
        stack: Vec::new(),
        words: 0,
        max_stack: usize::from(body.frame[2]),
    };
    let mut slot = 0;
    if !static_method {
        frame.store(slot, Kind::Reference)?;
        slot += 1;
    }
    for parameter in &descriptor.parameters {
        let kind = Kind::of(parameter)?;
        frame.store(slot, kind)?;
        slot += usize::from(kind.width());
    }
    for (i, instruction) in prefix.iter().enumerate() {
        if !instruction.known_operands {
            return None;
        }
        frame.apply(body, i)?;
    }
    if descriptor.return_type != ArgType::VOID {
        frame.pop(Kind::of(&descriptor.return_type)?)?;
    }
    frame.stack.is_empty().then_some(())
}

impl Frame {
    fn push(&mut self, value: Kind) -> Option<()> {
        self.words += usize::from(value.width());
        if self.words > self.max_stack {
            return None;
        }
        self.stack.push(value);
        Some(())
    }

    fn top(&mut self) -> Option<Kind> {
        let value = self.stack.pop()?;
        self.words -= usize::from(value.width());
        Some(value)
    }

    fn pop(&mut self, value: Kind) -> Option<()> {
        (self.top()? == value).then_some(())
    }

    fn store(&mut self, index: usize, kind: Kind) -> Option<()> {
        if index > 0
            && self
                .locals
                .get(index - 1)?
                .is_some_and(|prior| prior.width() == 2)
        {
            self.locals[index - 1] = None;
        }
        *self.locals.get_mut(index)? = Some(kind);
        if kind.width() == 2 {
            *self.locals.get_mut(index + 1)? = None;
        }
        Some(())
    }

    fn transform(&mut self, inputs: &[Kind], result: Kind) -> Option<()> {
        for &input in inputs.iter().rev() {
            self.pop(input)?;
        }
        self.push(result)
    }

    fn apply(&mut self, body: &Body, index: usize) -> Option<()> {
        use Kind::*;
        let opcode = body.code[index].opcode;
        match opcode {
            0x00 => Some(()),
            0x01 => self.push(Reference),
            0x02..=0x08 | 0x10..=0x11 => self.push(Int),
            0x09..=0x0a => self.push(Long),
            0x0b..=0x0d => self.push(Float),
            0x0e..=0x0f => self.push(Double),
            0x12 | 0x14 => self.push(match (opcode, body.operand_tag(index)?) {
                (0x12, b'I') => Int,
                (0x12, b'L') => Float,
                (0x12, b't' | b's' | b'h' | b'p') => Reference,
                (0x14, b'J') => Long,
                (0x14, b'L') => Double,
                _ => return None,
            }),
            0x15..=0x19 => {
                let slot = usize::from(local(body, index)?);
                let expected = LANES[(opcode - 0x15) as usize];
                (*self.locals.get(slot)? == Some(expected)).then_some(())?;
                self.push(expected)
            }
            0x36..=0x3a => {
                let slot = usize::from(local(body, index)?);
                let expected = LANES[(opcode - 0x36) as usize];
                self.pop(expected)?;
                self.store(slot, expected)
            }
            0x57 => (self.top()?.width() == 1).then_some(()),
            0x58 => {
                if self.top()?.width() == 1 {
                    (self.top()?.width() == 1).then_some(())?;
                }
                Some(())
            }
            0x59 => {
                let value = *self.stack.last()?;
                (value.width() == 1).then_some(())?;
                self.push(value)
            }
            0x60..=0x73 => {
                let lane = LANES[(opcode - 0x60) as usize % 4];
                self.transform(&[lane, lane], lane)
            }
            0x74..=0x77 => {
                let lane = LANES[(opcode - 0x74) as usize];
                self.transform(&[lane], lane)
            }
            0x78..=0x7d => {
                let lane = LANES[(opcode - 0x78) as usize % 2];
                self.transform(&[lane, Int], lane)
            }
            0x7e..=0x83 => {
                let lane = LANES[(opcode - 0x7e) as usize % 2];
                self.transform(&[lane, lane], lane)
            }
            0x84 => {
                let slot = usize::from(local(body, index)?);
                (*self.locals.get(slot)? == Some(Int)).then_some(())
            }
            0x85..=0x93 => {
                const CONVERSIONS: [(Kind, Kind); 15] = [
                    (Int, Long),
                    (Int, Float),
                    (Int, Double),
                    (Long, Int),
                    (Long, Float),
                    (Long, Double),
                    (Float, Int),
                    (Float, Long),
                    (Float, Double),
                    (Double, Int),
                    (Double, Long),
                    (Double, Float),
                    (Int, Int),
                    (Int, Int),
                    (Int, Int),
                ];
                let (input, result) = CONVERSIONS[(opcode - 0x85) as usize];
                self.transform(&[input], result)
            }
            0x94..=0x98 => {
                let lane = match opcode {
                    0x94 => Long,
                    0x95 | 0x96 => Float,
                    _ => Double,
                };
                self.transform(&[lane, lane], Int)
            }
            0xb2..=0xb5 => {
                let (_, ty) = body
                    .field_reference(index)?
                    .split_once("->")?
                    .1
                    .split_once(':')?;
                let ty: ArgType = ty.parse().ok()?;
                let value = Kind::of(&ty)?;
                match opcode {
                    0xb2 => self.push(value),
                    0xb3 => self.pop(value),
                    0xb4 => self.transform(&[Reference], value),
                    0xb5 => {
                        self.pop(value)?;
                        self.pop(Reference)
                    }
                    _ => None,
                }
            }
            0xb6 | 0xb8 | 0xb9 => {
                let (_, call) = body.callee(index)?.split_once("->")?;
                let site = call.find('(')?;
                let method: MethodDescriptor = call.get(site..)?.parse().ok()?;
                for parameter in method.parameters.iter().rev() {
                    self.pop(Kind::of(parameter)?)?;
                }
                if opcode != 0xb8 {
                    self.pop(Reference)?;
                }
                if method.return_type != ArgType::VOID {
                    self.push(Kind::of(&method.return_type)?)?;
                }
                Some(())
            }
            _ => None,
        }
    }
}
