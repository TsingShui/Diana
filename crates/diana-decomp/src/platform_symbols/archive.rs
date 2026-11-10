//! Class-granular storage for `.dexsym` version 3.
//!
//! A request touches a small number of platform classes, so the archive keeps
//! only a compact descriptor index resident and decompresses one block of class
//! records on demand. Version 2 files remain readable through the eager codec.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io::{self, Cursor};
use std::sync::{Arc, Mutex};

use super::codec::{invalid_data, BinaryReader, BinaryWriter, StringPool};
use super::{
    DexSymbolsCodec, PlatformClass, PlatformFamily, PlatformSymbolDatabase, PlatformTarget,
    SymbolAvailability, SymbolSource,
};

/// Class records per compressed block. Blocks stay small enough that decoding
/// one costs far less than re-reading the whole database.
const BLOCK_CLASSES: usize = 64;
/// Decoded blocks kept for the lifetime of one request.
const CACHE_BLOCKS: usize = 4;
const NONE: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Span {
    pub(super) offset: u32,
    pub(super) len: u32,
}

impl Span {
    fn write(self, writer: &mut BinaryWriter) {
        writer.u32(self.offset);
        writer.u32(self.len);
    }

    fn read(reader: &mut BinaryReader<'_>) -> io::Result<Self> {
        Ok(Self {
            offset: reader.u32()?,
            len: reader.u32()?,
        })
    }

    pub(super) fn bytes(self, strings: &[u8]) -> io::Result<&str> {
        let start = self.offset as usize;
        let end = start
            .checked_add(self.len as usize)
            .filter(|end| *end <= strings.len())
            .ok_or_else(|| invalid_data("dexsym string span exceeds the pool"))?;
        std::str::from_utf8(&strings[start..end])
            .map_err(|source| io::Error::new(io::ErrorKind::InvalidData, source))
    }
}

#[derive(Debug, Clone)]
pub(super) struct ArchiveRecord {
    pub(super) descriptor: Span,
    pub(super) source: u32,
    pub(super) availability: SymbolAvailability,
    pub(super) block: u32,
    pub(super) offset: u32,
    pub(super) access_flags: u32,
    pub(super) super_class: Option<Span>,
    pub(super) interface_start: u32,
    pub(super) interface_count: u32,
    pub(super) fields: u32,
    pub(super) methods: u32,
}

#[derive(Debug)]
pub(super) struct ArchiveIndex {
    pub(super) default_target: PlatformTarget,
    pub(super) sources: Vec<SymbolSource>,
    /// The decoded index frame; descriptor spans address its string region.
    raw: Vec<u8>,
    strings: Span,
    pub(super) records: Vec<ArchiveRecord>,
    pub(super) interfaces: Vec<Span>,
    pub(super) block_lens: Vec<u32>,
}

impl ArchiveIndex {
    fn string(&self, span: Span) -> io::Result<&str> {
        let base = self.strings.offset as usize + span.offset as usize;
        let end = base
            .checked_add(span.len as usize)
            .filter(|end| *end <= self.raw.len())
            .ok_or_else(|| invalid_data("dexsym string span exceeds the pool"))?;
        std::str::from_utf8(&self.raw[base..end])
            .map_err(|source| io::Error::new(io::ErrorKind::InvalidData, source))
    }
}

struct DecodedBlock {
    raw: Vec<u8>,
    strings: StringPool,
}

/// One immutable archive of class records plus its resident index.
pub(crate) struct SymbolArchive {
    index: ArchiveIndex,
    bytes: Cow<'static, [u8]>,
    blocks: Span,
    block_offsets: Vec<u32>,
    cache: Mutex<Vec<(u32, Arc<DecodedBlock>)>>,
}

