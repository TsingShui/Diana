//! Shared bounded Android string-pool reader for binary XML and resource tables.
use super::axml::ManifestError;

pub(super) struct StringPool<'a> {
    data: &'a [u8],
    offsets: &'a [u8],
    utf8: bool,
}
fn bad(offset: usize) -> ManifestError {
    ManifestError::InconsistentChunk { offset }
}
fn word(bytes: &[u8], at: usize) -> Result<u32, ManifestError> {
    let end = at.checked_add(4).ok_or_else(|| bad(at))?;
    let b = bytes.get(at..end).ok_or_else(|| bad(at))?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
impl<'a> StringPool<'a> {
    pub(super) fn parse(
        bytes: &'a [u8],
        offset: usize,
        header: usize,
        end: usize,
    ) -> Result<Self, ManifestError> {
        let chunk = bytes.get(offset..end).ok_or_else(|| bad(offset))?;
        if header < 28 || header > chunk.len() {
            return Err(bad(offset));
        }
        let count =
            usize::try_from(word(chunk, 8)?).map_err(|_| ManifestError::PlatformCapacity)?;
        let styles =
            usize::try_from(word(chunk, 12)?).map_err(|_| ManifestError::PlatformCapacity)?;
        let flags = word(chunk, 16)?;
        let start =
            usize::try_from(word(chunk, 20)?).map_err(|_| ManifestError::PlatformCapacity)?;
        let styles_start =
            usize::try_from(word(chunk, 24)?).map_err(|_| ManifestError::PlatformCapacity)?;
        let table_end = count
            .checked_add(styles)
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| header.checked_add(n))
            .ok_or_else(|| bad(offset))?;
        if start < table_end
            || start > chunk.len()
            || styles != 0 && styles_start == 0
            || styles_start != 0 && (styles_start < start || styles_start > chunk.len())
        {
            return Err(bad(offset));
        }
        let strings_end = if styles_start == 0 {
            chunk.len()
        } else {
            styles_start
        };
        let offsets_end = header
            .checked_add(count.checked_mul(4).ok_or_else(|| bad(offset))?)
            .ok_or_else(|| bad(offset))?;
        let offsets = chunk.get(header..offsets_end).ok_or_else(|| bad(offset))?;
        let data = chunk.get(start..strings_end).ok_or_else(|| bad(offset))?;
        Ok(Self {
            data,
            offsets,
            utf8: flags & 0x100 != 0,
        })
    }
    pub(super) fn text(&self, index: u32) -> Result<String, ManifestError> {
        let invalid = || ManifestError::InvalidString { index };
        let at = usize::try_from(index)
            .map_err(|_| ManifestError::PlatformCapacity)?
            .checked_mul(4)
            .ok_or_else(invalid)?;
        let relative = usize::try_from(word(self.offsets, at)?)
            .map_err(|_| ManifestError::PlatformCapacity)?;
        let bytes = self.data.get(relative..).ok_or_else(invalid)?;
        let mut text = String::new();
        if self.utf8 {
            let (units, next) = prefix8(bytes, 0).ok_or_else(invalid)?;
            let (length, start) = prefix8(bytes, next).ok_or_else(invalid)?;
            let end = start.checked_add(length).ok_or_else(invalid)?;
            let value = bytes.get(start..end).ok_or_else(invalid)?;
            if bytes.get(end) != Some(&0) {
                return Err(invalid());
            }
            let value = std::str::from_utf8(value).map_err(|_| invalid())?;
            if value.encode_utf16().count() != units {
                return Err(invalid());
            }
            text.try_reserve_exact(value.len())
                .map_err(ManifestError::Allocation)?;
            text.push_str(value);
        } else {
            let (units, start) = prefix16(bytes).ok_or_else(invalid)?;
            let end = units
                .checked_mul(2)
                .and_then(|n| start.checked_add(n))
                .ok_or_else(invalid)?;
            let value = bytes.get(start..end).ok_or_else(invalid)?;
            let terminator = end.checked_add(2).and_then(|end| bytes.get(end - 2..end));
            if terminator != Some(&[0, 0][..]) {
                return Err(invalid());
            }
            for scalar in char::decode_utf16(
                value
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|b| u16::from_le_bytes(*b)),
            ) {
                let scalar = scalar.map_err(|_| invalid())?;
                text.try_reserve(scalar.len_utf8())
                    .map_err(ManifestError::Allocation)?;
                text.push(scalar);
            }
        }
        Ok(text)
    }
}
fn prefix8(bytes: &[u8], at: usize) -> Option<(usize, usize)> {
    let first = *bytes.get(at)?;
    if first & 0x80 == 0 {
        Some((usize::from(first), at.checked_add(1)?))
    } else {
        Some((
            (usize::from(first & 0x7f) << 8) | usize::from(*bytes.get(at.checked_add(1)?)?),
            at.checked_add(2)?,
        ))
    }
}
fn prefix16(bytes: &[u8]) -> Option<(usize, usize)> {
    let first = u16::from_le_bytes([*bytes.first()?, *bytes.get(1)?]);
    if first & 0x8000 == 0 {
        Some((usize::from(first), 2))
    } else {
        Some((
            (usize::from(first & 0x7fff) << 16)
                | usize::from(u16::from_le_bytes([*bytes.get(2)?, *bytes.get(3)?])),
            4,
        ))
    }
}
