//! Exact literal, machine-type and arithmetic-operator spellings.

use crate::p4::{Bits, Family, Interpretation};
use crate::p5;
use crate::p9::{RenderError, buffer::Buffer};

pub(in crate::p9) fn machine_type(
    out: &mut Buffer,
    ty: p5::StorageType,
) -> Result<(), RenderError> {
    match ty {
        p5::StorageType::Family { family, .. } => out.put(match family {
            Family::Integral => "int",
            Family::Long => "long",
            Family::Float => "float",
            Family::Double => "double",
            Family::Reference => "reference",
        }),
        p5::StorageType::NumericBits { width, families } => {
            out.fmt(format_args!("bits<{width:?}; {families:?}>"))
        }
    }
}
pub(super) fn literal(
    out: &mut Buffer,
    bits: Bits,
    view: Interpretation,
) -> Result<(), RenderError> {
    let family = match view {
        Interpretation::Null => return out.put("null"),
        Interpretation::Literal { family, .. } | Interpretation::Direct(family) => Some(family),
        Interpretation::IntegralView { .. } => Some(Family::Integral),
        _ => None,
    };
    let word =
        match family {
            Some(Family::Integral | Family::Float) => {
                if bits.width != crate::SlotWidth::One {
                    return Err(RenderError::Invariant(
                        "word literal lost its delivered width",
                    ));
                }
                Some(u32::try_from(bits.value).map_err(|_| {
                    RenderError::Invariant("word literal exceeds its delivered width")
                })?)
            }
            _ => None,
        };
    if matches!(family, Some(Family::Long | Family::Double)) && bits.width != crate::SlotWidth::Two
    {
        return Err(RenderError::Invariant(
            "wide literal lost its delivered width",
        ));
    }
    match (family, word) {
        (Some(Family::Integral), Some(word)) => {
            out.fmt(format_args!("{}", i32::from_le_bytes(word.to_le_bytes())))
        }
        (Some(Family::Long), _) => out.fmt(format_args!(
            "{}L",
            i64::from_le_bytes(bits.value.to_le_bytes())
        )),
        (Some(Family::Float), Some(word)) if f32::from_bits(word).is_finite() => {
            out.fmt(format_args!("{:?}f", f32::from_bits(word)))
        }
        (Some(Family::Double), _) if f64::from_bits(bits.value).is_finite() => {
            out.fmt(format_args!("{:?}d", f64::from_bits(bits.value)))
        }
        // The bits are exact but no Java literal states them: a non-finite
        // float or a value whose interpretation the input never established.
        // Print the original bits with the fact that qualifies them, instead of
        // naming an external constant or inventing a conversion.
        _ => {
            out.fmt(format_args!("0x{:x}", bits.value))?;
            match family {
                Some(family) => out.fmt(format_args!(" /* {family:?}, width {:?} */", bits.width)),
                None => out.fmt(format_args!(
                    " /* width {:?}, unestablished interpretation */",
                    bits.width
                )),
            }
        }
    }
}
/// One payload element as its exact Java literal. The original bits are never
/// changed: a bit pattern these forms cannot state keeps the payload statement,
/// which P7 selects only for elements that print exactly.
pub(super) fn element_literal(
    out: &mut Buffer,
    element: crate::p4::ArrayElementType,
    bits: u64,
) -> Result<(), RenderError> {
    use crate::p4::ArrayElementType::*;
    match element {
        Boolean => match bits {
            0 => out.put("false")?,
            1 => out.put("true")?,
            _ => {
                return Err(RenderError::Invariant("boolean element lost its 0/1 value"));
            }
        },
        Byte => out.fmt(format_args!("{}", bits as u8 as i8))?,
        Short => out.fmt(format_args!("{}", bits as u16 as i16))?,
        Char => char_literal(out, bits as u16)?,
        Int => out.fmt(format_args!("{}", bits as u32 as i32))?,
        Long => out.fmt(format_args!("{}L", bits as i64))?,
        Float => {
            let value = f32::from_bits(bits as u32);
            if !value.is_finite() {
                return Err(RenderError::Invariant(
                    "float element has no exact decimal spelling",
                ));
            }
            // The shortest decimal for a finite float round-trips to these bits.
            out.fmt(format_args!("{value:?}f"))?;
        }
        Double => {
            let value = f64::from_bits(bits);
            if !value.is_finite() {
                return Err(RenderError::Invariant(
                    "double element has no exact decimal spelling",
                ));
            }
            out.fmt(format_args!("{value:?}d"))?;
        }
    }
    Ok(())
}

/// One original `char` unit as a single-quoted literal with the project's
/// existing escapes; an isolated surrogate or control unit keeps `\uXXXX`.
fn char_literal(out: &mut Buffer, unit: u16) -> Result<(), RenderError> {
    out.put("'")?;
    if unit == u16::from(b'\'') {
        out.put("\\'")?;
    } else {
        out.units(std::slice::from_ref(&unit), false)?;
    }
    out.put("'")
}
pub(super) fn unary(op: crate::p2::UnaryOp) -> &'static str {
    use crate::p2::UnaryOp::*;
    match op {
        NegInt | NegLong | NegFloat | NegDouble => "-",
        NotInt | NotLong => "~",
        IntToLong | FloatToLong | DoubleToLong => "(long) ",
        IntToFloat | LongToFloat | DoubleToFloat => "(float) ",
        IntToDouble | LongToDouble | FloatToDouble => "(double) ",
        LongToInt | FloatToInt | DoubleToInt => "(int) ",
        IntToByte => "(byte) ",
        IntToChar => "(char) ",
        IntToShort => "(short) ",
    }
}
pub(super) fn binary(op: crate::p2::BinaryOp) -> &'static str {
    use crate::p2::BinaryOp::*;
    match op {
        AddInt | AddLong | AddFloat | AddDouble => " + ",
        SubInt | RsubInt | SubLong | SubFloat | SubDouble => " - ",
        MulInt | MulLong | MulFloat | MulDouble => " * ",
        DivInt | DivLong | DivFloat | DivDouble => " / ",
        RemInt | RemLong | RemFloat | RemDouble => " % ",
        AndInt | AndLong => " & ",
        OrInt | OrLong => " | ",
        XorInt | XorLong => " ^ ",
        ShlInt | ShlLong => " << ",
        ShrInt | ShrLong => " >> ",
        UshrInt | UshrLong => " >>> ",
    }
}
