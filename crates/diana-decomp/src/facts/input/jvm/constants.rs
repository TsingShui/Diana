//! Original field ConstantValue attributes.

use super::members::MemberKind;
use super::{error, ClassError, ClassFile, ClassProblem, Constant, JvmString, Member};

#[derive(Clone, Copy, Debug)]
pub struct FieldConstant<'a> {
    pub offset: usize,
    pub value: FieldValue<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldValue<'a> {
    Integer(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    String(JvmString<'a>),
}

impl<'a> ClassFile<'a> {
    /// A validated value at its original class-file byte position. Whether the
    /// attribute initializes the field depends on the original static flag.
    pub fn field_constant(
        &self,
        member: Member<'a>,
    ) -> Result<Option<FieldConstant<'a>>, ClassError> {
        member.verify(self)?;
        if member.kind() != MemberKind::Field {
            return Ok(None);
        }
        let Some(attribute) = self.find_attribute(member.attributes(), b"ConstantValue")? else {
            return Ok(None);
        };
        let at = attribute.offset;
        let bytes = <[u8; 2]>::try_from(attribute.payload)
            .map_err(|_| error(at, ClassProblem::InvalidFieldConstant))?;
        let index = u16::from_be_bytes(bytes);
        let value = match (
            self.pool.constant(index, at)?,
            member.descriptor(self)?.bytes(),
        ) {
            (Constant::Integer(value), b"B" | b"C" | b"I" | b"S" | b"Z") => {
                FieldValue::Integer(value)
            }
            (Constant::Long(value), b"J") => FieldValue::Long(value),
            (Constant::Float(value), b"F") => FieldValue::Float(value),
            (Constant::Double(value), b"D") => FieldValue::Double(value),
            (Constant::String { string_index }, b"Ljava/lang/String;") => {
                FieldValue::String(self.pool.utf8(string_index, at)?)
            }
            _ => return Err(error(at, ClassProblem::InvalidFieldConstant)),
        };
        Ok(Some(FieldConstant { offset: at, value }))
    }
}
