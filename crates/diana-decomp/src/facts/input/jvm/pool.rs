use super::{error, ClassError, ClassProblem, JvmString, Reader};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Constant<'a> {
    Utf8(JvmString<'a>),
    Integer(i32),
    Float(u32),
    Long(i64),
    Double(u64),
    Class {
        name_index: u16,
    },
    String {
        string_index: u16,
    },
    FieldRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    MethodRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    InterfaceMethodRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    NameAndType {
        name_index: u16,
        descriptor_index: u16,
    },
    MethodHandle {
        kind: u8,
        reference_index: u16,
    },
    MethodType {
        descriptor_index: u16,
    },
    Dynamic {
        bootstrap_index: u16,
        name_and_type_index: u16,
    },
    InvokeDynamic {
        bootstrap_index: u16,
        name_and_type_index: u16,
    },
    Module {
        name_index: u16,
    },
    Package {
        name_index: u16,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NameAndType<'a> {
    pub name: JvmString<'a>,
    pub descriptor: JvmString<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceKind {
    Field,
    Method,
    InterfaceMethod,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemberReference<'a> {
    pub kind: ReferenceKind,
    pub owner: JvmString<'a>,
    pub name: JvmString<'a>,
    pub descriptor: JvmString<'a>,
}

#[derive(Debug)]
pub(super) enum PoolOffsets<'a> {
    Borrowed(&'a [u32]),
    Owned(Vec<u32>),
}

impl PoolOffsets<'_> {
    pub(super) fn as_slice(&self) -> &[u32] {
        match self {
            Self::Borrowed(offsets) => offsets,
            Self::Owned(offsets) => offsets,
        }
    }
}

#[derive(Debug)]
pub(super) struct Pool<'a> {
    pub bytes: &'a [u8],
    pub offsets: PoolOffsets<'a>,
}

const UTF8: u8 = 1;
const INTEGER: u8 = 3;
const FLOAT: u8 = 4;
const LONG: u8 = 5;
const DOUBLE: u8 = 6;
const CLASS: u8 = 7;
const STRING: u8 = 8;
const FIELD: u8 = 9;
const METHOD: u8 = 10;
const INTERFACE_METHOD: u8 = 11;
const NAME_AND_TYPE: u8 = 12;
const METHOD_HANDLE: u8 = 15;
const METHOD_TYPE: u8 = 16;
const DYNAMIC: u8 = 17;
const INVOKE_DYNAMIC: u8 = 18;
const MODULE: u8 = 19;
const PACKAGE: u8 = 20;

// A class starts with ten bytes before its first pool entry. There are
// at most 65,534 usable positions, each preceded by at most 65,533 entries
// of at most 65,538 bytes (the maximum length of one UTF-8 constant).
// Thus every original entry start fits in u32 even near the format limit.
const _: () = assert!((u16::MAX as u64 - 2) * (u16::MAX as u64 + 3) + 10 <= u32::MAX as u64);

/// Complete fixed-size slot widths, including their tags. UTF-8 entries have
/// a separate length field.
const SLOT_WIDTHS: [u8; PACKAGE as usize + 1] = {
    let mut widths = [0; PACKAGE as usize + 1];
    widths[INTEGER as usize] = 5;
    widths[FLOAT as usize] = 5;
    widths[LONG as usize] = 9;
    widths[DOUBLE as usize] = 9;
    widths[CLASS as usize] = 3;
    widths[STRING as usize] = 3;
    widths[FIELD as usize] = 5;
    widths[METHOD as usize] = 5;
    widths[INTERFACE_METHOD as usize] = 5;
    widths[NAME_AND_TYPE as usize] = 5;
    widths[METHOD_HANDLE as usize] = 4;
    widths[METHOD_TYPE as usize] = 3;
    widths[DYNAMIC as usize] = 5;
    widths[INVOKE_DYNAMIC as usize] = 5;
    widths[MODULE as usize] = 3;
    widths[PACKAGE as usize] = 3;
    widths
};

