use std::borrow::Cow;

use super::{error, ClassError, ClassProblem};

/// A checked, length-delimited JVM modified UTF-8 value. Original UTF-16
/// code units, including isolated surrogates, remain available for identity.
#[derive(Clone, Copy, Debug)]
pub struct JvmString<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl PartialEq for JvmString<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for JvmString<'_> {}

impl<'a> JvmString<'a> {
    pub(super) fn new(bytes: &'a [u8], offset: usize) -> Result<Self, ClassError> {
        let value = Self { bytes, offset };
        if nonzero_ascii(bytes) {
            return Ok(value);
        }
        let mut rest = bytes;
        while !rest.is_empty() {
            let Some((_, tail)) = decode_unit(rest) else {
                return Err(error(
                    offset + bytes.len() - rest.len(),
                    ClassProblem::InvalidModifiedUtf8,
                ));
            };
            rest = tail;
        }
        Ok(value)
    }

    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }

    pub fn offset(self) -> usize {
        self.offset
    }

    pub fn utf8(self) -> Option<&'a str> {
        std::str::from_utf8(self.bytes).ok()
    }

    pub fn units(self) -> Utf16Units<'a> {
        Utf16Units(self.bytes)
    }

    pub fn to_utf16(self) -> Result<Vec<u16>, ClassError> {
        let mut units = Vec::new();
        units
            .try_reserve_exact(self.bytes.len())
            .map_err(|_| error(self.offset, ClassProblem::Allocation))?;
        units.extend(self.units());
        Ok(units)
    }

    /// Unicode projection for filtering. The original identity stays in the
    /// UTF-16 view when a string contains isolated surrogate units.
    pub fn utf8_lossy(self) -> Result<Cow<'a, str>, ClassError> {
        if let Some(value) = self.utf8() {
            return Ok(Cow::Borrowed(value));
        }
        let mut text = String::new();
        text.try_reserve_exact(self.bytes.len())
            .map_err(|_| error(self.offset, ClassProblem::Allocation))?;
        for character in char::decode_utf16(self.units()) {
            text.push(character.unwrap_or(char::REPLACEMENT_CHARACTER));
        }
        Ok(Cow::Owned(text))
    }

    pub(super) fn binary_name(self) -> Result<Self, ClassError> {
        if binary_name(self.bytes) {
            Ok(self)
        } else {
            Err(error(self.offset, ClassProblem::InvalidName))
        }
    }

    pub(super) fn array_or_binary_name(self) -> Result<Self, ClassError> {
        if self.bytes.starts_with(b"[") {
            self.descriptor(false)
        } else {
            self.binary_name()
        }
    }

    pub(super) fn member_name(self, method: bool) -> Result<Self, ClassError> {
        let special = method && matches!(self.bytes, b"<init>" | b"<clinit>");
        let method_name = !method || !self.bytes.iter().any(|byte| matches!(byte, b'<' | b'>'));
        if special || (method_name && unqualified_name(self.bytes)) {
            Ok(self)
        } else {
            Err(error(self.offset, ClassProblem::InvalidName))
        }
    }

    pub(super) fn parameter_count(self) -> Option<usize> {
        method_parameters(self.bytes)
    }

    pub(super) fn descriptor(self, method: bool) -> Result<Self, ClassError> {
        let valid = if method {
            method_parameters(self.bytes).is_some()
        } else {
            field_type(self.bytes).is_some_and(|(tail, _)| tail.is_empty())
        };
        if valid {
            Ok(self)
        } else {
            Err(error(self.offset, ClassProblem::InvalidDescriptor))
        }
    }
}

#[derive(Clone, Debug)]
pub struct Utf16Units<'a>(&'a [u8]);

impl Iterator for Utf16Units<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        let (unit, tail) = decode_unit(self.0)?;
        self.0 = tail;
        Some(unit)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len().div_ceil(3), Some(self.0.len()))
    }
}

impl std::iter::FusedIterator for Utf16Units<'_> {}

fn nonzero_ascii(bytes: &[u8]) -> bool {
    let (words, tail) = bytes.as_chunks::<8>();
    words.iter().all(|&word| {
        let word = u64::from_ne_bytes(word);
        // If every byte is 1..=127, subtracting one per byte cannot borrow
        // or set a high bit. The least significant zero does set a high bit
        // through a borrow, and any original non-ASCII byte already has one.
        (word | word.wrapping_sub(0x0101_0101_0101_0101)) & 0x8080_8080_8080_8080 == 0
    }) && tail.iter().all(|&byte| (1..=127).contains(&byte))
}

fn decode_unit(bytes: &[u8]) -> Option<(u16, &[u8])> {
    match bytes {
        [first @ 1..=0x7f, rest @ ..] => Some((u16::from(*first), rest)),
        [0xc0, 0x80, rest @ ..] => Some((0, rest)),
        [first @ 0xc2..=0xdf, second @ 0x80..=0xbf, rest @ ..] => Some((
            (u16::from(*first & 31) << 6) | u16::from(*second & 63),
            rest,
        )),
        [first @ 0xe0..=0xef, second @ 0x80..=0xbf, third @ 0x80..=0xbf, rest @ ..]
            if *first != 0xe0 || *second >= 0xa0 =>
        {
            let unit = (u16::from(*first & 15) << 12)
                | (u16::from(*second & 63) << 6)
                | u16::from(*third & 63);
            Some((unit, rest))
        }
        _ => None,
    }
}

fn unqualified_name(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && !bytes
            .iter()
            .any(|byte| matches!(byte, b'.' | b';' | b'[' | b'/'))
}

fn binary_name(bytes: &[u8]) -> bool {
    bytes.split(|&byte| byte == b'/').all(unqualified_name)
}

fn field_type(mut bytes: &[u8]) -> Option<(&[u8], u16)> {
    let mut dimensions = 0usize;
    while let Some(tail) = bytes.strip_prefix(b"[") {
        dimensions += 1;
        if dimensions > 255 {
            return None;
        }
        bytes = tail;
    }
    let (tag, tail) = bytes.split_first()?;
    match tag {
        b'B' | b'C' | b'F' | b'I' | b'S' | b'Z' => Some((tail, 1)),
        b'D' | b'J' => Some((tail, if dimensions == 0 { 2 } else { 1 })),
        b'L' => {
            let end = memchr::memchr(b';', tail)?;
            binary_name(&tail[..end]).then(|| (&tail[end + 1..], 1))
        }
        _ => None,
    }
}

fn method_parameters(bytes: &[u8]) -> Option<usize> {
    let mut rest = bytes.strip_prefix(b"(")?;
    let mut slots = 0;
    let mut count = 0;
    loop {
        if let Some(result) = rest.strip_prefix(b")") {
            return (result == b"V" || field_type(result).is_some_and(|(tail, _)| tail.is_empty()))
                .then_some(count);
        }
        let (tail, width) = field_type(rest)?;
        slots += width;
        if slots > 255 {
            return None;
        }
        count += 1;
        rest = tail;
    }
}
