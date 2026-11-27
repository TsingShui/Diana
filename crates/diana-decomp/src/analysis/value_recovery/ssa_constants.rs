//! Sparse constant propagation over the complete SSA CFG.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{analysis::SsaVar, ArgType, InsnArg, InsnNode, InsnType, Utf16String, CFG};

use super::constant::ConstantEvaluator;

pub(super) struct SparseConstantPropagation<'a> {
    cfg: &'a CFG,
}

struct ValueState {
    value: SsaVar,
    fact: Lattice<InsnArg>,
    unique: bool,
    settled: bool,
}

type Readers = foldhash::HashMap<usize, smallvec::SmallVec<[usize; 2]>>;
type Slots = foldhash::HashMap<SsaVar, usize>;

impl<'a> SparseConstantPropagation<'a> {
    pub(super) fn new(cfg: &'a CFG) -> Self {
        Self { cfg }
    }

    pub(super) fn solve(&self) -> BTreeMap<SsaVar, InsnArg> {
        let instructions = || self.cfg.blocks.values().flat_map(|block| &block.insns);
        if !instructions().any(Self::may_seed) {
            return BTreeMap::new();
        }
        let mut slots = Slots::default();
        let mut states: Vec<ValueState> = Vec::new();
        let mut equations = Vec::new();
        for instruction in instructions() {
            let Some(value) = instruction.result.as_ref().and_then(SsaVar::from_reg) else {
                continue;
            };
            let index = *slots
                .entry(value)
                .and_modify(|&mut index| states[index].unique = false)
                .or_insert_with(|| {
                    let index = states.len();
                    states.push(ValueState {
                        value,
                        fact: Lattice::Unknown,
                        unique: true,
                        settled: false,
                    });
                    index
                });
            equations.push((index, instruction));
        }

        let mut readers = Readers::default();
        let mut pending = BTreeSet::new();
        for (position, &(index, instruction)) in equations.iter().enumerate() {
            if states[index].settled {
                continue;
            }
            let changed = Self::update(instruction, index, &slots, &mut states);
            let revisit = changed && states[index].fact.is_unkeyed();
            let mut settled = true;
            if !matches!(states[index].fact, Lattice::Varying) {
                for argument in Self::inputs(instruction) {
                    if let Some(source) =
                        Self::slot(argument, &slots).filter(|&i| !states[i].settled)
                    {
                        let users = readers.entry(source).or_default();
                        if users.last() != Some(&position) {
                            users.push(position);
                        }
                        settled = false;
                    }
                }
            }
            if changed {
                if revisit {
                    // Comparing an unkeyed constant with itself widens it on
                    // its next evaluation, even when it has no input register.
                    pending.insert(position);
                }
                if let Some(users) = readers.get(&index) {
                    pending.extend(users);
                }
            }
            states[index].settled = matches!(states[index].fact, Lattice::Varying)
                || (states[index].unique && settled && !revisit);
        }

        // Preserve the original CFG sweep order. A later reader observes a
        // change this round and an earlier reader observes it on the next round.
        // Equal constants retain the first observed wrapper and instruction ID.
        let mut deferred = BTreeSet::new();
        while !pending.is_empty() {
            while let Some(position) = pending.pop_first() {
                let (index, instruction) = equations[position];
                if Self::update(instruction, index, &slots, &mut states) {
                    if states[index].fact.is_unkeyed() {
                        deferred.insert(position);
                    }
                    if let Some(users) = readers.get(&index) {
                        for &reader in users {
                            if reader > position {
                                pending.insert(reader);
                            } else {
                                deferred.insert(reader);
                            }
                        }
                    }
                }
            }
            std::mem::swap(&mut pending, &mut deferred);
        }

        states
            .into_iter()
            .filter_map(|row| match row.fact {
                Lattice::Constant(constant) => Some((row.value, constant)),
                _ => None,
            })
            .collect()
    }

    /// Advance through bottom, a constant, and the absorbing varying state.
    fn update(
        instruction: &InsnNode,
        index: usize,
        slots: &Slots,
        states: &mut [ValueState],
    ) -> bool {
        if matches!(states[index].fact, Lattice::Varying) {
            return false;
        }
        let next = Self::evaluate(instruction, slots, states);
        let Some(update) = states[index].fact.advance(next) else {
            return false;
        };
        states[index].fact = update;
        true
    }

    /// Register-only equations cannot discover their first constant without
    /// a literal, a wrapped constant or a constant-producing instruction.
    fn may_seed(instruction: &InsnNode) -> bool {
        matches!(instruction.insn_type, InsnType::Const | InsnType::ConstStr)
            || (instruction.args.is_empty() && ConstantEvaluator::accepts(instruction.insn_type))
            || Self::inputs(instruction)
                .iter()
                .any(|arg| !matches!(arg, InsnArg::Reg(_)))
    }

    fn inputs(instruction: &InsnNode) -> &[InsnArg] {
        match instruction.insn_type {
            InsnType::Move => &instruction.args[..instruction.args.len().min(1)],
            InsnType::Const | InsnType::ConstStr => &[],
            InsnType::Phi => &instruction.args,
            kind if ConstantEvaluator::accepts(kind) => &instruction.args,
            _ => &[],
        }
    }