impl<'a> Pool<'a> {
    pub(super) fn scan(reader: &mut Reader<'a>, offsets: &mut Vec<u32>) -> Result<(), ClassError> {
        let count_offset = reader.at;
        let count = usize::from(reader.u16()?);
        if count == 0 {
            return Err(error(count_offset, ClassProblem::InvalidConstantCount));
        }
        // Even the smallest constant occupies three bytes. A two-slot constant
        // occupies nine. Check an inexpensive lower bound before reserving.
        if (count - 1) * 3 > reader.rest.len() {
            return Err(error(reader.at, ClassProblem::Truncated));
        }
        offsets.clear();
        offsets
            .try_reserve_exact(count)
            .map_err(|_| error(count_offset, ClassProblem::Allocation))?;
        offsets.push(0);
        while offsets.len() < count {
            let offset = reader.at;
            let Some(&tag) = reader.rest.first() else {
                return Err(error(offset, ClassProblem::Truncated));
            };
            let (span, header) = if tag == UTF8 {
                let [_, high, low] = reader
                    .rest
                    .first_chunk::<3>()
                    .ok_or_else(|| error(offset + 1, ClassProblem::Truncated))?;
                (3 + usize::from(u16::from_be_bytes([*high, *low])), 3)
            } else {
                let width = SLOT_WIDTHS
                    .get(usize::from(tag))
                    .copied()
                    .unwrap_or_default();
                if width == 0 {
                    return Err(error(offset, ClassProblem::UnknownConstantTag(tag)));
                }
                (usize::from(width), 1)
            };
            let Some(tail) = reader.rest.get(span..) else {
                return Err(error(offset + header, ClassProblem::Truncated));
            };
            reader.rest = tail;
            reader.at += span;
            offsets.push(offset as u32);
            if matches!(tag, LONG | DOUBLE) {
                if offsets.len() == count {
                    return Err(error(offset, ClassProblem::InvalidConstantCount));
                }
                offsets.push(0);
            }
        }
        Ok(())
    }

    /// Tests a UTF-8 payload by its checked original slot without decoding
    /// strings that cannot be one of the requested attribute names.
    pub(super) fn contains_utf8(&self, mut matches: impl FnMut(&[u8]) -> bool) -> bool {
        self.offsets
            .as_slice()
            .iter()
            .copied()
            .filter(|&offset| offset != 0)
            .any(|offset| {
                let Some(entry) = self.bytes.get(offset as usize..) else {
                    return false;
                };
                let [UTF8, high, low, rest @ ..] = entry else {
                    return false;
                };
                let size = usize::from(u16::from_be_bytes([*high, *low]));
                rest.get(..size).is_some_and(&mut matches)
            })
    }

    fn slot(&self, index: u16, used_at: usize) -> Result<(u8, Reader<'a>), ClassError> {
        let offset = self
            .offsets
            .as_slice()
            .get(usize::from(index))
            .copied()
            .filter(|&offset| offset != 0)
            .ok_or_else(|| error(used_at, ClassProblem::InvalidConstantIndex(index)))?;
        let offset = offset as usize;
        let mut reader = Reader::new(&self.bytes[offset..], offset);
        let tag = reader.u8()?;
        Ok((tag, reader))
    }

