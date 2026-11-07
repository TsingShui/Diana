//! Declaration-only reads of a logical DEX prefix.
//!
//! Class and string-pool listings can use declaration prefixes. A class index needs
//! the header, three id tables and the descriptor string items
//! of those classes — on a 343 MiB corpus-based APK that is 31.7% of the bytes. The
//! full reader bounds every string item by its `map_list` section, and `map_list`
//! sits at the end of the file in every entry of that corpus, so the full read has
//! to inflate the whole member. This module answers the same question from a
//! prefix, with bounds taken from the header's declared sizes and the accepted byte
//! length instead of a map section, and reports [`PrefixClasses::Needs`] rather than
//! guessing when it cannot answer. It never returns a partial class list: the caller
//! either gets every definition or is told to read the whole member.
//!
//! Every value here still comes from the shared checked reader: `Cursor` for range
//! arithmetic, `DexString::read` for bounded MUTF-8 payloads, the same table
//! strides and the same descriptor shape check as `LocalDeclarations::checked_class`.
//! What differs is only the *bound* of a string item, which the specification states
//! explicitly. DEX 041 chains are not served from a prefix because the chain lives
//! after the first header.

use super::strings::TypeShape;
use super::{
    Cursor, DexString, InputError, InputProblem, UnsupportedInput, count_usize, file_offset,
    malformed,
};
use crate::{ByteLength, ByteOffset, ClassDefId, StringId, TypeId};

const HEADER_SIZE: usize = 0x70;
/// Same strides as `LocalDeclarations` tables: string, type, proto, field, method,
/// class_def.
const TABLE_STRIDES: [usize; 6] = [4, 4, 12, 8, 8, 32];
/// How far past an unreachable string item a prefix round asks for.
const STRING_SLACK: usize = 1 << 8;

/// One `class_defs` definition reachable from a prefix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeclaredClass<'a> {
    pub class_def: ClassDefId,
    pub descriptor: DexString<'a>,
    pub access_flags: u32,
}

/// What a prefix could answer.
pub enum PrefixClasses<'a> {
    /// The accepted bytes do not reach this many bytes of the member yet.
    Needs(usize),
    /// A prefix cannot answer for this member; the whole member must be read.
    Whole,
    /// Every definition of the logical member, in original `class_defs` order.
    Ready(Vec<DeclaredClass<'a>>),
}

/// Availability of the complete original string pool under prefix bounds.
pub enum PrefixStrings<'a> {
    /// More accepted input is needed before the complete pool is available.
    Needs(usize),
    /// This container requires the whole-member reader.
    Whole,
    /// Every original pool entry, in string-ID order.
    Ready(StringPool<'a>),
}

/// Complete original pool facts. Input is borrowed once rather than once per item;
/// the private records contain only established payload offsets and unit counts.
/// DEX 035–040 declare a u32 physical extent, so all stored offsets fit that width.
pub struct StringPool<'a> {
    bytes: &'a [u8],
    records: Vec<StringRecord>,
}

struct StringRecord {
    start: u32,
    end: u32,
    utf16_len: u32,
}

