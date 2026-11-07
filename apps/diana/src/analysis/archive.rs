use super::input::{ArchiveProblem, BytesSource, EntryIndex, InputByteLen, InputLease};

/// One physical central-directory record. Names are kept as decoded metadata and
/// never used as output paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryRecord {
    index: EntryIndex,
    name: String,
    flags: u16,
    method: u16,
    compressed_size: InputByteLen,
    uncompressed_size: InputByteLen,
    crc32: u32,
    local_header_offset: u64,
}

impl EntryRecord {
    pub fn index(&self) -> EntryIndex {
        self.index
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn method(&self) -> u16 {
        self.method
    }
    pub fn compressed_size(&self) -> InputByteLen {
        self.compressed_size
    }
    pub fn uncompressed_size(&self) -> InputByteLen {
        self.uncompressed_size
    }
    pub fn crc32(&self) -> u32 {
        self.crc32
    }
    pub fn local_header_offset(&self) -> u64 {
        self.local_header_offset
    }
    pub fn is_encrypted(&self) -> bool {
        self.flags & 1 != 0
    }
}

/// Restricted entry metadata in physical directory order. It owns no payload and
/// borrows no input bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveDirectory {
    entries: Vec<EntryRecord>,
}

impl ArchiveDirectory {
    /// Reads the end-of-central-directory record and every central-directory
    /// record inside the leased bytes. Counts, offsets and total sizes are checked
    /// against the actual input before any record is trusted.
    pub fn parse(bytes: &[u8]) -> Result<Self, ArchiveProblem> {
        let (directory_offset, directory_size, entries) = end_record(bytes)?;

        let mut result = Vec::new();
        result
            .try_reserve_exact(usize::try_from(entries).map_err(|_| ArchiveProblem::OutOfRange)?)
            .map_err(ArchiveProblem::Allocation)?;
        let end = directory_offset
            .checked_add(directory_size)
            .ok_or(ArchiveProblem::OutOfRange)?;
        let end = usize::try_from(end).map_err(|_| ArchiveProblem::OutOfRange)?;
        if end > bytes.len() {
            return Err(ArchiveProblem::OutOfRange);
        }
        let mut cursor =
            usize::try_from(directory_offset).map_err(|_| ArchiveProblem::OutOfRange)?;
        for ordinal in 0..entries {
            let record = read_record(bytes, &mut cursor, end, ordinal)?;

            result.push(record);
        }
        if cursor != end {
            return Err(ArchiveProblem::InconsistentDirectory);
        }
        Ok(Self { entries: result })
    }

    pub fn entries(&self) -> &[EntryRecord] {
        &self.entries
    }

