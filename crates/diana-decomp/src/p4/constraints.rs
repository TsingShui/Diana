mod build;
mod initialize;
mod operations;
mod origins;
mod references;
mod scalars;
mod terminals;

pub(super) use build::collect;

use super::build::*;
use super::*;
use crate::p1::CheckedReferenceFact as Reference;
use crate::p2::*;
use crate::p3::{EntryRole, SsaMethod, StateId, ValueId, ValueOrigin};
use crate::raw::ReferenceSlot;
use crate::substrate::owned;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RuleKind {
    Copy,
    Phi,
}
pub(super) struct Rule {
    pub target: ValueId,
    pub inputs: Vec<StateId>,
    pub kind: RuleKind,
}
pub(super) struct ArraySite {
    pub operation: OpId,
    pub array: StateId,
    pub output: Option<ValueId>,
}
pub(super) struct CastBound {
    pub operation: OpId,
    pub receiver: StateId,
    pub target: TypeNameId,
    /// The original type identity this check resolves to.
    pub target_id: crate::TypeId,
}
/// One original `instanceof` whose result is available and whose type resolved.
pub(super) struct InstanceBound {
    pub operation: OpId,
    /// The reference value the result was tested on.
    pub receiver: StateId,
    pub target: crate::TypeId,
}
pub(super) struct Work {
    pub arrays: Vec<ArraySite>,
    pub casts: Vec<CastBound>,
    pub instances: Vec<InstanceBound>,
    pub rules: Vec<Rule>,
    pub compatible: Vec<(usize, usize)>,
}
fn copy_units(units: &[u16]) -> Result<Vec<u16>, TypeError> {
    let mut result = Vec::new();
    owned::reserve(&mut result, units.len())?;
    result.extend_from_slice(units);
    Ok(result)
}
fn reference_name(
    f: &mut TypeFacts,
    op: &Operation,
    slot: ReferenceSlot,
    part: SignaturePart,
    original: Option<crate::TypeId>,
    descriptor: crate::input::DexString<'_>,
) -> Result<TypeNameId, TypeError> {
    add_name(
        f,
        owned::copy_units(descriptor)?,
        TypeOrigin::Reference {
            pc: op.pc(),
            slot,
            part,
            original,
        },
    )
}
fn seed(f: &mut TypeFacts, value: ValueId, name: TypeNameId) -> Result<(), TypeError> {
    let family = name_family(f, name);
    let range = f.names[name.0 as usize]
        .units
        .first()
        .copied()
        .and_then(tag_range);
    let fact = &mut f.values[value.0 as usize];
    push(&mut fact.types, name)?;
    if let Some(family) = family {
        fact.families = fact.families.intersect(mask(family));
    }
    if let Some(range) = range {
        fact.range = Some(range);
        fact.range_unknown = false;
    }
    Ok(())
}
fn primitive(f: &mut TypeFacts, value: ValueId, tag: u8) -> Result<(), TypeError> {
    let name = intrinsic(f, &[tag])?;
    seed(f, value, name)
}
fn output(s: &crate::p3::SsaFacts, op: OpId) -> Option<ValueId> {
    s.operation_outputs(op)?
        .first()
        .and_then(|o| s.output_value(o))
}
fn use_index(s: &crate::p3::SsaFacts, op: OpId, argument: usize) -> Option<usize> {
    let site = s.operation_site(op)?;
    (argument < site.use_len() as usize).then_some(site.use_start() as usize + argument)
}
fn arg(f: &mut TypeFacts, s: &crate::p3::SsaFacts, op: OpId, n: usize, families: Families) {
    if let Some(i) = use_index(s, op, n) {
        f.uses[i].requirement.families = f.uses[i].requirement.families.intersect(families);
    }
}
fn arg_type(
    f: &mut TypeFacts,
    s: &crate::p3::SsaFacts,
    op: OpId,
    n: usize,
    name: TypeNameId,
) -> Result<(), TypeError> {
    if let Some(i) = use_index(s, op, n) {
        if let Some(family) = name_family(f, name) {
            f.uses[i].requirement.families = f.uses[i].requirement.families.intersect(mask(family));
        }
        push(&mut f.uses[i].requirement.types, name)?;
    }
    Ok(())
}
fn missing_args(f: &mut TypeFacts, s: &crate::p3::SsaFacts, op: OpId) {
    if let Some(site) = s.operation_site(op) {
        for i in site.use_start()..site.use_start() + site.use_len() {
            f.uses[i as usize].requirement.missing = Some(MissingFact::ReferenceDeclaration);
        }
    }
}
fn input_rule(
    s: &crate::p3::SsaFacts,
    work: &mut Work,
    value: ValueId,
    op: OpId,
    argument: usize,
    kind: RuleKind,
) -> Result<(), TypeError> {
    let mut inputs = Vec::new();
    if let Some(i) = use_index(s, op, argument) {
        push(&mut inputs, s.uses()[i].state())?;
    }
    push(
        &mut work.rules,
        Rule {
            target: value,
            inputs,
            kind,
        },
    )
}
fn value_kind(kind: ValueKind) -> Families {
    match kind {
        ValueKind::Word => Families::NUMBER,
        ValueKind::Wide => Families::WIDE,
        ValueKind::Object => mask(Family::Reference),
        _ => mask(Family::Integral),
    }
}
fn narrow_tag(kind: ValueKind) -> Option<u8> {
    Some(match kind {
        ValueKind::Boolean => b'Z',
        ValueKind::Byte => b'B',
        ValueKind::Char => b'C',
        ValueKind::Short => b'S',
        _ => return None,
    })
}
fn unary_tags(op: UnaryOp) -> (u8, u8) {
    use UnaryOp::*;
    match op {
        NegInt | NotInt => (b'I', b'I'),
        NegLong | NotLong => (b'J', b'J'),
        NegFloat => (b'F', b'F'),
        NegDouble => (b'D', b'D'),
        IntToLong => (b'I', b'J'),
        IntToFloat => (b'I', b'F'),
        IntToDouble => (b'I', b'D'),
        LongToInt => (b'J', b'I'),
        LongToFloat => (b'J', b'F'),
        LongToDouble => (b'J', b'D'),
        FloatToInt => (b'F', b'I'),
        FloatToLong => (b'F', b'J'),
        FloatToDouble => (b'F', b'D'),
        DoubleToInt => (b'D', b'I'),
        DoubleToLong => (b'D', b'J'),
        DoubleToFloat => (b'D', b'F'),
        IntToByte => (b'I', b'B'),
        IntToChar => (b'I', b'C'),
        IntToShort => (b'I', b'S'),
    }
}
