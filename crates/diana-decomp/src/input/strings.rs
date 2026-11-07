use std::fmt;

use super::{Cursor, InputError, InputProblem, malformed};

use crate::{ByteOffset, Utf16Count, Utf16Offset};

/// A bounded, borrowed MUTF-8 payload. Literal surrogate units remain intact;
/// declaration names and descriptors undergo additional syntax checks.
#[derive(Clone, Copy)]
pub struct DexString<'a> {
    bytes: &'a [u8],
    start_byte: ByteOffset,
    end_byte: ByteOffset,
    utf16_len: Utf16Count,
}

impl PartialEq for DexString<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes() == other.bytes()
    }
}

impl Eq for DexString<'_> {}

impl fmt::Debug for DexString<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DexString")
            .field("start_byte", &self.start_byte)
            .field("end_byte", &self.end_byte)
            .field("utf16_len", &self.utf16_len)
            .finish()
    }
}

impl<'a> DexString<'a> {
    pub fn utf16_len(&self) -> Utf16Count {
        self.utf16_len
    }

    /// The original MUTF-8 payload, without the length prefix and terminator.
    ///
    /// `read` established the payload range and its terminator, so this is a
    /// bounded view of the input, not a claim that every encoded unit is valid. A consumer that only has
    /// to display or compare the value can use it directly and skip materializing
    /// UTF-16 units; a scalar that needs decoding is [`Self::utf8`] or
    /// [`Self::units`].
    pub fn bytes(&self) -> &'a [u8] {
        &self.bytes[self.start_byte.0..self.end_byte.0]
    }

    /// True when the payload is pure ASCII, so its bytes are also its UTF-8 form.
    pub fn is_ascii(&self) -> bool {
        self.bytes().is_ascii()
    }

    /// The payload as UTF-8, when it already is UTF-8.
    ///
    /// MUTF-8 encodes surrogate pairs as two three-byte sequences (CESU-8), so a
    /// value outside the BMP is metavalid UTF-8 and needs [`Self::units`]. Every
    /// ASCII and BMP value, including all of a normal string pool, answers here
    /// without allocating.
    pub fn utf8(&self) -> Option<&'a str> {
        std::str::from_utf8(self.bytes()).ok()
    }

    /// Reads one `string_data_item`: its declared unit count and the byte payload
    /// up to the terminator.
    ///
    /// The payload is not decoded or validated here. A string pool is millions of
    /// items and most requests only display or match them, so the cost of proving
    /// every unit legal was paid on every access; what remains is the one bound a
    /// reader cannot do without — the terminator. A consumer that needs units
    /// decodes them with [`Units`], which replaces an illegal sequence with U+FFFD
    /// instead of failing the item that names it.
    pub(super) fn read(bytes: &'a [u8], mut cursor: Cursor<'a>) -> Result<Self, InputError> {
        let utf16_len = cursor.read_uleb32()?;
        let start_byte = cursor.position_byte();
        let limit = ByteOffset(start_byte.0 + cursor.remaining_byte().0);
        let available = bytes
            .get(start_byte.0..limit.0)
            .ok_or_else(|| malformed(InputProblem::Truncated, start_byte))?;
        let length = memchr::memchr(0, available)
            .ok_or_else(|| malformed(InputProblem::Truncated, start_byte))?;
        Ok(Self {
            bytes,
            start_byte,
            end_byte: ByteOffset(start_byte.0 + length),
            utf16_len: Utf16Count(utf16_len),
        })
    }

    /// Copies the original UTF-16 value of the payload.
    ///
    /// One unit is at least one byte, so the payload length bounds the reservation:
    /// a declared count that disagrees with the payload cannot drive the
    /// allocation. Units are decoded with replacement, so this never fails for a
    /// malformed payload.
    pub fn units(&self) -> Result<Vec<u16>, InputError> {
        let capacity = usize::try_from(self.utf16_len.0)
            .unwrap_or(usize::MAX)
            .min(self.end_byte.0.saturating_sub(self.start_byte.0));
        let mut units = Vec::new();
        units
            .try_reserve_exact(capacity)
            .map_err(|_| InputError::Allocation {
                offset_byte: self.start_byte,
            })?;
        let mut reader = self.reader()?;
        while let Some(unit) = reader.next()? {
            units.push(unit);
        }
        Ok(units)
    }

    pub fn matches(&self, units: &[u16]) -> Result<bool, InputError> {
        Ok(self.matches_prefix(units, Utf16Offset(0))? == Some(Utf16Offset(units.len())))
    }

    /// Literal UTF-16 substring matching with per-unit lowercase equality.
    /// Surrogates compare as units; no normalization or multi-unit case folding
    /// changes the match. The original declared count does not bound the search.
    /// ASCII payloads use byte candidates; other payloads use the shared decoder,
    /// including its replacement behavior, without materializing a unit vector.
    pub fn contains_ignore_case(&self, needle: &[u16]) -> Result<bool, InputError> {
        let Some(&first) = needle.first() else {
            return Ok(true);
        };
        let bytes = self.bytes();
        if needle.iter().all(|&unit| unit < 0x80) && bytes.is_ascii() {
            if needle.len() > bytes.len() {
                return Ok(false);
            }
            // first fits u8 by the all-ASCII predicate above.
            let first = first as u8;
            for at in memchr::memchr2_iter(
                first.to_ascii_lowercase(),
                first.to_ascii_uppercase(),
                bytes,
            ) {
                let Some(candidate) = bytes.get(at..).and_then(|tail| tail.get(..needle.len()))
                else {
                    continue;
                };
                if candidate
                    .iter()
                    .zip(needle)
                    .all(|(&byte, &unit)| byte.eq_ignore_ascii_case(&(unit as u8)))
                {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        let mut positions = self.reader()?;
        while let Some(unit) = positions.next()? {
            if !unit_eq_ignore_case(unit, first) {
                continue;
            }
            let mut candidate = positions;
            let mut matched = true;
            for &expected in &needle[1..] {
                match candidate.next()? {
                    Some(actual) if unit_eq_ignore_case(actual, expected) => {}
                    _ => {
                        matched = false;
                        break;
                    }
                }
            }
            if matched {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(super) fn matches_prefix(
        &self,
        units: &[u16],
        mut position: Utf16Offset,
    ) -> Result<Option<Utf16Offset>, InputError> {
        let mut reader = self.reader()?;
        while let Some(unit) = reader.next()? {
            if units.get(position.0) != Some(&unit) {
                return Ok(None);
            }
            position.0 += 1;
        }
        Ok(Some(position))
    }

    /// The parts of a resolved item, for a member-level cache that re-creates it
    /// without reading the pool record or the length prefix again. `None` when an
    /// offset does not fit the recorded width, in which case nothing is cached and
    /// the item keeps being read from the pool.
    pub(super) fn parts(&self) -> Option<(u32, u32, u32)> {
        Some((
            u32::try_from(self.start_byte.0).ok()?,
            u32::try_from(self.end_byte.0).ok()?,
            self.utf16_len.0,
        ))
    }

    pub(super) fn from_parts(
        bytes: &'a [u8],
        start: u32,
        end: u32,
        utf16_len: u32,
    ) -> Option<Self> {
        let start_byte = ByteOffset(usize::try_from(start).ok()?);
        let end_byte = ByteOffset(usize::try_from(end).ok()?);
        bytes.get(start_byte.0..end_byte.0)?;
        Some(Self {
            bytes,
            start_byte,
            end_byte,
            utf16_len: Utf16Count(utf16_len),
        })
    }

    pub(crate) fn reader(&self) -> Result<Units<'a>, InputError> {
        if self.start_byte > self.end_byte || self.end_byte.0 > self.bytes.len() {
            return Err(InputError::InternalInvariant(
                "string payload is not inside its container",
            ));
        }
        Ok(Units {
            bytes: self.bytes,
            position: self.start_byte.0,
            end: self.end_byte.0,
        })
    }

    pub(super) fn validate_name(&self) -> Result<(), InputError> {
        let payload = self.bytes();
        let (start, end) = if payload.first() == Some(&b'<') {
            if payload.len() < 3 || payload.last() != Some(&b'>') {
                return Err(malformed(InputProblem::InvalidName, self.start_byte));
            }
            (self.start_byte.0 + 1, self.end_byte.0 - 1)
        } else {
            (self.start_byte.0, self.end_byte.0)
        };
        validate_name_units(
            Units {
                bytes: self.bytes,
                position: start,
                end,
            },
            NameSyntax::Simple,
        )
    }

    pub(super) fn shape(&self) -> Result<TypeShape, InputError> {
        let payload = self.bytes();
        let mut index = 0usize;
        let mut dimensions = 0usize;
        while payload.get(index) == Some(&b'[') {
            dimensions += 1;
            if dimensions > 255 {
                return Err(malformed(InputProblem::InvalidDescriptor, self.start_byte));
            }
            index += 1;
        }
        let Some(&tag) = payload.get(index) else {
            return Err(malformed(InputProblem::InvalidDescriptor, self.start_byte));
        };
        if tag == b'L' {
            if payload.len() - index < 3 || payload.last() != Some(&b';') {
                return Err(malformed(InputProblem::InvalidDescriptor, self.start_byte));
            }
            validate_name_units(
                Units {
                    bytes: self.bytes,
                    position: self.start_byte.0 + index + 1,
                    end: self.end_byte.0 - 1,
                },
                NameSyntax::ClassPath,
            )?;
            return Ok(if dimensions == 0 {
                TypeShape::Class
            } else {
                TypeShape::Array
            });
        }
        if index + 1 != payload.len() {
            return Err(malformed(InputProblem::InvalidDescriptor, self.start_byte));
        }
        if tag == b'V' && dimensions == 0 {
            return Ok(TypeShape::Void);
        }
        let primitive = PrimitiveType::from_tag(u16::from(tag))
            .ok_or_else(|| malformed(InputProblem::InvalidDescriptor, self.start_byte))?;
        Ok(if dimensions == 0 {
            TypeShape::Primitive(primitive)
        } else {
            TypeShape::Array
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) enum TypeShape {
    Void,
    Primitive(PrimitiveType),
    Class,
    Array,
}

impl TypeShape {
    pub(super) fn shorty(self) -> u8 {
        match self {
            Self::Void => b'V',
            Self::Class | Self::Array => b'L',
            Self::Primitive(primitive) => primitive as u8,
        }
    }
    pub(crate) fn words(self) -> u8 {
        match self {
            Self::Void => 0,
            Self::Primitive(PrimitiveType::Long | PrimitiveType::Double) => 2,
            _ => 1,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub(crate) enum PrimitiveType {
    Boolean = b'Z',
    Byte = b'B',
    Short = b'S',
    Char = b'C',
    Int = b'I',
    Long = b'J',
    Float = b'F',
    Double = b'D',
}

impl PrimitiveType {
    pub(crate) fn from_tag(tag: u16) -> Option<Self> {
        Some(match tag {
            0x5a => Self::Boolean,
            0x42 => Self::Byte,
            0x53 => Self::Short,
            0x43 => Self::Char,
            0x49 => Self::Int,
            0x4a => Self::Long,
            0x46 => Self::Float,
            0x44 => Self::Double,
            _ => return None,
        })
    }
}

/// Original UTF-16 units of one payload, decoded on demand.
///
/// An illegal sequence becomes U+FFFD and consumes one byte, so every payload
/// decodes to a finite unit stream: a malformed item stays readable instead of
/// failing the request that mentions it. Isolated surrogates are kept as units,
/// because that is what the item declares and what a caller re-encodes.
#[derive(Clone, Copy)]
pub(crate) struct Units<'a> {
    bytes: &'a [u8],
    position: usize,
    end: usize,
}

impl Units<'_> {
    pub(crate) fn next(&mut self) -> Result<Option<u16>, InputError> {
        if self.position >= self.end {
            return Ok(None);
        }
        Ok(Some(self.decode()))
    }

    fn position(&self) -> ByteOffset {
        ByteOffset(self.position)
    }

    fn decode(&mut self) -> u16 {
        const REPLACEMENT: u16 = 0xfffd;
        let first = self.bytes[self.position];
        match first {
            0x01..=0x7f => {
                self.position += 1;
                u16::from(first)
            }
            0xc0..=0xdf => match self.byte_at(1) {
                Some(second) if second & 0xc0 == 0x80 => {
                    self.position += 2;
                    (u16::from(first & 31) << 6) | u16::from(second & 63)
                }
                _ => {
                    self.position += 1;
                    REPLACEMENT
                }
            },
            0xe0..=0xef => match (self.byte_at(1), self.byte_at(2)) {
                (Some(second), Some(third)) if second & 0xc0 == 0x80 && third & 0xc0 == 0x80 => {
                    self.position += 3;
                    (u16::from(first & 15) << 12)
                        | (u16::from(second & 63) << 6)
                        | u16::from(third & 63)
                }
                _ => {
                    self.position += 1;
                    REPLACEMENT
                }
            },
            _ => {
                self.position += 1;
                REPLACEMENT
            }
        }
    }

    fn byte_at(&self, ahead: usize) -> Option<u8> {
        let at = self.position.checked_add(ahead)?;
        (at < self.end).then(|| self.bytes[at])
    }
}

/// Unit equality deliberately does not fold across UTF-16 boundaries.
fn unit_eq_ignore_case(left: u16, right: u16) -> bool {
    if left == right {
        return true;
    }
    match (
        char::from_u32(u32::from(left)),
        char::from_u32(u32::from(right)),
    ) {
        (Some(left), Some(right)) => left.to_lowercase().eq(right.to_lowercase()),
        _ => false,
    }
}

/// Byte form of [`validate_name_units`] for an all-ASCII payload.
///
/// The reported offsets match the decoding path: a rejected character is reported
/// past the byte that produced it, and a trailing separator reports the end.
fn validate_name_bytes(payload: &[u8], syntax: NameSyntax, base: usize) -> Result<(), InputError> {
    let mut component = false;
    for (index, &byte) in payload.iter().enumerate() {
        if matches!(syntax, NameSyntax::ClassPath) && byte == b'/' {
            if !component {
                return Err(malformed(
                    InputProblem::InvalidName,
                    ByteOffset(base + index + 1),
                ));
            }
            component = false;
            continue;
        }
        if !matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'$' | b'-' | b'_') {
            return Err(malformed(
                InputProblem::InvalidName,
                ByteOffset(base + index + 1),
            ));
        }
        component = true;
    }
    if !component {
        return Err(malformed(
            InputProblem::InvalidName,
            ByteOffset(base + payload.len()),
        ));
    }
    Ok(())
}

enum NameSyntax {
    Simple,
    ClassPath,
}

fn validate_name_units(mut reader: Units<'_>, syntax: NameSyntax) -> Result<(), InputError> {
    // An all-ASCII payload names the same characters as its bytes, so it is
    // validated without decoding a unit per character. Names and descriptors are
    // almost always ASCII and this check runs once per declaration.
    let payload = reader.bytes.get(reader.position..reader.end);
    if let Some(payload) = payload
        && payload.is_ascii()
    {
        return validate_name_bytes(payload, syntax, reader.position);
    }
    let mut component = false;
    while let Some(unit) = reader.next()? {
        if matches!(syntax, NameSyntax::ClassPath) && unit == u16::from(b'/') {
            if !component {
                return Err(malformed(InputProblem::InvalidName, reader.position()));
            }
            component = false;
            continue;
        }
        let scalar = if (0xd800..=0xdbff).contains(&unit) {
            let low = reader
                .next()?
                .ok_or_else(|| malformed(InputProblem::InvalidName, reader.position()))?;
            if !(0xdc00..=0xdfff).contains(&low) {
                return Err(malformed(InputProblem::InvalidName, reader.position()));
            }
            0x10000 + ((u32::from(unit) - 0xd800) << 10) + u32::from(low) - 0xdc00
        } else {
            u32::from(unit)
        };
        if !matches!(scalar, 0x41..=0x5a | 0x61..=0x7a | 0x30..=0x39 | 0x24 | 0x2d | 0x5f | 0xa1..=0x1fff | 0x2010..=0x2027 | 0x2030..=0xd7ff | 0xe000..=0xffef | 0x10000..=0x10ffff)
        {
            return Err(malformed(InputProblem::InvalidName, reader.position()));
        }
        component = true;
    }
    if !component {
        return Err(malformed(InputProblem::InvalidName, reader.position()));
    }
    Ok(())
}