    pub fn entry(&self, index: EntryIndex) -> Option<&EntryRecord> {
        self.entries.get(index.0)
    }
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, ArchiveProblem> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or(ArchiveProblem::OutOfRange)?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, ArchiveProblem> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(ArchiveProblem::OutOfRange)?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn end_record(bytes: &[u8]) -> Result<(u64, u64, u32), ArchiveProblem> {
    const SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
    let window = bytes.len().min(22 + usize::from(u16::MAX));
    let start = bytes.len() - window;
    let mut found = None;
    for position in (start..bytes.len().saturating_sub(21)).rev() {
        if bytes.get(position..position + 4) == Some(&SIGNATURE) {
            found = Some(position);
            break;
        }
    }
    let at = found.ok_or(ArchiveProblem::MissingDirectory)?;
    let comment = usize::from(u16_at(bytes, at + 20)?);
    if at.checked_add(22).and_then(|end| end.checked_add(comment)) != Some(bytes.len()) {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    if u16_at(bytes, at + 4)? != 0
        || u16_at(bytes, at + 6)? != 0
        || u16_at(bytes, at + 8)? != u16_at(bytes, at + 10)?
    {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    let entries = u32::from(u16_at(bytes, at + 10)?);
    let size = u32_at(bytes, at + 12)?;
    let offset = u32_at(bytes, at + 16)?;
    if entries == u32::from(u16::MAX) || size == u32::MAX || offset == u32::MAX {
        return Err(ArchiveProblem::UnsupportedZip64);
    }
    let end = u64::from(offset)
        .checked_add(u64::from(size))
        .ok_or(ArchiveProblem::OutOfRange)?;
    if end > u64::try_from(at).map_err(|_| ArchiveProblem::OutOfRange)? {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    Ok((u64::from(offset), u64::from(size), entries))
}

fn read_record(
    bytes: &[u8],
    cursor: &mut usize,
    end: usize,
    ordinal: u32,
) -> Result<EntryRecord, ArchiveProblem> {
    const SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];
    let at = *cursor;
    if bytes.get(at..at + 4) != Some(&SIGNATURE) {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    let flags = u16_at(bytes, at + 8)?;
    let method = u16_at(bytes, at + 10)?;
    let crc = u32_at(bytes, at + 16)?;
    let compressed = u64::from(u32_at(bytes, at + 20)?);
    let uncompressed = u64::from(u32_at(bytes, at + 24)?);
    let name_len = usize::from(u16_at(bytes, at + 28)?);
    let extra_len = usize::from(u16_at(bytes, at + 30)?);
    let comment_len = usize::from(u16_at(bytes, at + 32)?);
    let local = u64::from(u32_at(bytes, at + 42)?);
    let name_at = at + 46;
    let name_bytes = bytes
        .get(name_at..name_at + name_len)
        .ok_or(ArchiveProblem::OutOfRange)?;
    let name = std::str::from_utf8(name_bytes)
        .map_err(|_| ArchiveProblem::NameEncoding)?
        .to_owned();
    if flags & 1 != 0 && name.is_empty() {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    let next = name_at
        .checked_add(name_len)
        .and_then(|value| value.checked_add(extra_len))
        .and_then(|value| value.checked_add(comment_len))
        .ok_or(ArchiveProblem::OutOfRange)?;
    if next > end {
        return Err(ArchiveProblem::OutOfRange);
    }
    if compressed == u64::from(u32::MAX) || uncompressed == u64::from(u32::MAX) {
        return Err(ArchiveProblem::UnsupportedZip64);
    }
    *cursor = next;
    Ok(EntryRecord {
        index: EntryIndex(usize::try_from(ordinal).map_err(|_| ArchiveProblem::OutOfRange)?),
        name,
        flags,
        method,
        compressed_size: InputByteLen(compressed),
        uncompressed_size: InputByteLen(uncompressed),
        crc32: crc,
        local_header_offset: local,
    })
}

/// One entry payload: a borrow for a stored record or an owned inflate result.
#[derive(Debug)]
pub enum EntryPayload<'a> {
    Borrowed(&'a [u8]),
    Inflated(Vec<u8>),
}

impl EntryPayload<'_> {
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Borrowed(bytes) => bytes,
            Self::Inflated(bytes) => bytes,
        }
    }
}

/// Reads one entry payload and checks that its local header agrees with the
/// central record. Stored payloads are borrowed; deflate payloads are inflated
/// under the declared size. The declared size and the deflate stream itself are
/// the only integrity facts this reader charges for.
pub fn read_entry<'a>(
    bytes: &'a [u8],
    entry: &EntryRecord,
) -> Result<EntryPayload<'a>, ArchiveProblem> {
    let compressed = compressed_payload(bytes, entry)?;
    match entry.method {
        0 => {
            if entry.compressed_size.0 != entry.uncompressed_size.0 {
                return Err(ArchiveProblem::DamagedEntry);
            }
            Ok(EntryPayload::Borrowed(compressed))
        }
        8 => inflate(compressed, entry),
        method => Err(ArchiveProblem::UnsupportedCompression(method)),
    }
}

fn inflate<'a>(compressed: &[u8], entry: &EntryRecord) -> Result<EntryPayload<'a>, ArchiveProblem> {
    let mut output = Vec::new();
    let written = inflate_bytes(compressed, entry, &mut output)?;
    output.truncate(written);
    Ok(EntryPayload::Inflated(output))
}

/// Inflates one entry into a caller-owned buffer and returns the payload length.
///
/// The buffer is only grown, never re-zeroed: a scan inflates dozens of entries
/// through the same pages, and first-touching fresh pages is what made a
/// per-entry allocation cost about twice the decompression itself. The payload is
/// `buffer[..written]`.
pub fn inflate_into(
    bytes: &[u8],
    entry: &EntryRecord,
    buffer: &mut Vec<u8>,
) -> Result<usize, ArchiveProblem> {
    let compressed = compressed_payload(bytes, entry)?;
    inflate_bytes(compressed, entry, buffer)
}

fn inflate_bytes(
    compressed: &[u8],
    entry: &EntryRecord,
    buffer: &mut Vec<u8>,
) -> Result<usize, ArchiveProblem> {
    let declared = entry.uncompressed_size.0;
    let size = usize::try_from(declared).map_err(|_| ArchiveProblem::OutOfRange)?;
    if buffer.len() < size {
        buffer
            .try_reserve_exact(size - buffer.len())
            .map_err(ArchiveProblem::Allocation)?;
        buffer.resize(size, 0);
    }
    let mut decompressor = libdeflater::Decompressor::new();
    let written = decompressor
        .deflate_decompress(compressed, &mut buffer[..size])
        .map_err(|_| ArchiveProblem::DecompressionFailed)?;
    if written != size {
        return Err(ArchiveProblem::DamagedEntry);
    }
    Ok(written)
}

/// A streaming prefix cursor bound to one compressed entry and its scratch buffer.
/// Each growth resumes from measured input/output positions; no bytes are replayed.
pub(super) struct PrefixReader<'a, 's> {
    compressed: &'a [u8],
    declared: usize,
    buffer: &'s mut Vec<u8>,
    decoder: flate2::Decompress,
}