impl<'a> StringPool<'a> {
    /// Delivers existing item facts in original string-ID order. Rebinding a span
    /// does not read a pool record, length prefix, terminator or encoded unit again.
    pub fn visit<E: From<InputError>>(
        self,
        mut visit: impl FnMut(StringId, DexString<'a>) -> Result<(), E>,
    ) -> Result<(), E> {
        for (index, record) in self.records.into_iter().enumerate() {
            let value =
                DexString::from_parts(self.bytes, record.start, record.end, record.utf16_len)
                    .ok_or(InputError::InternalInvariant(
                        "prefix pool span lost its input range",
                    ))?;
            let id = StringId(
                u32::try_from(index).map_err(|_| InputError::PlatformCapacity {
                    offset_byte: ByteOffset(0),
                })?,
            );
            visit(id, value)?;
        }
        Ok(())
    }
}

/// Reads every original string through the shared item reader and prefix bounds.
/// No values are published until the entire pool is present.
pub fn strings(bytes: &[u8]) -> Result<PrefixStrings<'_>, InputError> {
    if bytes.len() < HEADER_SIZE {
        return Ok(PrefixStrings::Needs(HEADER_SIZE));
    }
    let Some(header) = read_header(bytes)? else {
        return Ok(PrefixStrings::Whole);
    };
    let (count, offset) = header.tables[0];
    let count_usize = count_usize(count, offset)?;
    // Header parsing already checked this complete table extent.
    let table_end = offset.0 + count_usize * 4;
    if table_end > bytes.len() {
        return Ok(PrefixStrings::Needs(table_end));
    }
    let mut tail = 0;
    for index in 0..count {
        tail = tail.max(string_item(bytes, &header, StringId(index))?.0);
    }
    let wanted = tail.saturating_add(STRING_SLACK).min(header.declared);
    if wanted > bytes.len() {
        return Ok(PrefixStrings::Needs(wanted));
    }
    let mut values = Vec::new();
    values
        .try_reserve_exact(count_usize)
        .map_err(|_| InputError::Allocation {
            offset_byte: offset,
        })?;
    for index in 0..count {
        let data_at = string_item(bytes, &header, StringId(index))?;
        let cursor = Cursor::bounded(bytes, data_at, string_end(bytes, &header, data_at))?;
        match DexString::read(bytes, cursor) {
            Ok(value) => {
                let (start, end, utf16_len) = value.parts().ok_or(
                    InputError::InternalInvariant("prefix pool span exceeds its u32 header extent"),
                )?;
                values.push(StringRecord {
                    start,
                    end,
                    utf16_len,
                });
            }
            Err(_) if bytes.len() < header.declared => {
                return Ok(PrefixStrings::Needs(next_request(
                    bytes.len(),
                    bytes.len(),
                    header.declared,
                )));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(PrefixStrings::Ready(StringPool {
        bytes,
        records: values,
    }))
}

/// Resolves a pool entry under declaration-prefix bounds, shared by class and
/// string listings. Header parsing established the complete table/data extents.
fn string_item(bytes: &[u8], header: &Header, id: StringId) -> Result<ByteOffset, InputError> {
    let (count, offset) = header.tables[0];
    if id.0 >= count {
        return Err(malformed(InputProblem::InvalidReference, offset));
    }
    let record_at = ByteOffset(offset.0 + count_usize(id.0, offset)? * 4);
    let mut record = Cursor::at(bytes, record_at)?;
    let data_at = file_offset(record.read_u32()?, record_at)?;
    let data_end = header.data.0.0 + header.data.1;
    if data_at < header.data.0 || data_at.0 >= data_end {
        return Err(malformed(InputProblem::InvalidReference, data_at));
    }
    if header.link.1 != 0 && data_at >= header.link.0 && data_at.0 < header.link.0.0 + header.link.1
    {
        return Err(malformed(InputProblem::OverlappingSections, data_at));
    }
    Ok(data_at)
}

fn string_end(bytes: &[u8], header: &Header, data_at: ByteOffset) -> ByteOffset {
    let mut end = bytes.len().min(header.data.0.0 + header.data.1);
    if data_at < header.link.0 {
        end = end.min(header.link.0.0);
    }
    ByteOffset(end)
}

/// Lists the class definitions of a logical DEX prefix.
///
/// Errors are the same classified input errors the full reader reports; only
/// truncation inside a still-growing prefix is reported as [`PrefixClasses::Needs`].
pub fn classes(bytes: &[u8]) -> Result<PrefixClasses<'_>, InputError> {
    if bytes.len() < HEADER_SIZE {
        return Ok(PrefixClasses::Needs(HEADER_SIZE));
    }
    match read_header(bytes)? {
        None => Ok(PrefixClasses::Whole),
        Some(header) => classes_of(bytes, &header),
    }
}

/// The bytes a prefix needs before a class list is attemptable at all.
fn tables_end(header: &Header) -> Result<usize, InputError> {
    let mut need = HEADER_SIZE;
    for (index, (count, offset)) in header.tables.iter().enumerate() {
        let stride = TABLE_STRIDES[index];
        let end = usize::try_from(*count)
            .ok()
            .and_then(|count| count.checked_mul(stride))
            .and_then(|size| offset.0.checked_add(size))
            .ok_or_else(|| malformed(InputProblem::RangeOverflow, *offset))?;
        need = need.max(end);
    }
    Ok(need)
}

struct Header {
    declared: usize,
    tables: [(u32, ByteOffset); 6],
    data: (ByteOffset, usize),
    link: (ByteOffset, usize),
}

fn read_header(bytes: &[u8]) -> Result<Option<Header>, InputError> {
    match &bytes[..8] {
        b"dex\n035\0" | b"dex\n037\0" | b"dex\n038\0" | b"dex\n039\0" | b"dex\n040\0" => {}
        b"dex\n041\0" => return Ok(None),
        _ => {
            return Err(InputError::Unsupported {
                reason: UnsupportedInput::Container,
                offset_byte: ByteOffset(0),
            });
        }
    }
    let mut cursor = Cursor::bounded(bytes, ByteOffset(0), ByteOffset(HEADER_SIZE))?;
    cursor.skip(1, ByteLength(0x20))?;
    let declared = count_usize(cursor.read_u32()?, ByteOffset(0x20))?;
    let header_size = count_usize(cursor.read_u32()?, ByteOffset(0x24))?;
    if header_size != HEADER_SIZE {
        return Err(malformed(InputProblem::InvalidHeaderSize, ByteOffset(0x24)));
    }
    if declared < HEADER_SIZE || declared < bytes.len() {
        return Err(malformed(InputProblem::FileSizeMismatch, ByteOffset(0x20)));
    }
    match cursor.read_u32()? {
        0x1234_5678 => {}
        0x7856_3412 => {
            return Err(InputError::Unsupported {
                reason: UnsupportedInput::ReverseEndian,
                offset_byte: ByteOffset(0x28),
            });
        }
        _ => return Err(malformed(InputProblem::InvalidEndianTag, ByteOffset(0x28))),
    }
    let link_size = count_usize(cursor.read_u32()?, ByteOffset(0x2c))?;
    let link_offset = file_offset(cursor.read_u32()?, ByteOffset(0x30))?;
    let link_end = link_offset
        .0
        .checked_add(link_size)
        .ok_or_else(|| malformed(InputProblem::RangeOverflow, link_offset))?;
    if link_end > declared {
        return Err(malformed(InputProblem::Truncated, link_offset));
    }
    cursor.skip(1, ByteLength(4))?;
    let mut tables = [(0u32, ByteOffset(0)); 6];
    for (index, table) in tables.iter_mut().enumerate() {
        let field = ByteOffset(0x38 + index * 8);
        let count = cursor.read_u32()?;
        let offset = file_offset(cursor.read_u32()?, ByteOffset(field.0 + 4))?;
        let end = usize::try_from(count)
            .ok()
            .and_then(|count| count.checked_mul(TABLE_STRIDES[index]))
            .and_then(|size| offset.0.checked_add(size))
            .ok_or_else(|| malformed(InputProblem::RangeOverflow, field))?;
        if end > declared {
            return Err(malformed(InputProblem::Truncated, field));
        }
        if matches!(index, 1 | 2) && count > 65535 {
            return Err(malformed(InputProblem::InvalidTableCount, field));
        }
        *table = (count, offset);
    }
    let data_size = count_usize(cursor.read_u32()?, ByteOffset(0x68))?;
    let data_offset = file_offset(cursor.read_u32()?, ByteOffset(0x6c))?;
    if !data_offset.0.is_multiple_of(4) {
        return Err(malformed(InputProblem::InvalidAlignment, data_offset));
    }
    let data_end = data_offset
        .0
        .checked_add(data_size)
        .ok_or_else(|| malformed(InputProblem::RangeOverflow, data_offset))?;
    if data_end > declared {
        return Err(malformed(InputProblem::Truncated, data_offset));
    }
    Ok(Some(Header {
        declared,
        tables,
        data: (data_offset, data_size),
        link: (link_offset, link_size),
    }))
}

fn classes_of<'a>(bytes: &'a [u8], header: &Header) -> Result<PrefixClasses<'a>, InputError> {
    let need = tables_end(header)?;
    if need > bytes.len() {
        return Ok(PrefixClasses::Needs(need.min(header.declared)));
    }
    let (class_count, class_offset) = header.tables[5];

