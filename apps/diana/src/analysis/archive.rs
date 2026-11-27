use super::input::{
    ArchiveProblem, BytesSource, EntryIndex, InputByteLen, InputError, InputLease, InputWindow,
};
use std::io::{self, Write};
use std::sync::Arc;

const CENTRAL_HEADER_SIZE: usize = 46;
const LOCAL_HEADER_SIZE: usize = 30;

/// One physical central-directory record. Names are kept as decoded metadata and
/// require portable component checks before use in an export path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryRecord {
    index: EntryIndex,
    name: EntryName,
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
        self.name.as_str()
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

/// Each archive owns one UTF-8 name buffer. Record ranges are formed from
/// validated strings before the records can leave the directory parser.
#[derive(Clone)]
struct EntryName {
    storage: Option<Arc<String>>,
    start: u32,
    end: u32,
}

impl EntryName {
    fn as_str(&self) -> &str {
        &self
            .storage
            .as_deref()
            .expect("published directory name storage")[self.start as usize..self.end as usize]
    }
}

impl std::fmt::Debug for EntryName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(formatter)
    }
}

impl PartialEq for EntryName {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for EntryName {}

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
        let end_record = end_record(bytes)?;
        let end = end_record
            .offset
            .checked_add(end_record.size)
            .filter(|&end| end <= end_record.start)
            .ok_or(ArchiveProblem::InconsistentDirectory)?;
        let end = usize::try_from(end).map_err(|_| ArchiveProblem::OutOfRange)?;
        let entries =
            usize::try_from(end_record.entries).map_err(|_| ArchiveProblem::OutOfRange)?;
        if end_record.entries > end_record.size / CENTRAL_HEADER_SIZE as u64 {
            return Err(ArchiveProblem::InconsistentDirectory);
        }
        let directory = bytes.get(..end).ok_or(ArchiveProblem::OutOfRange)?;
        let mut cursor =
            usize::try_from(end_record.offset).map_err(|_| ArchiveProblem::OutOfRange)?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(entries)
            .map_err(ArchiveProblem::Allocation)?;
        let mut names = String::new();
        for ordinal in 0..entries {
            result.push(read_record(directory, &mut cursor, ordinal, &mut names)?);
        }
        if cursor != end {
            return Err(ArchiveProblem::InconsistentDirectory);
        }
        let storage = Arc::new(names);
        for entry in &mut result {
            entry.name.storage = Some(Arc::clone(&storage));
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

fn u64_at(bytes: &[u8], offset: usize) -> Result<u64, ArchiveProblem> {
    let field = bytes
        .get(offset..)
        .and_then(<[u8]>::first_chunk::<8>)
        .ok_or(ArchiveProblem::OutOfRange)?;
    Ok(u64::from_le_bytes(*field))
}

struct EndRecord {
    offset: u64,
    size: u64,
    entries: u64,
    start: u64,
}

fn end_record(bytes: &[u8]) -> Result<EndRecord, ArchiveProblem> {
    const SIGNATURE: &[u8; 4] = b"PK\x05\x06";
    let window = bytes.len().min(22 + usize::from(u16::MAX));
    let start = bytes.len() - window;
    let mut found = None;
    let mut saw_signature = false;
    for position in (start..bytes.len().saturating_sub(21)).rev() {
        if bytes.get(position..position + 4) != Some(SIGNATURE) {
            continue;
        }
        saw_signature = true;
        let comment = usize::from(u16_at(bytes, position + 20)?);
        if bytes.len() - position - 22 == comment {
            found = Some(position);
            break;
        }
    }
    let at = found.ok_or(if saw_signature {
        ArchiveProblem::InconsistentDirectory
    } else {
        ArchiveProblem::MissingDirectory
    })?;
    let end = &bytes[at..];
    let disk = u16_at(end, 4)?;
    let directory_disk = u16_at(end, 6)?;
    let disk_entries = u16_at(end, 8)?;
    let entries = u16_at(end, 10)?;
    let size = u32_at(end, 12)?;
    let offset = u32_at(end, 16)?;

    // The common archive has its directory immediately before this record.
    // No locator can occupy those bytes. ZIP64 can also occur with every
    // classic field representable when an encoder explicitly enables it.
    let directory_before_end =
        u64::from(offset) + u64::from(size) != at as u64 || offset == u32::MAX || size == u32::MAX;
    if directory_before_end
        && let Some(locator) = at.checked_sub(20)
        && bytes[locator..].starts_with(b"PK\x06\x07")
    {
        let record = zip64_end_record(bytes, locator)?;
        if !matches!(disk, 0 | u16::MAX) || !matches!(directory_disk, 0 | u16::MAX) {
            return Err(ArchiveProblem::MultiDisk);
        }
        if !legacy_count_agrees(disk_entries, record.entries)
            || !legacy_count_agrees(entries, record.entries)
            || !legacy_word_agrees(size, record.size)
            || !legacy_word_agrees(offset, record.offset)
        {
            return Err(ArchiveProblem::InconsistentDirectory);
        }
        return Ok(record);
    }
    if disk != 0 || directory_disk != 0 {
        return Err(ArchiveProblem::MultiDisk);
    }
    if disk_entries != entries || size == u32::MAX || offset == u32::MAX {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    Ok(EndRecord {
        offset: offset.into(),
        size: size.into(),
        entries: entries.into(),
        start: at as u64,
    })
}

fn legacy_count_agrees(legacy: u16, wide: u64) -> bool {
    legacy == u16::MAX || u64::from(legacy) == wide
}

fn legacy_word_agrees(legacy: u32, wide: u64) -> bool {
    legacy == u32::MAX || u64::from(legacy) == wide
}

fn zip64_end_record(bytes: &[u8], locator: usize) -> Result<EndRecord, ArchiveProblem> {
    let locator_data = &bytes[locator..locator + 20];
    if u32_at(locator_data, 4)? != 0 || u32_at(locator_data, 16)? != 1 {
        return Err(ArchiveProblem::MultiDisk);
    }
    let at = u64_at(locator_data, 8)?;
    let start = usize::try_from(at).map_err(|_| ArchiveProblem::OutOfRange)?;
    let record = bytes
        .get(start..locator)
        .filter(|record| record.starts_with(b"PK\x06\x06"))
        .ok_or(ArchiveProblem::InconsistentDirectory)?;
    let size = u64_at(record, 4)?;
    if size < 44 || size.checked_add(12) != Some(record.len() as u64) {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    if u32_at(record, 16)? != 0 || u32_at(record, 20)? != 0 {
        return Err(ArchiveProblem::MultiDisk);
    }
    let entries = u64_at(record, 32)?;
    if entries != u64_at(record, 24)? {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    Ok(EndRecord {
        offset: u64_at(record, 48)?,
        size: u64_at(record, 40)?,
        entries,
        start: at,
    })
}

fn zip64_extra(mut extra: &[u8]) -> Result<&[u8], ArchiveProblem> {
    while !extra.is_empty() {
        let tag = u16_at(extra, 0)?;
        let size = usize::from(u16_at(extra, 2)?);
        let (field, rest) = extra[4..]
            .split_at_checked(size)
            .ok_or(ArchiveProblem::InconsistentDirectory)?;
        if tag == 1 {
            return Ok(field);
        }
        extra = rest;
    }
    Err(ArchiveProblem::InconsistentDirectory)
}

fn wide_entry_fields(
    extra: &[u8],
    compressed: u32,
    uncompressed: u32,
    local: u32,
    disk: u16,
) -> Result<(u64, u64, u64, u32), ArchiveProblem> {
    // APPNOTE section 4.5.3 orders only the fields with legacy sentinels.
    let extra = zip64_extra(extra)?;
    let mut cursor = 0;
    let mut size = |legacy| {
        if legacy != u32::MAX {
            return Ok(u64::from(legacy));
        }
        let value = u64_at(extra, cursor)?;
        cursor += 8;
        Ok(value)
    };
    let uncompressed = size(uncompressed)?;
    let compressed = size(compressed)?;
    let local = size(local)?;
    let disk = if disk == u16::MAX {
        u32_at(extra, cursor)?
    } else {
        disk.into()
    };
    Ok((compressed, uncompressed, local, disk))
}

fn read_record(
    bytes: &[u8],
    cursor: &mut usize,
    ordinal: usize,
    names: &mut String,
) -> Result<EntryRecord, ArchiveProblem> {
    const SIGNATURE: &[u8; 4] = b"PK\x01\x02";
    let at = *cursor;
    // All fixed-width fields inherit the checked central-record slice.
    let tail = bytes.get(at..).ok_or(ArchiveProblem::OutOfRange)?;
    let header = tail
        .first_chunk::<CENTRAL_HEADER_SIZE>()
        .ok_or(ArchiveProblem::OutOfRange)?;
    if !header.starts_with(SIGNATURE) {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    let word = |index| u16::from_le_bytes([header[index], header[index + 1]]);
    let wide = |index| {
        u32::from_le_bytes([
            header[index],
            header[index + 1],
            header[index + 2],
            header[index + 3],
        ])
    };
    let flags = word(8);
    let method = word(10);
    let crc = wide(16);
    let compressed = wide(20);
    let uncompressed = wide(24);
    let name_len = usize::from(word(28));
    let extra_len = usize::from(word(30));
    let comment_len = usize::from(word(32));
    let disk = word(34);
    let local = wide(42);
    let name_end = CENTRAL_HEADER_SIZE + name_len;
    let extra_end = name_end + extra_len;
    let record_len = extra_end + comment_len;
    if record_len > tail.len() {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    let name = std::str::from_utf8(&tail[CENTRAL_HEADER_SIZE..name_end])
        .map_err(|_| ArchiveProblem::NameEncoding)?;
    if flags & 1 != 0 && name.is_empty() {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    let (compressed, uncompressed, local, disk) = if compressed == u32::MAX
        || uncompressed == u32::MAX
        || local == u32::MAX
        || disk == u16::MAX
    {
        wide_entry_fields(
            &tail[name_end..extra_end],
            compressed,
            uncompressed,
            local,
            disk,
        )?
    } else {
        (
            compressed.into(),
            uncompressed.into(),
            local.into(),
            disk.into(),
        )
    };
    if disk != 0 {
        return Err(ArchiveProblem::MultiDisk);
    }
    let next = at + record_len;
    *cursor = next;
    let name_start = u32::try_from(names.len()).map_err(|_| ArchiveProblem::OutOfRange)?;
    names
        .try_reserve(name.len())
        .map_err(ArchiveProblem::Allocation)?;
    names.push_str(name);
    let name_end = u32::try_from(names.len()).map_err(|_| ArchiveProblem::OutOfRange)?;
    let name = EntryName {
        storage: None,
        start: name_start,
        end: name_end,
    };
    Ok(EntryRecord {
        index: EntryIndex(ordinal),
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
    inflate_exact(&mut decompressor, compressed, &mut buffer[..size])?;
    Ok(size)
}

fn inflate_exact(
    decoder: &mut libdeflater::Decompressor,
    compressed: &[u8],
    output: &mut [u8],
) -> Result<(), ArchiveProblem> {
    let written = decoder
        .deflate_decompress(compressed, output)
        .map_err(|_| ArchiveProblem::DecompressionFailed)?;
    if written != output.len() {
        return Err(ArchiveProblem::DamagedEntry);
    }
    Ok(())
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
    let start =
        usize::try_from(entry.local_header_offset).map_err(|_| ArchiveProblem::OutOfRange)?;
    let local = bytes.get(start..).ok_or(ArchiveProblem::OutOfRange)?;
    let range = payload_range(entry, local, bytes.len() as u64)?;
    let start = usize::try_from(range.start).map_err(|_| ArchiveProblem::OutOfRange)?;
    let end = usize::try_from(range.end).map_err(|_| ArchiveProblem::OutOfRange)?;
    bytes.get(start..end).ok_or(ArchiveProblem::OutOfRange)
}

/// The local header and its original name must fit before the compressed data.
/// Both the whole-entry reader and the sliding export window use this range.
fn payload_range(
    entry: &EntryRecord,
    local: &[u8],
    file_len: u64,
) -> Result<std::ops::Range<u64>, ArchiveProblem> {
    if entry.is_encrypted() {
        return Err(ArchiveProblem::EncryptedEntry);
    }
    if local.get(..4) != Some(b"PK\x03\x04") || u16_at(local, 8)? != entry.method {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    let flags = u16_at(local, 6)?;
    if flags & 1 != 0 {
        return Err(ArchiveProblem::EncryptedEntry);
    }
    let name_end = LOCAL_HEADER_SIZE + usize::from(u16_at(local, 26)?);
    let header_len = name_end + usize::from(u16_at(local, 28)?);
    if local.get(LOCAL_HEADER_SIZE..name_end) != Some(entry.name().as_bytes()) {
        return Err(ArchiveProblem::InconsistentDirectory);
    }
    if local_uses_zip64(local)? {
        let extra = local
            .get(name_end..header_len)
            .ok_or(ArchiveProblem::OutOfRange)?;
        let sizes = zip64_extra(extra)?;
        let uncompressed = u64_at(sizes, 0)?;
        let compressed = u64_at(sizes, 8)?;
        let descriptor = flags & 8 != 0;
        if descriptor != (entry.flags & 8 != 0)
            || !local_size_agrees(compressed, entry.compressed_size.0, descriptor)
            || !local_size_agrees(uncompressed, entry.uncompressed_size.0, descriptor)
            || !local_legacy_size_agrees(u32_at(local, 18)?, compressed, descriptor)
            || !local_legacy_size_agrees(u32_at(local, 22)?, uncompressed, descriptor)
        {
            return Err(ArchiveProblem::DamagedEntry);
        }
    }
    let start = entry
        .local_header_offset
        .checked_add(header_len as u64)
        .ok_or(ArchiveProblem::OutOfRange)?;
    let end = start
        .checked_add(entry.compressed_size.0)
        .filter(|&end| end <= file_len)
        .ok_or(ArchiveProblem::OutOfRange)?;
    Ok(start..end)
}

fn local_uses_zip64(local: &[u8]) -> Result<bool, ArchiveProblem> {
    Ok(u32_at(local, 18)? == u32::MAX || u32_at(local, 22)? == u32::MAX)
}

fn local_size_agrees(local: u64, central: u64, descriptor: bool) -> bool {
    local == central || (descriptor && local == 0)
}

fn local_legacy_size_agrees(legacy: u32, wide: u64, descriptor: bool) -> bool {
    legacy == u32::MAX || local_size_agrees(legacy.into(), wide, descriptor)
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

/// Keeps input failures local to their entry and output failures with the sink.
#[derive(Debug, thiserror::Error)]
pub(super) enum CopyError {
    #[error(transparent)]
    Input(#[from] InputError),
    #[error(transparent)]
    Output(#[from] io::Error),
}

impl From<ArchiveProblem> for CopyError {
    fn from(problem: ArchiveProblem) -> Self {
        Self::Input(InputError::MalformedArchive(problem))
    }
}

/// Decode checked entries from an existing byte lease. The decoder and output
/// buffer are reused by each worker, including across small JVM class entries.
pub(super) struct EntryDecoder {
    small: libdeflater::Decompressor,
    scratch: Vec<u8>,
}

impl EntryDecoder {
    pub(super) fn new() -> Self {
        Self {
            small: libdeflater::Decompressor::new(),
            scratch: Vec::new(),
        }
    }

    pub(super) fn read<'a>(
        &'a mut self,
        bytes: &'a [u8],
        entry: &EntryRecord,
        limit: usize,
    ) -> Result<&'a [u8], InputError> {
        let size = entry_size(entry, limit)?;
        let compressed = compressed_payload(bytes, entry)?;
        let payload = match entry.method {
            0 if compressed.len() == size => compressed,
            0 => return Err(ArchiveProblem::DamagedEntry.into()),
            8 => {
                if size > self.scratch.len() {
                    self.scratch
                        .try_reserve(size - self.scratch.len())
                        .map_err(InputError::Allocation)?;
                    self.scratch.resize(size, 0);
                }
                let output = &mut self.scratch[..size];
                inflate_exact(&mut self.small, compressed, output)?;
                output
            }
            method => return Err(ArchiveProblem::UnsupportedCompression(method).into()),
        };
        if libdeflater::crc32(payload) != entry.crc32 {
            return Err(ArchiveProblem::DamagedEntry.into());
        }
        Ok(payload)
    }

    /// Transfer inflated storage to a nested archive without copying it.
    pub(super) fn take(
        &mut self,
        bytes: &[u8],
        entry: &EntryRecord,
        limit: usize,
    ) -> Result<Vec<u8>, InputError> {
        let payload = self.read(bytes, entry, limit)?;
        if entry.method == 0 {
            let mut owned = Vec::new();
            owned
                .try_reserve_exact(payload.len())
                .map_err(InputError::Allocation)?;
            owned.extend_from_slice(payload);
            return Ok(owned);
        }
        let len = payload.len();
        let mut output = std::mem::take(&mut self.scratch);
        output.truncate(len);
        Ok(output)
    }
}

fn entry_size(entry: &EntryRecord, limit: usize) -> Result<usize, InputError> {
    let size = entry.uncompressed_size.0;
    if size > limit as u64 {
        return Err(InputError::EntrySizeLimit {
            size,
            limit: limit as u64,
        });
    }
    usize::try_from(size).map_err(|_| InputError::PlatformCapacity)
}

pub(super) fn checked_stored_entry<'a>(
    bytes: &'a [u8],
    entry: &EntryRecord,
    limit: usize,
) -> Result<&'a [u8], InputError> {
    let declared = entry_size(entry, limit)?;
    let payload = entry_payload(bytes, entry)?;
    if payload.len() != declared || libdeflater::crc32(payload) != entry.crc32 {
        return Err(ArchiveProblem::DamagedEntry.into());
    }
    Ok(payload)
}

/// Small entries take the whole-entry path. Larger records stream through the
/// same fixed scratch space, checking stream completion and the declared size.
pub(super) struct ExportReader<'a> {
    input: ExportInput<'a>,
    small: libdeflater::Decompressor,
    stream: flate2::Decompress,
    output: Vec<u8>,
}

enum ExportInput<'a> {
    File(InputWindow<'a>),
    Borrowed(&'a [u8]),
}

impl ExportInput<'_> {
    fn len(&self) -> u64 {
        match self {
            Self::File(input) => input.len(),
            Self::Borrowed(bytes) => bytes.len() as u64,
        }
    }

    fn read(&mut self, offset: u64, length: usize) -> Result<&[u8], InputError> {
        match self {
            Self::File(input) => input.read(offset, length),
            Self::Borrowed(bytes) => {
                let start = usize::try_from(offset).map_err(|_| InputError::PlatformCapacity)?;
                let end = start
                    .checked_add(length)
                    .ok_or(ArchiveProblem::OutOfRange)?;
                Ok(bytes.get(start..end).ok_or(ArchiveProblem::OutOfRange)?)
            }
        }
    }
}

impl<'a> ExportReader<'a> {
    pub(super) fn new(input: &'a InputLease) -> Self {
        Self::with_input(ExportInput::File(input.window()))
    }

    pub(super) fn from_slice(bytes: &'a [u8]) -> Self {
        Self::with_input(ExportInput::Borrowed(bytes))
    }

    fn with_input(input: ExportInput<'a>) -> Self {
        Self {
            input,
            small: libdeflater::Decompressor::new(),
            stream: flate2::Decompress::new(false),
            output: vec![0; 64 * 1024],
        }
    }

    pub(super) fn copy(
        &mut self,
        entry: &EntryRecord,
        mut out: impl Write,
    ) -> Result<(), CopyError> {
        let range = self.range(entry)?;
        self.chunks(entry, range, |bytes| {
            out.write_all(bytes).map_err(CopyError::Output)
        })
    }

    fn range(&mut self, entry: &EntryRecord) -> Result<std::ops::Range<u64>, InputError> {
        let file_len = self.input.len();
        let prefix_len = LOCAL_HEADER_SIZE + entry.name().len();
        let local = self.input.read(entry.local_header_offset, prefix_len)?;
        let local = if local_uses_zip64(local)? {
            let header_len = prefix_len + usize::from(u16_at(local, 28)?);
            self.input.read(entry.local_header_offset, header_len)?
        } else {
            local
        };
        let range = payload_range(entry, local, file_len)?;
        if entry.method == 0 && range.end - range.start != entry.uncompressed_size.0 {
            return Err(ArchiveProblem::DamagedEntry.into());
        }
        Ok(range)
    }

    /// Lends the checked payload for a complete DEX image. Stored entries
    /// borrow their own sliding mapping. Inflation grows the caller's reusable
    /// storage only as bytes arrive from the stream.
    pub(super) fn read_entry<'s>(
        &'s mut self,
        entry: &EntryRecord,
        scratch: &'s mut Vec<u8>,
    ) -> Result<&'s [u8], InputError> {
        let range = self.range(entry)?;
        if entry.method == 0 {
            let size = usize::try_from(entry.uncompressed_size.0)
                .map_err(|_| InputError::PlatformCapacity)?;
            let bytes = self.input.read(range.start, size)?;
            if libdeflater::crc32(bytes) != entry.crc32 {
                return Err(ArchiveProblem::DamagedEntry.into());
            }
            return Ok(bytes);
        }
        scratch.clear();
        self.chunks(entry, range, |bytes| {
            scratch
                .try_reserve(bytes.len())
                .map_err(InputError::Allocation)?;
            scratch.extend_from_slice(bytes);
            Ok::<_, InputError>(())
        })?;
        Ok(scratch)
    }

    fn chunks<E: From<ArchiveProblem> + From<InputError>>(
        &mut self,
        entry: &EntryRecord,
        range: std::ops::Range<u64>,
        mut emit: impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut crc = libdeflater::Crc::new();
        let expected = entry.uncompressed_size.0;
        let compressed = range.end - range.start;
        let chunk = self.output.len() as u64;
        match entry.method {
            0 => {
                let mut at = range.start;
                while at < range.end {
                    let n = (range.end - at).min(1024 * 1024) as usize;
                    let bytes = self.input.read(at, n)?;
                    crc.update(bytes);
                    emit(bytes)?;
                    at += n as u64;
                }
            }
            8 if expected <= chunk && compressed <= chunk => {
                let data = self.input.read(range.start, compressed as usize)?;
                let output = &mut self.output[..expected as usize];
                inflate_exact(&mut self.small, data, output)?;
                crc.update(output);
                emit(output)?;
            }
            8 => {
                self.stream.reset(false);
                loop {
                    let consumed = self.stream.total_in();
                    let produced = self.stream.total_out();
                    let n = (compressed - consumed).min(chunk) as usize;
                    let data = self.input.read(range.start + consumed, n)?;
                    let status = self
                        .stream
                        .decompress(data, &mut self.output, flate2::FlushDecompress::None)
                        .map_err(|_| ArchiveProblem::DecompressionFailed)?;
                    let written = self.stream.total_out();
                    if written > expected {
                        return Err(ArchiveProblem::DamagedEntry.into());
                    }
                    let bytes = &self.output[..(written - produced) as usize];
                    crc.update(bytes);
                    emit(bytes)?;
                    if status == flate2::Status::StreamEnd {
                        if written != expected {
                            return Err(ArchiveProblem::DamagedEntry.into());
                        }
                        break;
                    }
                    if consumed == self.stream.total_in() && written == produced {
                        return Err(ArchiveProblem::DecompressionFailed.into());
                    }
                }
            }
            method => return Err(ArchiveProblem::UnsupportedCompression(method).into()),
        }
        if crc.sum() != entry.crc32 {
            return Err(ArchiveProblem::DamagedEntry.into());
        }
        Ok(())
    }
}