impl<'a, 's> PrefixReader<'a, 's> {
    pub(super) fn new(
        bytes: &'a [u8],
        entry: &EntryRecord,
        buffer: &'s mut Vec<u8>,
    ) -> Result<Self, ArchiveProblem> {
        Ok(Self {
            compressed: compressed_payload(bytes, entry)?,
            declared: usize::try_from(entry.uncompressed_size.0)
                .map_err(|_| ArchiveProblem::OutOfRange)?,
            buffer,
            decoder: flate2::Decompress::new(false),
        })
    }

    /// Only total_out establishes which bytes belong to this entry. A stalled or
    /// short stream is an error, never a successful target-length prefix.
    pub(super) fn read(&mut self, aim: usize) -> Result<&[u8], ArchiveProblem> {
        let aim = aim.min(self.declared);
        if self.buffer.len() < aim {
            self.buffer
                .try_reserve_exact(aim - self.buffer.len())
                .map_err(ArchiveProblem::Allocation)?;
            self.buffer.resize(aim, 0);
        }
        loop {
            let consumed =
                usize::try_from(self.decoder.total_in()).map_err(|_| ArchiveProblem::OutOfRange)?;
            let written = usize::try_from(self.decoder.total_out())
                .map_err(|_| ArchiveProblem::OutOfRange)?;
            if written >= aim && aim < self.declared {
                return self.buffer.get(..written).ok_or(ArchiveProblem::OutOfRange);
            }
            let status = self
                .decoder
                .decompress(
                    self.compressed
                        .get(consumed..)
                        .ok_or(ArchiveProblem::OutOfRange)?,
                    self.buffer
                        .get_mut(written..aim)
                        .ok_or(ArchiveProblem::OutOfRange)?,
                    flate2::FlushDecompress::None,
                )
                .map_err(|_| ArchiveProblem::DecompressionFailed)?;
            let end = usize::try_from(self.decoder.total_out())
                .map_err(|_| ArchiveProblem::OutOfRange)?;
            if status == flate2::Status::StreamEnd {
                if end != self.declared {
                    return Err(ArchiveProblem::DamagedEntry);
                }
                return self.buffer.get(..end).ok_or(ArchiveProblem::OutOfRange);
            }
            if self.decoder.total_in() == consumed as u64 && end == written {
                return Err(ArchiveProblem::DecompressionFailed);
            }
        }
    }
}

/// Reads one stored entry payload. A deflate record needs `read_entry` with its
/// configured inflation bound instead.
pub fn entry_payload<'a>(bytes: &'a [u8], entry: &EntryRecord) -> Result<&'a [u8], ArchiveProblem> {
    match read_entry(bytes, entry)? {
        EntryPayload::Borrowed(payload) => Ok(payload),
        EntryPayload::Inflated(_) => Err(ArchiveProblem::UnsupportedCompression(entry.method)),
    }
}

fn compressed_payload<'a>(
    bytes: &'a [u8],
    entry: &EntryRecord,
) -> Result<&'a [u8], ArchiveProblem> {
    if entry.is_encrypted() {
        return Err(ArchiveProblem::EncryptedEntry);
    }
    const SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];
    let at = usize::try_from(entry.local_header_offset).map_err(|_| ArchiveProblem::OutOfRange)?;
    if bytes.get(at..at + 4) != Some(&SIGNATURE) {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    let method = u16_at(bytes, at + 8)?;
    let name_len = usize::from(u16_at(bytes, at + 26)?);
    let extra_len = usize::from(u16_at(bytes, at + 28)?);
    let name_at = at + 30;
    let name = bytes
        .get(name_at..name_at + name_len)
        .ok_or(ArchiveProblem::OutOfRange)?;
    if method != entry.method || name != entry.name.as_bytes() {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    let start = name_at
        .checked_add(name_len)
        .and_then(|value| value.checked_add(extra_len))
        .ok_or(ArchiveProblem::OutOfRange)?;
    let size = usize::try_from(entry.compressed_size.0).map_err(|_| ArchiveProblem::OutOfRange)?;
    let end = start.checked_add(size).ok_or(ArchiveProblem::OutOfRange)?;
    bytes.get(start..end).ok_or(ArchiveProblem::OutOfRange)
}

/// A borrowed payload view that cannot outlive its lease.
#[derive(Debug)]
pub struct EntryView<'a> {
    entry: &'a EntryRecord,
    payload: &'a [u8],
}

impl BytesSource for EntryView<'_> {
    fn bytes(&self) -> &[u8] {
        self.payload
    }
}

impl<'a> EntryView<'a> {
    pub fn from_lease(lease: &'a InputLease, index: EntryIndex) -> Result<Self, ArchiveProblem> {
        let directory = lease
            .archive()
            .ok_or(ArchiveProblem::InconsistentDirectory)?;
        let entry = directory.entry(index).ok_or(ArchiveProblem::UnknownEntry)?;
        Ok(Self {
            entry,
            payload: entry_payload(lease.bytes(), entry)?,
        })
    }
    pub fn record(&self) -> &'a EntryRecord {
        self.entry
    }
    pub fn payload(&self) -> &'a [u8] {
        self.payload
    }
}
