//! Original resources.arsc facts and integer-ID candidates. No runtime resource
//! selection, framework table, external declarations or data-flow reconstruction.

use super::{
    android_strings::StringPool,
    archive::read_entry,
    filter::{FilterMatch, TextFilter, best_first},
    input::{EntryIndex, InputError},
    query::QueryError,
    request::RequestScope,
    xref::Issue,
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize)]
#[serde(transparent)]
pub struct ResourceId(pub u32);
#[derive(Clone, Debug)]
pub struct ResourceRow {
    pub(crate) entry: EntryIndex,
    pub(crate) id: ResourceId,
    pub(crate) name: String,
    pub(crate) config: String,
    pub(crate) value: String,
    pub(crate) range_bytes: [usize; 2],
}
#[derive(Default)]
pub struct Catalog {
    pub(crate) rows: Vec<ResourceRow>,
    pub(crate) issues: Vec<Issue>,
}
#[derive(Clone, Debug)]
pub struct ResourceTarget {
    pub(crate) entries: HashMap<ResourceId, (String, FilterMatch)>,
}

pub fn numeric_id(pattern: &str) -> Option<ResourceId> {
    pattern
        .strip_prefix("0x")
        .or_else(|| pattern.strip_prefix("0X"))
        .and_then(|value| u32::from_str_radix(value, 16).ok())
        .or_else(|| pattern.parse::<u32>().ok())
        .map(ResourceId)
}
impl ResourceTarget {
    pub fn unresolved(id: ResourceId) -> Result<Self, QueryError> {
        let mut entries = HashMap::new();
        entries.try_reserve(1).map_err(InputError::Allocation)?;
        entries.insert(
            id,
            ("[unknown resource name]".to_owned(), FilterMatch::default()),
        );
        Ok(Self { entries })
    }
}

