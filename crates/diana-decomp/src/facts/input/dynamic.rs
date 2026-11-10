use super::values::{EncodedKind, EncodedValues, ValueStep};
use super::{malformed, InputError, InputProblem, MethodError, ProgramIndex, Prototype};

use crate::raw::OperandReference;
use crate::{ByteLength, ByteOffset, CallSiteId, FieldId, MethodHandleId, MethodId, ProtoId};

pub struct CheckedMethodHandle {
    pub kind: u16,
    pub member: OperandReference,
}

pub struct CheckedCallSite<'i, 'a> {
    pub handle_id: MethodHandleId,
    pub name_id: crate::StringId,
    pub proto_id: ProtoId,
    pub bootstrap: Vec<ValueStep<'a>>,
    pub range: crate::ByteRange,
    pub encoded: &'a [u8],
    pub count: u32,
    pub name: super::DexString<'a>,
    pub handle: CheckedMethodHandle,
    pub prototype: Prototype<'i, 'a>,
}

/// Established mandatory call-site header and an explicitly unfinished value walk.
pub(super) struct CallSitePrefix<'i, 'a> {
    pub offset: ByteOffset,
    pub values: EncodedValues<'a>,
    pub handle_id: MethodHandleId,
    pub name_id: crate::StringId,
    pub proto_id: ProtoId,
    pub name: super::DexString<'a>,
    pub handle: CheckedMethodHandle,
    pub prototype: Prototype<'i, 'a>,
}

fn invalid(at: ByteOffset) -> InputError {
    malformed(InputProblem::InvalidReference, at)
}

impl<'a> ProgramIndex<'a> {
    pub fn method_handle(&self, id: MethodHandleId) -> Result<CheckedMethodHandle, InputError> {
        let mut record = self.mapped_entry(8, id.0, ByteLength(8))?;
        let at = record.position_byte();
        let kind = record.read_u16()?;
        let _first_unused = record.read_u16()?;
        let target = u32::from(record.read_u16()?);
        let _second_unused = record.read_u16()?;
        // DEX calls these words unused; it does not require their contents to be zero.
        if kind > 8 {
            return Err(invalid(at));
        }
        let member = if kind <= 3 {
            self.field_reference(FieldId(target))?;
            OperandReference::Field(FieldId(target))
        } else {
            let method = self.method_reference(MethodId(target))?;
            let constructor = method
                .declaration
                .name
                .matches(&const { crate::substrate::owned::ascii_units(*b"<init>") })?;
            if constructor != (kind == 6)
                || method
                    .declaration
                    .name
                    .matches(&const { crate::substrate::owned::ascii_units(*b"<clinit>") })?
                || constructor && !method.declaration.prototype.return_type.matches(&[86])?
            {
                return Err(invalid(at));
            }
            OperandReference::Method(MethodId(target))
        };
        Ok(CheckedMethodHandle { kind, member })
    }

    pub(super) fn call_site_prefix(
        &self,
        id: CallSiteId,
    ) -> Result<CallSitePrefix<'_, 'a>, MethodError> {
        self.call_site_prefix_observed(id, &mut |_, _| Ok(()))
    }

    pub(super) fn call_site_prefix_observed(
        &self,
        id: CallSiteId,
        established: &mut impl FnMut(OperandReference, &'static str) -> Result<(), MethodError>,
    ) -> Result<CallSitePrefix<'_, 'a>, MethodError> {
        let mut record = self.mapped_entry(7, id.0, ByteLength(4))?;
        let offset = ByteOffset(
            usize::try_from(record.read_u32()?).map_err(|_| MethodError::PlatformCapacity)?,
        );
        let mut values = EncodedValues::open(self, offset, None)?;
        if values.count < 3 {
            return Err(invalid(offset).into());
        }
        let Some(ValueStep::Value(handle)) = values.next(self)? else {
            return Err(invalid(offset).into());
        };
        let EncodedKind::Reference(OperandReference::MethodHandle(handle_id)) = handle.kind else {
            return Err(invalid(handle.range.start).into());
        };
        let handle = self.method_handle(handle_id)?;
        established(OperandReference::MethodHandle(handle_id), "bootstrap")?;
        let Some(ValueStep::Value(name)) = values.next(self)? else {
            return Err(invalid(offset).into());
        };
        let EncodedKind::Reference(OperandReference::String(name_id)) = name.kind else {
            return Err(invalid(name.range.start).into());
        };
        let name = self.string(name_id)?;
        name.validate_name()?;
        if name.matches(&const { crate::substrate::owned::ascii_units(*b"<init>") })?
            || name.matches(&const { crate::substrate::owned::ascii_units(*b"<clinit>") })?
        {
            return Err(invalid(offset).into());
        }
        established(OperandReference::String(name_id), "name")?;
        let Some(ValueStep::Value(proto)) = values.next(self)? else {
            return Err(invalid(offset).into());
        };
        let EncodedKind::Reference(OperandReference::Proto(proto_id)) = proto.kind else {
            return Err(invalid(proto.range.start).into());
        };
        let prototype = self.prototype(proto_id)?;
        established(OperandReference::Proto(proto_id), "signature")?;
        Ok(CallSitePrefix {
            offset,
            values,
            handle_id,
            name_id,
            proto_id,
            name,
            handle,
            prototype,
        })
    }

    pub fn call_site(&self, id: CallSiteId) -> Result<CheckedCallSite<'_, 'a>, MethodError> {
        let CallSitePrefix {
            offset,
            mut values,
            handle_id,
            name_id,
            proto_id,
            name,
            handle,
            prototype,
        } = self.call_site_prefix(id)?;
        let mut bootstrap = Vec::new();
        while let Some(value) = values.next(self)? {
            bootstrap.try_reserve(1).map_err(MethodError::Allocation)?;
            bootstrap.push(value);
        }
        let range = crate::ByteRange {
            start: offset,
            end: values.position(),
        };
        let encoded =
            self.bytes
                .get(range.start.0..range.end.0)
                .ok_or(MethodError::InternalInvariant(
                    "checked call site lost its encoded range",
                ))?;
        Ok(CheckedCallSite {
            handle_id,
            name_id,
            proto_id,
            bootstrap,
            range,
            encoded,
            count: values.count - 3,
            name,
            handle,
            prototype,
        })
    }

    pub(super) fn encoded_dynamic_reference(
        &self,
        tag: u8,
        id: u32,
        at: ByteOffset,
    ) -> Result<OperandReference, InputError> {
        if self.header.version < super::formats::DexVersion::V038 {
            return Err(InputError::Unsupported {
                reason: super::UnsupportedInput::EncodedValueTag(tag),
                offset_byte: at,
            });
        }
        match tag {
            0x15 => {
                self.prototype(ProtoId(id))?;
                Ok(OperandReference::Proto(ProtoId(id)))
            }
            0x16 => {
                self.method_handle(MethodHandleId(id))?;
                Ok(OperandReference::MethodHandle(MethodHandleId(id)))
            }
            _ => Err(InputError::InternalInvariant(
                "non-dynamic encoded tag in dynamic reference query",
            )),
        }
    }
}