impl SymbolArchive {
    pub(crate) fn open(bytes: Cow<'static, [u8]>) -> io::Result<Self> {
        let mut reader = BinaryReader::new(&bytes);
        if reader.bytes(DexSymbolsCodec::MAGIC.len())? != DexSymbolsCodec::MAGIC {
            return Err(invalid_data("invalid dexsym header"));
        }
        let major = reader.u16()?;
        let minor = reader.u16()?;
        if major != DexSymbolsCodec::MAJOR || minor > DexSymbolsCodec::MINOR {
            return Err(invalid_data("unsupported dexsym format version"));
        }
        let flags = reader.u32()?;
        if flags & !DexSymbolsCodec::FLAG_ZSTD != 0 {
            return Err(invalid_data("unsupported dexsym feature flags"));
        }
        let index_stored = reader.usize_from_u64("index")?;
        let index_raw = reader.usize_from_u64("index payload")?;
        let index_checksum = reader.u32()?;
        let blocks_checksum = reader.u32()?;
        let blocks_len = reader.usize_from_u64("blocks")?;
        if index_stored > DexSymbolsCodec::MAX_PAYLOAD || blocks_len > DexSymbolsCodec::MAX_PAYLOAD
        {
            return Err(invalid_data("dexsym payload exceeds safety limit"));
        }
        let stored = reader.bytes(index_stored)?;
        let decoded = zstd::stream::decode_all(Cursor::new(stored))?;
        if decoded.len() != index_raw {
            return Err(invalid_data("dexsym index length mismatch"));
        }
        if crc32fast::hash(&decoded) != index_checksum {
            return Err(invalid_data("dexsym index checksum mismatch"));
        }
        let blocks = Span {
            offset: u32::try_from(reader.position())
                .map_err(|_| invalid_data("dexsym blocks exceed the payload"))?,
            len: u32::try_from(blocks_len)
                .map_err(|_| invalid_data("dexsym blocks exceed the payload"))?,
        };
        let index = read_index(decoded)?;
        reader.bytes(blocks_len)?;
        reader.finish()?;
        let block_bytes = bytes
            .get(blocks.offset as usize..blocks.offset as usize + blocks.len as usize)
            .ok_or_else(|| invalid_data("dexsym blocks exceed the payload"))?;
        if crc32fast::hash(block_bytes) != blocks_checksum {
            return Err(invalid_data("dexsym block checksum mismatch"));
        }
        let mut block_offsets = Vec::with_capacity(index.block_lens.len());
        let mut offset = 0usize;
        for len in &index.block_lens {
            block_offsets.push(offset as u32);
            offset = offset
                .checked_add(*len as usize)
                .ok_or_else(|| invalid_data("dexsym block offsets overflow"))?;
        }
        if offset != blocks_len {
            return Err(invalid_data("dexsym block lengths contradict the payload"));
        }
        Ok(Self {
            index,
            bytes,
            blocks,
            block_offsets,
            cache: Mutex::new(Vec::new()),
        })
    }

    pub(crate) fn default_target(&self) -> PlatformTarget {
        self.index.default_target
    }

    pub(crate) fn sources(&self) -> &[SymbolSource] {
        &self.index.sources
    }

    pub(crate) fn descriptor(&self, record: u32) -> &str {
        self.index
            .string(self.index.records[record as usize].descriptor)
            .unwrap_or_default()
    }

    pub(crate) fn hierarchy(&self, record: u32) -> (&str, Option<&str>, Vec<&str>, u32) {
        let entry = &self.index.records[record as usize];
        let parents = (0..entry.interface_count as usize)
            .filter_map(|index| {
                let span = *self
                    .index
                    .interfaces
                    .get(entry.interface_start as usize + index)?;
                self.index.string(span).ok()
            })
            .collect();
        let super_class = entry
            .super_class
            .and_then(|span| self.index.string(span).ok());
        (
            self.descriptor(record),
            super_class,
            parents,
            entry.access_flags,
        )
    }

    /// Picks the best variant of every descriptor for one target without
    /// decoding any class record.
    pub(crate) fn select(&self, target: PlatformTarget) -> Vec<u32> {
        let mut selected = Vec::new();
        let mut start = 0usize;
        while start < self.index.records.len() {
            let descriptor = self.index.records[start].descriptor;
            let mut end = start + 1;
            while end < self.index.records.len() && self.index.records[end].descriptor == descriptor
            {
                end += 1;
            }
            if let Some(best) = self.best(&self.index.records[start..end], target) {
                selected.push((start + best) as u32);
            }
            start = end;
        }
        selected
    }