    // Resolve every descriptor item first. One round then reaches the last item,
    // instead of growing the prefix by a few hundred bytes per class; the table
    // arithmetic is the same in both passes.
    let mut tail = 0usize;
    for index in 0..class_count {
        let (data_at, _) = class_item(bytes, header, index)?;
        tail = tail.max(data_at.0);
    }
    if tail.saturating_add(STRING_SLACK) > bytes.len() {
        let wanted = tail.saturating_add(STRING_SLACK).min(header.declared);
        if wanted > bytes.len() {
            return Ok(PrefixClasses::Needs(wanted));
        }
    }

    let mut classes = Vec::new();
    classes
        .try_reserve(usize::try_from(class_count).unwrap_or(0))
        .map_err(|_| InputError::Allocation {
            offset_byte: class_offset,
        })?;
    for index in 0..class_count {
        let (data_at, access_flags) = class_item(bytes, header, index)?;
        let end = string_end(bytes, header, data_at);
        let cursor = Cursor::bounded(bytes, data_at, end)?;
        let descriptor = match DexString::read(bytes, cursor) {
            Ok(value) => value,
            // Any decode failure inside a still-growing prefix may simply be a
            // prefix that ends inside a unit, so ask for the rest of the member and
            // let the whole-member read classify it. At the declared size the same
            // failure is a real one and is reported.
            Err(_) if bytes.len() < header.declared => {
                return Ok(PrefixClasses::Needs(next_request(
                    bytes.len(),
                    bytes.len(),
                    header.declared,
                )));
            }
            Err(error) => return Err(error),
        };
        // Same rule as the decode: while the member is still a prefix, a
        // descriptor that does not look like a class may just be cut short.
        match descriptor.shape() {
            Ok(TypeShape::Class) => {}
            produced => {
                if bytes.len() < header.declared {
                    return Ok(PrefixClasses::Needs(next_request(
                        bytes.len(),
                        bytes.len(),
                        header.declared,
                    )));
                }
                return match produced {
                    Err(error) => Err(error),
                    Ok(_) => Err(malformed(InputProblem::InvalidDescriptor, data_at)),
                };
            }
        }
        classes.push(DeclaredClass {
            class_def: ClassDefId(index),
            descriptor,
            access_flags,
        });
    }
    Ok(PrefixClasses::Ready(classes))
}