    fn slot(argument: &InsnArg, slots: &Slots) -> Option<usize> {
        slots
            .get(&SsaVar::from_reg(argument.as_register()?)?)
            .copied()
    }

    fn argument<'b>(
        argument: &'b InsnArg,
        slots: &Slots,
        states: &'b [ValueState],
    ) -> Lattice<&'b InsnArg> {
        match argument {
            InsnArg::Lit(_) => Lattice::Constant(argument),
            InsnArg::Reg(_) => Self::slot(argument, slots).map_or(Lattice::Varying, |index| {
                match &states[index].fact {
                    Lattice::Unknown => Lattice::Unknown,
                    Lattice::Varying => Lattice::Varying,
                    Lattice::Constant(value) => Lattice::Constant(value),
                }
            }),
            InsnArg::Wrapped(instruction)
                if matches!(instruction.insn_type, InsnType::Const | InsnType::ConstStr) =>
            {
                Lattice::Constant(argument)
            }
            InsnArg::Wrapped(_) => Lattice::Varying,
        }
    }

    fn evaluate<'b>(
        instruction: &'b InsnNode,
        slots: &Slots,
        states: &'b [ValueState],
    ) -> Lattice<Cow<'b, InsnArg>> {
        match instruction.insn_type {
            InsnType::Const | InsnType::ConstStr => {
                Lattice::Constant(Cow::Owned(InsnArg::wrap(instruction.clone())))
            }
            InsnType::Move => instruction
                .args
                .first()
                .map_or(Lattice::Varying, |argument| {
                    Self::argument(argument, slots, states).map(Cow::Borrowed)
                }),
            InsnType::Phi => {
                let mut constant = None;
                for argument in &instruction.args {
                    match Self::argument(argument, slots, states) {
                        Lattice::Unknown => continue,
                        Lattice::Varying => return Lattice::Varying,
                        Lattice::Constant(value) => match constant {
                            Some(prior) if !same_constant(prior, value) => return Lattice::Varying,
                            Some(_) => {}
                            None => constant = Some(value),
                        },
                    }
                }
                constant.map_or(Lattice::Unknown, |value| {
                    Lattice::Constant(Cow::Borrowed(value))
                })
            }
            kind if ConstantEvaluator::accepts(kind) => {
                let mut arguments = smallvec::SmallVec::<[InsnArg; 2]>::new();
                for argument in &instruction.args {
                    match Self::argument(argument, slots, states) {
                        Lattice::Unknown => return Lattice::Unknown,
                        Lattice::Varying => return Lattice::Varying,
                        Lattice::Constant(value) => arguments.push(value.clone()),
                    }
                }
                ConstantEvaluator::fold(
                    kind,
                    &instruction.payload,
                    instruction.result.as_ref(),
                    &arguments,
                )
                .map_or(Lattice::Varying, |value| {
                    Lattice::Constant(Cow::Owned(value))
                })
            }
            _ => Lattice::Varying,
        }
    }
}

enum Lattice<T> {
    Unknown,
    Constant(T),
    Varying,
}

impl<T> Lattice<T> {
    fn map<U>(self, f: impl FnOnce(T) -> U) -> Lattice<U> {
        match self {
            Self::Unknown => Lattice::Unknown,
            Self::Constant(c) => Lattice::Constant(f(c)),
            Self::Varying => Lattice::Varying,
        }
    }
}

impl Lattice<InsnArg> {
    fn is_unkeyed(&self) -> bool {
        matches!(self, Self::Constant(value) if constant_key(value).is_none())
    }

    fn advance(&self, next: Lattice<Cow<'_, InsnArg>>) -> Option<Self> {
        match (self, next) {
            (Self::Varying, _) | (_, Lattice::Unknown) => None,
            (_, Lattice::Varying) => Some(Self::Varying),
            (Self::Unknown, Lattice::Constant(value)) => Some(Self::Constant(value.into_owned())),
            (Self::Constant(first), Lattice::Constant(next)) => {
                (!same_constant(first, &next)).then_some(Self::Varying)
            }
        }
    }
}

fn same_constant(left: &InsnArg, right: &InsnArg) -> bool {
    match (constant_key(left), constant_key(right)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

#[derive(PartialEq, Eq)]
enum ConstantKey<'a> {
    Literal(i64, &'a ArgType),
    String(&'a Utf16String),
}

fn constant_key(value: &InsnArg) -> Option<ConstantKey<'_>> {
    match value {
        InsnArg::Lit(literal) => Some(ConstantKey::Literal(literal.value, &literal.ty)),
        InsnArg::Wrapped(instruction) if instruction.insn_type == InsnType::Const => {
            let literal = instruction.args.first()?.as_literal()?;
            Some(ConstantKey::Literal(
                literal.value,
                instruction
                    .result
                    .as_ref()
                    .map(|result| &result.ty)
                    .unwrap_or(&literal.ty),
            ))
        }
        InsnArg::Wrapped(instruction) if instruction.insn_type == InsnType::ConstStr => Some(
            ConstantKey::String(instruction.payload.string_value.as_deref()?),
        ),
        InsnArg::Reg(_) | InsnArg::Wrapped(_) => None,
    }
}
#[cfg(test)]
#[path = "../../../tests/internal/ssa_constants.rs"]
mod tests;