    pub(super) fn constant(&self, index: u16, used_at: usize) -> Result<Constant<'a>, ClassError> {
        let (tag, mut reader) = self.slot(index, used_at)?;
        Ok(match tag {
            UTF8 => {
                let size = usize::from(reader.u16()?);
                let offset = reader.at;
                Constant::Utf8(JvmString::new(reader.take(size)?, offset)?)
            }
            INTEGER => Constant::Integer(i32::from_be_bytes(reader.array()?)),
            FLOAT => Constant::Float(reader.u32()?),
            LONG => Constant::Long(i64::from_be_bytes(reader.array()?)),
            DOUBLE => Constant::Double(u64::from_be_bytes(reader.array()?)),
            CLASS => Constant::Class {
                name_index: reader.u16()?,
            },
            STRING => Constant::String {
                string_index: reader.u16()?,
            },
            FIELD => Constant::FieldRef {
                class_index: reader.u16()?,
                name_and_type_index: reader.u16()?,
            },
            METHOD => Constant::MethodRef {
                class_index: reader.u16()?,
                name_and_type_index: reader.u16()?,
            },
            INTERFACE_METHOD => Constant::InterfaceMethodRef {
                class_index: reader.u16()?,
                name_and_type_index: reader.u16()?,
            },
            NAME_AND_TYPE => Constant::NameAndType {
                name_index: reader.u16()?,
                descriptor_index: reader.u16()?,
            },
            METHOD_HANDLE => Constant::MethodHandle {
                kind: reader.u8()?,
                reference_index: reader.u16()?,
            },
            METHOD_TYPE => Constant::MethodType {
                descriptor_index: reader.u16()?,
            },
            DYNAMIC => Constant::Dynamic {
                bootstrap_index: reader.u16()?,
                name_and_type_index: reader.u16()?,
            },
            INVOKE_DYNAMIC => Constant::InvokeDynamic {
                bootstrap_index: reader.u16()?,
                name_and_type_index: reader.u16()?,
            },
            MODULE => Constant::Module {
                name_index: reader.u16()?,
            },
            PACKAGE => Constant::Package {
                name_index: reader.u16()?,
            },
            _ => return Err(error(reader.at - 1, ClassProblem::UnknownConstantTag(tag))),
        })
    }

    fn expect_slot(
        &self,
        index: u16,
        used_at: usize,
        expected_tag: u8,
    ) -> Result<Reader<'a>, ClassError> {
        let (tag, reader) = self.slot(index, used_at)?;
        if tag == expected_tag {
            Ok(reader)
        } else {
            Err(error(
                used_at,
                ClassProblem::UnexpectedConstantKind { index, tag },
            ))
        }
    }

    pub(super) fn utf8_bytes(
        &self,
        index: u16,
        used_at: usize,
    ) -> Result<(&'a [u8], usize), ClassError> {
        let mut reader = self.expect_slot(index, used_at, UTF8)?;
        let count = usize::from(reader.u16()?);
        let offset = reader.at;
        Ok((reader.take(count)?, offset))
    }

    pub(super) fn utf8(&self, index: u16, used_at: usize) -> Result<JvmString<'a>, ClassError> {
        let (bytes, offset) = self.utf8_bytes(index, used_at)?;
        JvmString::new(bytes, offset)
    }

    fn indirect_utf8(
        &self,
        index: u16,
        used_at: usize,
        tag: u8,
    ) -> Result<JvmString<'a>, ClassError> {
        let mut reader = self.expect_slot(index, used_at, tag)?;
        let utf8_at = reader.at;
        self.utf8(reader.u16()?, utf8_at)
    }

    pub(super) fn binary_name(
        &self,
        index: u16,
        used_at: usize,
    ) -> Result<JvmString<'a>, ClassError> {
        self.indirect_utf8(index, used_at, CLASS)?.binary_name()
    }

    pub(super) fn class_name(
        &self,
        index: u16,
        used_at: usize,
    ) -> Result<JvmString<'a>, ClassError> {
        self.indirect_utf8(index, used_at, CLASS)?
            .array_or_binary_name()
    }

    pub(super) fn string_constant(
        &self,
        index: u16,
        used_at: usize,
    ) -> Result<JvmString<'a>, ClassError> {
        self.indirect_utf8(index, used_at, STRING)
    }

    pub(super) fn tags(&self) -> impl Iterator<Item = (u16, u8)> + '_ {
        self.offsets
            .as_slice()
            .iter()
            .copied()
            .enumerate()
            .filter(|&(_, offset)| offset != 0)
            .map(|(index, offset)| (index as u16, self.bytes[offset as usize]))
    }

    pub(super) fn strings(
        &self,
    ) -> impl Iterator<Item = Result<(u16, JvmString<'a>), ClassError>> + '_ {
        self.tags()
            .filter(|&(_, tag)| tag == STRING)
            .map(|(index, _)| self.string_constant(index, 8).map(|value| (index, value)))
    }

    pub(super) fn name_and_type(
        &self,
        index: u16,
        used_at: usize,
    ) -> Result<NameAndType<'a>, ClassError> {
        let mut reader = self.expect_slot(index, used_at, NAME_AND_TYPE)?;
        let at = reader.at;
        let name = self.utf8(reader.u16()?, at)?;
        let descriptor = self.utf8(reader.u16()?, at + 2)?;
        let method = descriptor.bytes().starts_with(b"(");
        Ok(NameAndType {
            name: name.member_name(method)?,
            descriptor: descriptor.descriptor(method)?,
        })
    }

    pub(super) fn member_reference(&self, index: u16) -> Result<MemberReference<'a>, ClassError> {
        let (tag, mut reader) = self.slot(index, 8)?;
        let offset = reader.at;
        let kind = match tag {
            FIELD => ReferenceKind::Field,
            METHOD => ReferenceKind::Method,
            INTERFACE_METHOD => ReferenceKind::InterfaceMethod,
            _ => {
                return Err(error(
                    offset - 1,
                    ClassProblem::UnexpectedConstantKind { index, tag },
                ))
            }
        };
        let owner = self.class_name(reader.u16()?, offset)?;
        let NameAndType { name, descriptor } = self.name_and_type(reader.u16()?, offset + 2)?;
        let is_method = descriptor.bytes().starts_with(b"(");
        if is_method != (kind != ReferenceKind::Field) {
            return Err(error(descriptor.offset(), ClassProblem::InvalidDescriptor));
        }
        Ok(MemberReference {
            kind,
            owner,
            name,
            descriptor,
        })
    }
}