    fn best(&self, records: &[ArchiveRecord], target: PlatformTarget) -> Option<usize> {
        let mut best = None;
        let mut best_key = None;
        for (index, record) in records.iter().enumerate() {
            let Some(source) = self.index.sources.get(record.source as usize) else {
                continue;
            };
            let family = match (target.family, source.family) {
                (expected, actual) if expected == actual => 3,
                (PlatformFamily::Android, PlatformFamily::Java) => 2,
                (_, PlatformFamily::Library) => 1,
                _ => 0,
            };
            let availability = if record.availability.contains(target.version) {
                (3, 0)
            } else if record.availability.until < target.version {
                (
                    2,
                    i32::from(record.availability.until) - i32::from(target.version),
                )
            } else {
                (
                    1,
                    i32::from(target.version) - i32::from(record.availability.since),
                )
            };
            let key = (family, availability, source.priority);
            if best_key.is_none_or(|current| key >= current) {
                best_key = Some(key);
                best = Some(index);
            }
        }
        best
    }

    pub(crate) fn decode_record(&self, record: u32) -> io::Result<PlatformClass> {
        let entry = self
            .index
            .records
            .get(record as usize)
            .ok_or_else(|| invalid_data("dexsym record index is out of range"))?;
        let block = self.block(entry.block)?;
        let mut reader = BinaryReader::at(&block.raw, entry.offset as usize)?;
        DexSymbolsCodec::read_class(&mut reader, &block.strings, self.index.sources.len())
    }

    fn block(&self, id: u32) -> io::Result<Arc<DecodedBlock>> {
        if let Some(hit) = self
            .cache
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .iter()
            .find(|(candidate, _)| *candidate == id)
        {
            return Ok(Arc::clone(&hit.1));
        }
        let start = *self
            .block_offsets
            .get(id as usize)
            .ok_or_else(|| invalid_data("dexsym block index is out of range"))?
            as usize;
        let len = *self
            .index
            .block_lens
            .get(id as usize)
            .ok_or_else(|| invalid_data("dexsym block index is out of range"))?
            as usize;
        let start = self.blocks.offset as usize + start;
        let stored = self
            .bytes
            .get(start..start + len)
            .ok_or_else(|| invalid_data("dexsym block exceeds the payload"))?;
        let raw = zstd::stream::decode_all(Cursor::new(stored))?;
        let mut reader = BinaryReader::new(&raw);
        let strings = StringPool::read(&mut reader)?;
        let block = Arc::new(DecodedBlock { raw, strings });
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        cache.insert(0, (id, Arc::clone(&block)));
        cache.truncate(CACHE_BLOCKS);
        Ok(block)
    }

    pub(crate) fn stats(&self, selected: &[u32]) -> super::SymbolDatabaseStats {
        let mut fields = 0usize;
        let mut methods = 0usize;
        for record in selected {
            let entry = &self.index.records[*record as usize];
            fields += entry.fields as usize;
            methods += entry.methods as usize;
        }
        super::SymbolDatabaseStats {
            sources: self.index.sources.len(),
            class_variants: self.index.records.len(),
            selected_classes: selected.len(),
            fields,
            methods,
        }
    }
}

