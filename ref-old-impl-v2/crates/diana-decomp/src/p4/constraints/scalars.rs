//! Numeric family, literal and copy constraints preserve the original operands.

use super::*;

pub(super) fn constant(
    f: &mut TypeFacts,
    out: Option<ValueId>,
    bits: u64,
    width: crate::SlotWidth,
) -> Result<(), TypeError> {
    if let Some(v) = out {
        let fact = &mut f.values[v.0 as usize];
        let literal = Bits { value: bits, width };
        fact.literal = Some(literal);
        fact.families = fact.families.intersect(literal_families(literal));
        fact.mixed_literal = false;
        fact.range_unknown = false;
        // Reinterpret the original two's-complement bits; this is not a numeric conversion.
        let value = if width == crate::SlotWidth::One {
            i64::from(bits as u32 as i32)
        } else {
            bits as i64
        };
        fact.range = Some(IntegralRange {
            min: value,
            max: value,
        });
    }
    Ok(())
}

pub(super) fn copy(
    ssa: &SsaMethod<'_>,
    f: &mut TypeFacts,
    work: &mut Work,
    operation: OpId,
    kind: MoveKind,
    out: Option<ValueId>,
) -> Result<(), TypeError> {
    let s = ssa.facts();
    if let Some(index) = use_index(s, operation, 0) {
        f.uses[index].requirement.meaning = ReadMeaning::CopyBits;
    }
    let families = match kind {
        MoveKind::Word => Families::NUMBER,
        MoveKind::Wide => Families::WIDE,
        MoveKind::Object => mask(Family::Reference),
    };
    arg(f, s, operation, 0, families);
    if let Some(v) = out {
        let fact = &mut f.values[v.0 as usize];
        fact.families = fact.families.intersect(families);
        fact.mixed_literal = false;
        fact.range_unknown = false;
        input_rule(s, work, v, operation, 0, RuleKind::Copy)?;
    }
    Ok(())
}

pub(super) fn unary(
    s: &crate::p3::SsaFacts,
    f: &mut TypeFacts,
    operation: OpId,
    unary: UnaryOp,
    out: Option<ValueId>,
) -> Result<(), TypeError> {
    let (src, dst) = unary_tags(unary);
    arg(
        f,
        s,
        operation,
        0,
        mask(
            tag_family(u16::from(src))
                .ok_or(TypeError::InternalInvariant("unary source family"))?,
        ),
    );
    if let Some(v) = out {
        primitive(f, v, dst)?;
    }
    Ok(())
}

pub(super) fn binary(
    s: &crate::p3::SsaFacts,
    f: &mut TypeFacts,
    operation: OpId,
    binary: BinaryOp,
    out: Option<ValueId>,
) -> Result<(), TypeError> {
    let tag = if binary.integer() {
        if binary.width() == crate::SlotWidth::Two {
            b'J'
        } else {
            b'I'
        }
    } else if binary.width() == crate::SlotWidth::Two {
        b'D'
    } else {
        b'F'
    };
    let family = tag_family(u16::from(tag)).ok_or(TypeError::InternalInvariant("binary family"))?;
    arg(f, s, operation, 0, mask(family));
    arg(
        f,
        s,
        operation,
        1,
        if binary.right_width() != binary.left_width() {
            mask(Family::Integral)
        } else {
            mask(family)
        },
    );
    if let Some(v) = out {
        primitive(f, v, tag)?;
    }
    Ok(())
}

pub(super) fn compare(
    s: &crate::p3::SsaFacts,
    f: &mut TypeFacts,
    operation: OpId,
    cmp: CompareOp,
    out: Option<ValueId>,
) -> Result<(), TypeError> {
    let family = match cmp {
        CompareOp::CmpLong => Family::Long,
        CompareOp::CmplDouble | CompareOp::CmpgDouble => Family::Double,
        _ => Family::Float,
    };
    arg(f, s, operation, 0, mask(family));
    arg(f, s, operation, 1, mask(family));
    if let Some(v) = out {
        primitive(f, v, b'I')?;
        f.values[v.0 as usize].range = Some(IntegralRange { min: -1, max: 1 });
    }
    Ok(())
}