/// Resolves one class definition's descriptor string item and access flags.
fn class_item(bytes: &[u8], header: &Header, index: u32) -> Result<(ByteOffset, u32), InputError> {
    let (type_count, type_offset) = header.tables[1];
    let (_, class_offset) = header.tables[5];
    let record_at = ByteOffset(
        class_offset
            .0
            .checked_add(usize::try_from(index).unwrap_or(0) * 32)
            .ok_or_else(|| malformed(InputProblem::RangeOverflow, class_offset))?,
    );
    let mut record = Cursor::at(bytes, record_at)?;
    let type_idx = TypeId(record.read_u32()?);
    let access_flags = record.read_u32()?;
    if type_idx.0 >= type_count {
        return Err(malformed(InputProblem::InvalidReference, record_at));
    }
    let type_at = ByteOffset(type_offset.0 + usize::try_from(type_idx.0).unwrap_or(0) * 4);
    let mut type_record = Cursor::at(bytes, type_at)?;
    let string_idx = StringId(type_record.read_u32()?);
    if string_idx.0 >= header.tables[0].0 {
        return Err(malformed(InputProblem::InvalidReference, type_at));
    }
    let data_at = string_item(bytes, header, string_idx)?;
    Ok((data_at, access_flags))
}

/// The next prefix length to ask for: past the item, never past the member, and
/// always larger than what was accepted so a prefix round always makes progress.
fn next_request(want: usize, accepted: usize, declared: usize) -> usize {
    let target = want
        .saturating_add(STRING_SLACK)
        .max(accepted.saturating_mul(2))
        .min(declared);
    if target > accepted { target } else { declared }
}