fn read_index(raw: Vec<u8>) -> io::Result<ArchiveIndex> {
    let mut reader = BinaryReader::new(&raw);
    let default_target = PlatformTarget {
        family: PlatformFamily::from_code(reader.u8()?)?,
        version: reader.u16()?,
    };
    let source_count = reader.count("source")?;
    let mut sources = Vec::with_capacity(source_count);
    for _ in 0..source_count {
        let len = reader.count("source name byte")?;
        let name = std::str::from_utf8(reader.bytes(len)?)
            .map_err(|source| io::Error::new(io::ErrorKind::InvalidData, source))?
            .to_string();
        sources.push(SymbolSource {
            name,
            family: PlatformFamily::from_code(reader.u8()?)?,
            priority: reader.i16()?,
        });
    }
    let strings_len = reader.count("string byte")?;
    let strings_offset = reader.position();
    reader.bytes(strings_len)?;
    let strings = Span {
        offset: u32::try_from(strings_offset)
            .map_err(|_| invalid_data("dexsym strings exceed the payload"))?,
        len: u32::try_from(strings_len)
            .map_err(|_| invalid_data("dexsym strings exceed the payload"))?,
    };
    let record_count = reader.count("record")?;
    let mut records = Vec::with_capacity(record_count);
    for _ in 0..record_count {
        let descriptor = Span::read(&mut reader)?;
        let source = reader.u32()?;
        if source as usize >= sources.len() {
            return Err(invalid_data("dexsym record references an invalid source"));
        }
        let availability = SymbolAvailability::new(reader.u16()?, reader.u16()?)?;
        let block = reader.u32()?;
        let offset = reader.u32()?;
        let access_flags = reader.u32()?;
        let super_raw = Span::read(&mut reader)?;
        let super_class = (super_raw.len != NONE).then_some(super_raw);
        let interface_start = reader.u32()?;
        let interface_count = reader.u32()?;
        let fields = reader.u32()?;
        let methods = reader.u32()?;
        records.push(ArchiveRecord {
            descriptor,
            source,
            availability,
            block,
            offset,
            access_flags,
            super_class,
            interface_start,
            interface_count,
            fields,
            methods,
        });
    }
    let interface_len = reader.count("interface")?;
    let mut interfaces = Vec::with_capacity(interface_len);
    for _ in 0..interface_len {
        interfaces.push(Span::read(&mut reader)?);
    }
    let block_count = reader.count("block")?;
    let mut block_lens = Vec::with_capacity(block_count);
    for _ in 0..block_count {
        block_lens.push(reader.u32()?);
    }
    reader.finish()?;
    for record in &records {
        if record.block as usize >= block_lens.len() {
            return Err(invalid_data("dexsym record references an invalid block"));
        }
        if record.interface_start as usize + record.interface_count as usize > interfaces.len() {
            return Err(invalid_data("dexsym record interfaces exceed the table"));
        }
    }
    Ok(ArchiveIndex {
        default_target,
        sources,
        raw,
        strings,
        records,
        interfaces,
        block_lens,
    })
}

fn write_index(
    default_target: PlatformTarget,
    sources: &[SymbolSource],
    strings: &[u8],
    records: &[ArchiveRecord],
    interfaces: &[Span],
    block_lens: &[u32],
) -> io::Result<Vec<u8>> {
    let mut writer = BinaryWriter::default();
    writer.u8(default_target.family.code());
    writer.u16(default_target.version);
    writer.count(sources.len())?;
    for source in sources {
        writer.count(source.name.len())?;
        writer.bytes(source.name.as_bytes());
        writer.u8(source.family.code());
        writer.i16(source.priority);
    }
    writer.count(strings.len())?;
    writer.bytes(strings);
    writer.count(records.len())?;
    for record in records {
        record.descriptor.write(&mut writer);
        writer.u32(record.source);
        writer.u16(record.availability.since);
        writer.u16(record.availability.until);
        writer.u32(record.block);
        writer.u32(record.offset);
        writer.u32(record.access_flags);
        match record.super_class {
            Some(span) => span.write(&mut writer),
            None => {
                writer.u32(0);
                writer.u32(NONE);
            }
        }
        writer.u32(record.interface_start);
        writer.u32(record.interface_count);
        writer.u32(record.fields);
        writer.u32(record.methods);
    }
    writer.count(interfaces.len())?;
    for span in interfaces {
        span.write(&mut writer);
    }
    writer.count(block_lens.len())?;
    for len in block_lens {
        writer.u32(*len);
    }
    Ok(writer.finish())
}

/// Decodes every record into the eager representation used by tooling and tests.
pub(super) fn decode_all(bytes: &[u8]) -> io::Result<PlatformSymbolDatabase> {
    let archive = SymbolArchive::open(Cow::Owned(bytes.to_vec()))?;
    let mut classes: BTreeMap<String, Vec<PlatformClass>> = BTreeMap::new();
    for record in 0..archive.index.records.len() as u32 {
        let class = archive.decode_record(record)?;
        classes
            .entry(class.descriptor.clone())
            .or_default()
            .push(class);
    }
    Ok(PlatformSymbolDatabase {
        default_target: archive.index.default_target,
        sources: archive.index.sources.clone(),
        classes,
    })
}