fn malformed(at: usize, reason: &str) -> QueryError {
    QueryError::Local {
        kind: diana_decomp::FailureKind::Malformed,
        reason: format!("resources.arsc byte {at:#x}: {reason}"),
    }
}
fn unsupported(at: usize, reason: &str) -> QueryError {
    QueryError::Local {
        kind: diana_decomp::FailureKind::Unsupported,
        reason: format!("resources.arsc byte {at:#x}: {reason}"),
    }
}
#[derive(Clone, Copy)]
struct View<'a> {
    bytes: &'a [u8],
    base: usize,
}
impl<'a> View<'a> {
    fn slice(self, start: usize, size: usize) -> Result<Self, QueryError> {
        let end = start
            .checked_add(size)
            .ok_or_else(|| malformed(self.base, "range overflow"))?;
        Ok(Self {
            bytes: self
                .bytes
                .get(start..end)
                .ok_or_else(|| malformed(self.base, "truncated range"))?,
            base: self
                .base
                .checked_add(start)
                .ok_or(InputError::PlatformCapacity)?,
        })
    }
    fn u8(self, at: usize) -> Result<u8, QueryError> {
        self.bytes
            .get(at)
            .copied()
            .ok_or_else(|| malformed(self.base, "truncated byte"))
    }
    fn u16(self, at: usize) -> Result<u16, QueryError> {
        let v = self.slice(at, 2)?.bytes;
        Ok(u16::from_le_bytes([v[0], v[1]]))
    }
    fn u32(self, at: usize) -> Result<u32, QueryError> {
        let v = self.slice(at, 4)?.bytes;
        Ok(u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
    }
    fn size(self, at: usize) -> Result<usize, QueryError> {
        usize::try_from(self.u32(at)?).map_err(|_| InputError::PlatformCapacity.into())
    }
    fn chunk(self, at: usize) -> Result<(u16, usize, Self), QueryError> {
        let header = self.slice(at, 8)?;
        let kind = header.u16(0)?;
        let size = header.size(4)?;
        let head = usize::from(header.u16(2)?);
        if head < 8 || size < head {
            return Err(malformed(header.base, "inconsistent chunk sizes"));
        }
        Ok((kind, head, self.slice(at, size)?))
    }
}
fn copy_text(value: &str) -> Result<String, QueryError> {
    let mut result = String::new();
    result
        .try_reserve_exact(value.len())
        .map_err(InputError::Allocation)?;
    result.push_str(value);
    Ok(result)
}
fn add(out: &mut String, value: &str) -> Result<(), QueryError> {
    out.try_reserve(value.len())
        .map_err(InputError::Allocation)?;
    out.push_str(value);
    Ok(())
}
fn pool(chunk: View<'_>, at: usize) -> Result<StringPool<'_>, QueryError> {
    let (kind, head, data) = chunk.chunk(at)?;
    if kind != 1 {
        return Err(malformed(data.base, "expected string pool"));
    }
    StringPool::parse(data.bytes, 0, head, data.bytes.len()).map_err(|error| QueryError::Local {
        kind: error.kind(),
        reason: error.to_string(),
    })
}
fn optional_text(pool: Option<&StringPool<'_>>, index: u32) -> Result<String, QueryError> {
    let pool = pool.ok_or_else(|| malformed(0, "required string pool unavailable"))?;
    text(pool, index)
}
fn text(pool: &StringPool<'_>, index: u32) -> Result<String, QueryError> {
    pool.text(index).map_err(|error| QueryError::Local {
        kind: error.kind(),
        reason: error.to_string(),
    })
}
impl Catalog {
    fn issue(
        &mut self,
        entry: EntryIndex,
        error: &QueryError,
        range: Option<[usize; 2]>,
    ) -> Result<(), QueryError> {
        self.issues.try_reserve(1).map_err(InputError::Allocation)?;
        self.issues.push(Issue::new(
            format!("e{}:resources.arsc", entry.0),
            error,
            range,
        ));
        Ok(())
    }
    pub fn read(scope: &RequestScope) -> Result<Self, QueryError> {
        let archive = scope
            .input()
            .archive()
            .ok_or_else(|| unsupported(0, "resource queries require an APK/ZIP"))?;
        let mut result = Self::default();
        let mut found = false;
        for entry in archive
            .entries()
            .iter()
            .filter(|entry| entry.name() == "resources.arsc")
        {
            found = true;
            let read = read_entry(scope.input().bytes(), entry)
                .map_err(InputError::MalformedArchive)
                .map_err(QueryError::from)
                .and_then(|payload| result.table(entry.index(), payload.bytes()));
            if let Err(error) = read {
                result.issue(entry.index(), &error, None)?;
            }
        }
        if !found {
            return Err(unsupported(0, "resources.arsc is absent"));
        }
        Ok(result)
    }
    fn table(&mut self, entry: EntryIndex, bytes: &[u8]) -> Result<(), QueryError> {
        let (kind, head, root) = View { bytes, base: 0 }.chunk(0)?;
        if kind != 2 || head < 12 {
            return Err(malformed(0, "not a resource table"));
        }
        if root.bytes.len() != bytes.len() {
            self.issue(
                entry,
                &malformed(root.bytes.len(), "trailing bytes after resource table"),
                None,
            )?;
        }
        let mut chunks = Vec::new();
        let mut cursor = head;
        while cursor < root.bytes.len() {
            match root.chunk(cursor) {
                Ok((kind, header, chunk)) => {
                    chunks.try_reserve(1).map_err(InputError::Allocation)?;
                    chunks.push((kind, header, chunk));
                    cursor = cursor
                        .checked_add(chunk.bytes.len())
                        .ok_or(InputError::PlatformCapacity)?;
                }
                Err(error) => {
                    self.issue(entry, &error, Some([cursor, root.bytes.len()]))?;
                    break;
                }
            }
        }
        let mut strings = None;
        let mut pool_seen = false;
        for (kind, header, chunk) in &chunks {
            if *kind == 1 {
                if pool_seen {
                    self.issue(
                        entry,
                        &malformed(chunk.base, "duplicate global string pool"),
                        None,
                    )?;
                    continue;
                }
                pool_seen = true;
                match StringPool::parse(chunk.bytes, 0, *header, chunk.bytes.len()) {
                    Ok(pool) => strings = Some(pool),
                    Err(error) => self.issue(
                        entry,
                        &QueryError::Local {
                            kind: error.kind(),
                            reason: error.to_string(),
                        },
                        Some([chunk.base, chunk.base + chunk.bytes.len()]),
                    )?,
                }
            }
        }
        if !pool_seen {
            self.issue(entry, &malformed(0, "global string pool absent"), None)?;
        }
        let mut packages = 0u32;
        for (kind, header, chunk) in chunks {
            match kind {
                0x200 => {
                    packages = packages
                        .checked_add(1)
                        .ok_or(InputError::PlatformCapacity)?;
                    if let Err(error) = self.package(entry, chunk, header, strings.as_ref()) {
                        self.issue(
                            entry,
                            &error,
                            Some([chunk.base, chunk.base + chunk.bytes.len()]),
                        )?;
                    }
                }
                1 => (),
                _ => self.issue(
                    entry,
                    &unsupported(chunk.base, "table chunk not interpreted"),
                    Some([chunk.base, chunk.base + chunk.bytes.len()]),
                )?,
            }
        }
        if packages != root.u32(8)? {
            self.issue(
                entry,
                &malformed(0, "package count disagrees with located packages"),
                None,
            )?;
        }
        Ok(())
    }
    fn package(
        &mut self,
        entry: EntryIndex,
        package: View<'_>,
        header: usize,
        strings: Option<&StringPool<'_>>,
    ) -> Result<(), QueryError> {
        if header < 284 {
            return Err(malformed(package.base, "package header too small"));
        }
        let id = package.u32(8)?;
        if id > 255 {
            return Err(malformed(
                package.base,
                "package ID does not fit a resource ID",
            ));
        }
        let type_offset = if header >= 288 { package.u32(284)? } else { 0 };
        let types = match pool(package, package.size(268)?) {
            Ok(pool) => Some(pool),
            Err(error) => {
                self.issue(entry, &error, None)?;
                None
            }
        };
        let keys = match pool(package, package.size(276)?) {
            Ok(pool) => Some(pool),
            Err(error) => {
                self.issue(entry, &error, None)?;
                None
            }
        };
        let mut cursor = header;
        while cursor < package.bytes.len() {
            let (kind, head, chunk) = package.chunk(cursor)?;
            match kind {
                0x201 => {
                    if let Err(error) = self.type_chunk(
                        entry,
                        chunk,
                        head,
                        id,
                        type_offset,
                        types.as_ref(),
                        keys.as_ref(),
                        strings,
                    ) {
                        self.issue(
                            entry,
                            &error,
                            Some([chunk.base, chunk.base + chunk.bytes.len()]),
                        )?;
                    }
                }
                1 | 0x202 => (),
                _ => self.issue(
                    entry,
                    &unsupported(
                        chunk.base,
                        "package metadata chunk not interpreted; IDs are not runtime-linked",
                    ),
                    Some([chunk.base, chunk.base + chunk.bytes.len()]),
                )?,
            }
            cursor = cursor
                .checked_add(chunk.bytes.len())
                .ok_or(InputError::PlatformCapacity)?;
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    fn type_chunk(
        &mut self,
        entry: EntryIndex,
        chunk: View<'_>,
        header: usize,
        package_id: u32,
        type_offset: u32,
        types: Option<&StringPool<'_>>,
        keys: Option<&StringPool<'_>>,
        strings: Option<&StringPool<'_>>,
    ) -> Result<(), QueryError> {
        if header < 24 {
            return Err(malformed(chunk.base, "type header too small"));
        }
        let raw_type = u32::from(chunk.u8(8)?);
        let type_id = raw_type
            .checked_add(type_offset)
            .ok_or(InputError::PlatformCapacity)?;
        if raw_type == 0 || type_id > 255 {
            return Err(malformed(chunk.base, "invalid type ID"));
        }
        let flags = chunk.u8(9)?;
        if flags & !3 != 0 || flags & 3 == 3 {
            return Err(unsupported(chunk.base, "unknown entry-offset encoding"));
        }
        let count = chunk.size(12)?;
        let start = chunk.size(16)?;
        let config_size = chunk.size(20)?;
        if config_size < 4 || config_size.checked_add(20).is_none_or(|end| end > header) {
            return Err(malformed(chunk.base, "configuration range exceeds header"));
        }
        let config_bytes = chunk.slice(20, config_size)?.bytes;
        let config = if config_bytes[4..].iter().all(|&byte| byte == 0) {
            "default".to_owned()
        } else {
            let mut value = String::new();
            add(&mut value, "config:")?;
            for byte in config_bytes {
                add(&mut value, &format!("{byte:02x}"))?;
            }
            value
        };
        let stride = if flags & 2 != 0 { 2usize } else { 4usize };
        let table_size = count
            .checked_mul(stride)
            .ok_or(InputError::PlatformCapacity)?;
        let offsets = chunk.slice(header, table_size)?;
        if header.checked_add(table_size).is_none_or(|end| end > start) || start > chunk.bytes.len()
        {
            return Err(malformed(chunk.base, "entry data overlaps offset table"));
        }
        let type_name = match optional_text(types, raw_type - 1) {
            Ok(value) => value,
            Err(error) => {
                self.issue(entry, &error, None)?;
                format!("[type-id:{type_id} unavailable]")
            }
        };
        let mut previous = None;
        for ordinal in 0..count {
            let offset_at = ordinal
                .checked_mul(stride)
                .ok_or(InputError::PlatformCapacity)?;
            let (index, relative) = if flags & 1 != 0 {
                let record = offsets.slice(offset_at, 4)?;
                let index = u32::from(record.u16(0)?);
                if previous.is_some_and(|prior| prior >= index) {
                    self.issue(
                        entry,
                        &malformed(record.base, "sparse indices not strictly increasing"),
                        None,
                    )?;
                }
                previous = Some(index);
                (
                    index,
                    u32::from(record.u16(2)?)
                        .checked_mul(4)
                        .ok_or(InputError::PlatformCapacity)?,
                )
            } else if flags & 2 != 0 {
                let value = offsets.u16(offset_at)?;
                if value == u16::MAX {
                    continue;
                }
                (
                    u32::try_from(ordinal).map_err(|_| InputError::PlatformCapacity)?,
                    u32::from(value)
                        .checked_mul(4)
                        .ok_or(InputError::PlatformCapacity)?,
                )
            } else {
                let value = offsets.u32(offset_at)?;
                if value == u32::MAX {
                    continue;
                }
                (
                    u32::try_from(ordinal).map_err(|_| InputError::PlatformCapacity)?,
                    value,
                )
            };
            if index > u32::from(u16::MAX) {
                self.issue(
                    entry,
                    &malformed(chunk.base, "entry index exceeds resource ID width"),
                    None,
                )?;
                continue;
            }
            let id = ResourceId((package_id << 24) | (type_id << 16) | index);
            let mut recovered_name = format!("{type_name}/[entry-index:{index} unavailable]");
            let resource = (|| -> Result<ResourceRow, QueryError> {
                if relative & 3 != 0 {
                    return Err(malformed(chunk.base, "unaligned entry offset"));
                }
                let at = start
                    .checked_add(
                        usize::try_from(relative).map_err(|_| InputError::PlatformCapacity)?,
                    )
                    .ok_or(InputError::PlatformCapacity)?;
                let record = chunk.slice(
                    at,
                    chunk
                        .bytes
                        .len()
                        .checked_sub(at)
                        .ok_or_else(|| malformed(chunk.base, "entry outside chunk"))?,
                )?;
                let flags = record.u16(2)?;
                let compact = flags & 8 != 0;
                let key = if compact {
                    u32::from(record.u16(0)?)
                } else {
                    record.u32(4)?
                };
                recovered_name = match optional_text(keys, key) {
                    Ok(name) => format!("{type_name}/{name}"),
                    Err(error) => {
                        self.issue(entry, &error, None)?;
                        format!("{type_name}/[key:{key} unavailable]")
                    }
                };
                let (value, length) = if compact {
                    if flags & 1 != 0 {
                        return Err(malformed(record.base, "compact entry cannot be complex"));
                    }
                    (
                        self.typed_value(
                            entry,
                            (flags >> 8) as u8,
                            record.u32(4)?,
                            strings,
                            record.base,
                        )?,
                        8,
                    )
                } else {
                    let size = usize::from(record.u16(0)?);
                    if size < 8 {
                        return Err(malformed(record.base, "entry header too small"));
                    }
                    record.slice(0, size)?;
                    if flags & 1 == 0 {
                        let value = record.slice(size, 8)?;
                        (
                            self.value(entry, value, strings)?,
                            size.checked_add(8).ok_or(InputError::PlatformCapacity)?,
                        )
                    } else {
                        if size < 16 {
                            return Err(malformed(record.base, "map entry header too small"));
                        }
                        let declared_count = record.size(12)?;
                        let available_count = record
                            .bytes
                            .len()
                            .checked_sub(size)
                            .ok_or_else(|| malformed(record.base, "map data outside entry"))?
                            / 12;
                        let count = declared_count.min(available_count);
                        let length = count
                            .checked_mul(12)
                            .and_then(|n| size.checked_add(n))
                            .ok_or(InputError::PlatformCapacity)?;
                        if count != declared_count {
                            self.issue(
                                entry,
                                &malformed(record.base, "map suffix unavailable"),
                                Some([record.base, record.base + record.bytes.len()]),
                            )?;
                        }
                        let mut value = format!("bag parent=@{:#010x} {{", record.u32(8)?);
                        for ordinal in 0..count {
                            let item = record.slice(
                                size.checked_add(
                                    ordinal
                                        .checked_mul(12)
                                        .ok_or(InputError::PlatformCapacity)?,
                                )
                                .ok_or(InputError::PlatformCapacity)?,
                                12,
                            )?;
                            let rendered = self.value(entry, item.slice(4, 8)?, strings)?;
                            add(&mut value, &format!(" {}:{rendered};", item.u32(0)?))?;
                        }
                        if count != declared_count {
                            add(
                                &mut value,
                                &format!(" [unavailable suffix; declared-count:{declared_count}]"),
                            )?;
                        }
                        add(&mut value, " }")?;
                        (value, length)
                    }
                };
                record.slice(0, length)?;
                Ok(ResourceRow {
                    entry,
                    id,
                    name: copy_text(&recovered_name)?,
                    config: copy_text(&config)?,
                    value,
                    range_bytes: [
                        record.base,
                        record
                            .base
                            .checked_add(length)
                            .ok_or(InputError::PlatformCapacity)?,
                    ],
                })
            })();
            match resource {
                Ok(row) => {
                    self.rows.try_reserve(1).map_err(InputError::Allocation)?;
                    self.rows.push(row);
                }
                Err(error) => {
                    self.issue(entry, &error, None)?;
                    let at = offsets
                        .base
                        .checked_add(offset_at)
                        .ok_or(InputError::PlatformCapacity)?;
                    self.rows.try_reserve(1).map_err(InputError::Allocation)?;
                    self.rows.push(ResourceRow {
                        entry,
                        id,
                        name: recovered_name,
                        config: copy_text(&config)?,
                        value: format!("[unavailable: {error}]"),
                        range_bytes: [
                            at,
                            at.checked_add(stride).ok_or(InputError::PlatformCapacity)?,
                        ],
                    });
                }
            }
        }
        Ok(())
    }
    fn value(
        &mut self,
        entry: EntryIndex,
        value: View<'_>,
        strings: Option<&StringPool<'_>>,
    ) -> Result<String, QueryError> {
        if value.u16(0)? != 8 || value.u8(2)? != 0 {
            self.issue(
                entry,
                &malformed(value.base, "unsupported Res_value size/reserved word"),
                Some([
                    value.base,
                    value
                        .base
                        .checked_add(value.bytes.len())
                        .ok_or(InputError::PlatformCapacity)?,
                ]),
            )?;
            return Ok(format!(
                "[unavailable type:{:#04x} data:{:#010x}]",
                value.u8(3)?,
                value.u32(4)?
            ));
        }
        self.typed_value(entry, value.u8(3)?, value.u32(4)?, strings, value.base)
    }
    fn typed_value(
        &mut self,
        entry: EntryIndex,
        kind: u8,
        data: u32,
        strings: Option<&StringPool<'_>>,
        at: usize,
    ) -> Result<String, QueryError> {
        Ok(match kind {
            0 => format!("null:{data:#010x}"),
            1 => format!("@{data:#010x}"),
            2 => format!("?{data:#010x}"),
            3 => match optional_text(strings, data) {
                Ok(value) => value,
                Err(error) => {
                    self.issue(entry, &error, Some([at, at.saturating_add(8)]))?;
                    format!("[unavailable string-index:{data}]")
                }
            },
            4 => format!("float:{} bits:{data:#010x}", f32::from_bits(data)),
            5 => format!("dimension-bits:{data:#010x}"),
            6 => format!("fraction-bits:{data:#010x}"),
            7 => format!("dynamic-reference:@{data:#010x}"),
            8 => format!("dynamic-attribute:?{data:#010x}"),
            0x10 => i32::from_le_bytes(data.to_le_bytes()).to_string(),
            0x11 => format!("{data:#010x}"),
            0x12 => format!("{} bits:{data:#010x}", data != 0),
            0x1c..=0x1f => format!("color-type:{kind:#04x} #{data:08x}"),
            _ => {
                self.issue(
                    entry,
                    &unsupported(at, "typed value not interpreted"),
                    Some([at, at.saturating_add(8)]),
                )?;
                format!("[type:{kind:#04x} data:{data:#010x}]")
            }
        })
    }
    pub fn select(
        &self,
        filter: Option<&TextFilter>,
    ) -> Result<Vec<(&ResourceRow, FilterMatch)>, QueryError> {
        let mut rows = Vec::new();
        for row in &self.rows {
            let selected = match filter {
                None => Some(FilterMatch::default()),
                Some(filter) => {
                    let mut selected = filter.evaluate_text(&format!("{:#010x}", row.id.0))?;
                    for field in [&row.name, &row.value] {
                        if let Some(next) = filter.evaluate_text(field)?
                            && (selected.is_none()
                                || best_first(
                                    next.relevance,
                                    selected.and_then(|value| value.relevance),
                                )
                                .is_lt())
                        {
                            selected = Some(next);
                        }
                    }
                    selected
                }
            };
            if let Some(selected) = selected {
                rows.try_reserve(1).map_err(InputError::Allocation)?;
                rows.push((row, selected));
            }
        }
        if filter.is_some_and(TextFilter::is_fuzzy) {
            rows.sort_unstable_by(|(left, a), (right, b)| {
                best_first(a.relevance, b.relevance)
                    .then_with(|| left.entry.cmp(&right.entry))
                    .then_with(|| left.id.0.cmp(&right.id.0))
                    .then_with(|| left.range_bytes.cmp(&right.range_bytes))
            });
        }
        Ok(rows)
    }
    pub fn target(
        &self,
        pattern: &str,
        mode: super::filter::MatchMode,
    ) -> Result<ResourceTarget, QueryError> {
        let numeric = numeric_id(pattern);
        let filter = TextFilter::compile(pattern, mode).map_err(|error| QueryError::Local {
            kind: error.kind(),
            reason: error.to_string(),
        })?;
        let selected = if let Some(id) = numeric {
            let mut rows = Vec::new();
            for row in &self.rows {
                if row.id == id {
                    rows.try_reserve(1).map_err(InputError::Allocation)?;
                    rows.push((row, FilterMatch::default()));
                }
            }
            rows
        } else {
            self.select(Some(&filter))?
        };
        let mut entries = HashMap::new();
        for (row, selected) in selected {
            entries.try_reserve(1).map_err(InputError::Allocation)?;
            let value = entries
                .entry(row.id)
                .or_insert((copy_text(&row.name)?, selected));
            if value.0 != row.name && !value.0.split(" | ").any(|name| name == row.name) {
                add(&mut value.0, " | ")?;
                add(&mut value.0, &row.name)?;
            }
            if best_first(selected.relevance, value.1.relevance).is_lt() {
                value.1 = selected;
            }
        }
        if entries.is_empty()
            && let Some(id) = numeric
        {
            return ResourceTarget::unresolved(id);
        }
        Ok(ResourceTarget { entries })
    }
}