/// Encodes one database into the class-granular archive format.
pub(super) fn encode(database: &PlatformSymbolDatabase) -> io::Result<Vec<u8>> {
    let mut ordered: Vec<&PlatformClass> = database.classes.values().flatten().collect();
    ordered.sort_by(|left, right| {
        left.descriptor
            .cmp(&right.descriptor)
            .then(left.source.cmp(&right.source))
            .then(left.availability.since.cmp(&right.availability.since))
            .then(left.availability.until.cmp(&right.availability.until))
    });

    let mut stored_blocks: Vec<Vec<u8>> = Vec::new();
    let mut block_lens: Vec<u32> = Vec::new();
    let mut records: Vec<ArchiveRecord> = Vec::with_capacity(ordered.len());
    let mut interfaces: Vec<Span> = Vec::new();
    let mut strings = StringBlob::default();

    for chunk in ordered.chunks(BLOCK_CLASSES) {
        let block = u32::try_from(stored_blocks.len())
            .map_err(|_| invalid_data("too many dexsym blocks"))?;
        let pool = StringPool::build_for(chunk.iter().copied())?;
        let mut payload = BinaryWriter::default();
        pool.write(&mut payload)?;
        payload.count(chunk.len())?;
        let mut offsets = Vec::with_capacity(chunk.len());
        for class in chunk {
            offsets.push(payload.len());
            DexSymbolsCodec::write_class(&mut payload, &pool, class)?;
        }
        let raw = payload.finish();
        let stored = zstd::stream::encode_all(Cursor::new(&raw), 8)?;
        block_lens.push(
            u32::try_from(stored.len()).map_err(|_| invalid_data("dexsym block is too large"))?,
        );
        stored_blocks.push(stored);

        for (class, offset) in chunk.iter().zip(offsets) {
            let descriptor = strings.span(&class.descriptor);
            let super_class = class
                .super_class
                .as_deref()
                .map(|value| strings.span(value));
            let interface_start = interfaces.len() as u32;
            for interface in &class.interfaces {
                interfaces.push(strings.span(interface));
            }
            records.push(ArchiveRecord {
                descriptor,
                source: class.source,
                availability: class.availability,
                block,
                offset,
                access_flags: class.access_flags,
                super_class,
                interface_start,
                interface_count: class.interfaces.len() as u32,
                fields: class.fields.len() as u32,
                methods: class.methods.len() as u32,
            });
        }
    }

    let index_raw = write_index(
        database.default_target,
        &database.sources,
        &strings.bytes,
        &records,
        &interfaces,
        &block_lens,
    )?;
    let index_stored = zstd::stream::encode_all(Cursor::new(&index_raw), 8)?;
    let blocks = stored_blocks.concat();

    let mut output = BinaryWriter::default();
    output.bytes(DexSymbolsCodec::MAGIC);
    output.u16(DexSymbolsCodec::MAJOR);
    output.u16(DexSymbolsCodec::MINOR);
    output.u32(DexSymbolsCodec::FLAG_ZSTD);
    output.u64(index_stored.len() as u64);
    output.u64(index_raw.len() as u64);
    output.u32(crc32fast::hash(&index_raw));
    output.u32(crc32fast::hash(&blocks));
    output.u64(blocks.len() as u64);
    output.bytes(&index_stored);
    output.bytes(&blocks);
    Ok(output.finish())
}

#[derive(Default)]
struct StringBlob {
    bytes: Vec<u8>,
    spans: BTreeMap<String, Span>,
}

impl StringBlob {
    fn span(&mut self, value: &str) -> Span {
        if let Some(span) = self.spans.get(value) {
            return *span;
        }
        let span = Span {
            offset: self.bytes.len() as u32,
            len: value.len() as u32,
        };
        self.bytes.extend_from_slice(value.as_bytes());
        self.spans.insert(value.to_string(), span);
        span
    }
}
