//! TSV and JSON presentation of query rows, formatted inside the scan.
//!
//! Two costs decided this shape. Formatting every row on the consumer thread made
//! the scan wait for one core (`strings` on a 208 MiB multi-DEX APK: 6.7 s for
//! 2.2 M rows), and writing each formatted piece straight to the sink turned one
//! row into a handful of syscalls. So a worker formats its own rows into a byte
//! chunk and only the chunk crosses to the consumer, which writes it once.
//!
//! Key order is written out explicitly instead of coming from `serde_json::json!`,
//! whose `Map` is a `BTreeMap` or an `IndexMap` depending on whether any crate in
//! the build graph enables `preserve_order`. That made the published byte order
//! depend on the build, which no caller can rely on.

use std::io::Write;
use std::path::Path;

use super::output::{Failure, Format, publish_with};
use crate::analysis::at::{self, Mode};
use crate::analysis::query::QueryError;
use diana_decomp::input::DexString;

/// One row in original locator order.
///
/// The locator and the value are described, not formatted: a string or class index
/// is millions of rows, and building two `String`s per row cost more than the scan
/// that produced them. The sink writes them straight into the chunk.
pub(super) struct QueryRow<'a> {
    pub at: Locator<'a>,
    pub value: Option<DexString<'a>>,
}

/// Everything the canonical `at` locator is made of, kept decoded.
pub(super) enum Locator<'a> {
    Class {
        entry: Option<usize>,
        logical: u32,
        class_def: u32,
        descriptor: DexString<'a>,
    },
    Method {
        entry: Option<usize>,
        logical: u32,
        class_def: u32,
        method: u32,
        descriptor: DexString<'a>,
        name: DexString<'a>,
        parameters: &'a [DexString<'a>],
        returns: DexString<'a>,
    },
    Field {
        entry: Option<usize>,
        logical: u32,
        class_def: u32,
        field: u32,
        descriptor: DexString<'a>,
        name: DexString<'a>,
        field_type: DexString<'a>,
    },
    String {
        entry: Option<usize>,
        logical: u32,
        string: u32,
    },
}

/// Formats one row into a chunk. Shared by workers; only immutable presentation
/// settings are retained, and output appends to the caller-owned buffer.
pub(super) struct RowFormat<'a> {
    format: Format,
    columns: &'a [&'a str],
    /// `"column":` for each column, encoded once. JSON object keys are fixed
    /// identifiers, so they are the same bytes on every row.
    keys: Vec<Vec<u8>>,
}

impl<'a> RowFormat<'a> {
    pub(super) fn new(format: Format, columns: &'a [&'a str]) -> Self {
        let keys = columns
            .iter()
            .map(|column| {
                let mut key = Vec::with_capacity(column.len() + 4);
                key.push(b'"');
                key.extend_from_slice(column.as_bytes());
                key.extend_from_slice(b"\":");
                key
            })
            .collect();
        Self {
            format,
            columns,
            keys,
        }
    }

    pub(super) fn query_row(
        &self,
        row: &QueryRow<'_>,
        out: &mut Vec<u8>,
        first: bool,
    ) -> Result<(), QueryError> {
        let mode = mode(self.format);
        match self.format {
            Format::Text => {
                for (index, column) in self.columns.iter().enumerate() {
                    if index > 0 {
                        out.push(b'\t');
                    }
                    write_column(out, column, row, mode)?;
                }
                out.push(b'\n');
            }
            Format::Json => {
                if !first {
                    out.push(b',');
                }
                out.push(b'{');
                for (index, (key, column)) in self.keys.iter().zip(self.columns).enumerate() {
                    if index > 0 {
                        out.push(b',');
                    }
                    out.extend_from_slice(key);
                    write_column(out, column, row, mode)?;
                }
                out.push(b'}');
            }
        }
        Ok(())
    }

    pub(super) fn entry_row(
        &self,
        name: &str,
        compressed: u64,
        uncompressed: u64,
        method: u64,
        out: &mut Vec<u8>,
        first: bool,
    ) {
        let cells = [
            Cell::Text(name),
            Cell::Number(compressed),
            Cell::Number(uncompressed),
            Cell::Number(method),
        ];
        self.render(cells.into_iter().take(self.columns.len()), out, first);
    }

    fn render<'c>(&self, cells: impl Iterator<Item = Cell<'c>>, out: &mut Vec<u8>, first: bool) {
        match self.format {
            Format::Text => {
                let mut run = Vec::new();
                for (index, cell) in cells.enumerate() {
                    if index > 0 {
                        out.push(b'\t');
                    }
                    match cell {
                        Cell::Text(value) => escape_into(value, &mut run, out),
                        Cell::Number(value) => push_decimal(out, value),
                    }
                }
                out.push(b'\n');
            }
            Format::Json => {
                if !first {
                    out.push(b',');
                }
                out.push(b'{');
                for (index, (key, cell)) in self.keys.iter().zip(cells).enumerate() {
                    if index > 0 {
                        out.push(b',');
                    }
                    out.extend_from_slice(key);
                    match cell {
                        Cell::Text(value) => write_json_string(out, value),
                        Cell::Number(value) => push_decimal(out, value),
                    }
                }
                out.push(b'}');
            }
        }
    }
}

enum Cell<'a> {
    Text(&'a str),
    Number(u64),
}

fn mode(format: Format) -> Mode {
    match format {
        Format::Text => Mode::Text,
        Format::Json => Mode::Json,
    }
}

/// Writes one column's content without separators. Every query column is text, so
/// JSON mode wraps the escaped value in quotes; the numbers of `entries` are
/// written by the other row path, which decides that per cell.
fn write_column(
    out: &mut Vec<u8>,
    column: &str,
    row: &QueryRow<'_>,
    mode: Mode,
) -> Result<(), QueryError> {
    let quoted = mode == Mode::Json;
    if quoted {
        out.push(b'"');
    }
    match column {
        "at" => write_locator(out, &row.at, mode),
        "value" => match row.value {
            Some(value) => Ok(at::write_value(out, mode, value)?),
            None => Ok(()),
        },
        _ => Ok(()),
    }?;
    if quoted {
        out.push(b'"');
    }
    Ok(())
}

/// Writes the canonical locator: physical entry, logical DEX, original index and
/// the verifiable original name.
fn write_locator(out: &mut Vec<u8>, locator: &Locator<'_>, mode: Mode) -> Result<(), QueryError> {
    match locator {
        Locator::Class {
            entry,
            logical,
            class_def,
            descriptor,
        } => {
            at::write_prefix(out, *entry, *logical);
            out.extend_from_slice(b":c");
            at::push_decimal(out, u64::from(*class_def));
            out.extend_from_slice(b"::");
            at::write_value(out, mode, *descriptor)?;
        }
        Locator::Method {
            entry,
            logical,
            class_def,
            method,
            descriptor,
            name,
            parameters,
            returns,
        } => {
            at::write_prefix(out, *entry, *logical);
            out.extend_from_slice(b":c");
            at::push_decimal(out, u64::from(*class_def));
            out.extend_from_slice(b":m");
            at::push_decimal(out, u64::from(*method));
            out.extend_from_slice(b"::");
            at::write_value(out, mode, *descriptor)?;
            out.extend_from_slice(b"->");
            at::write_value(out, mode, *name)?;
            out.push(b'(');
            for parameter in parameters.iter() {
                at::write_value(out, mode, *parameter)?;
            }
            out.push(b')');
            at::write_value(out, mode, *returns)?;
        }
        Locator::Field {
            entry,
            logical,
            class_def,
            field,
            descriptor,
            name,
            field_type,
        } => {
            at::write_prefix(out, *entry, *logical);
            out.extend_from_slice(b":c");
            at::push_decimal(out, u64::from(*class_def));
            out.extend_from_slice(b":f");
            at::push_decimal(out, u64::from(*field));
            out.extend_from_slice(b"::");
            at::write_value(out, mode, *descriptor)?;
            out.extend_from_slice(b"->");
            at::write_value(out, mode, *name)?;
            out.push(b':');
            at::write_value(out, mode, *field_type)?;
        }
        Locator::String {
            entry,
            logical,
            string,
        } => {
            at::write_prefix(out, *entry, *logical);
            out.extend_from_slice(b":s");
            at::push_decimal(out, u64::from(*string));
        }
    }
    Ok(())
}

/// Appends `value` as a JSON string body, byte-identical to what
/// `serde_json::to_writer` emits for the same value: only `"`, `\\`, the five
/// short control escapes and the remaining bytes below `0x20` are rewritten;
/// every other byte, including multi-byte UTF-8, is copied verbatim.
fn write_json_string(out: &mut Vec<u8>, value: &str) {
    out.push(b'"');
    let bytes = value.as_bytes();
    let mut run = 0usize;
    for (index, &byte) in bytes.iter().enumerate() {
        let escape: Option<&[u8]> = match byte {
            b'"' => Some(b"\\\""),
            b'\\' => Some(b"\\\\"),
            0x08 => Some(b"\\b"),
            0x09 => Some(b"\\t"),
            0x0a => Some(b"\\n"),
            0x0c => Some(b"\\f"),
            0x0d => Some(b"\\r"),
            other if other < 0x20 => None,
            _ => continue,
        };
        if index > run {
            out.extend_from_slice(&bytes[run..index]);
        }
        match escape {
            Some(fixed) => out.extend_from_slice(fixed),
            None => {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                out.extend_from_slice(b"\\u00");
                out.push(HEX[usize::from(byte >> 4)]);
                out.push(HEX[usize::from(byte & 0x0f)]);
            }
        }
        run = index + 1;
    }
    if run < bytes.len() {
        out.extend_from_slice(&bytes[run..]);
    }
    out.push(b'"');
}

/// Appends the decimal form of `value` without allocating.
fn push_decimal(out: &mut Vec<u8>, value: u64) {
    let mut digits = [0u8; 20];
    let mut at = digits.len();
    let mut rest = value;
    loop {
        at -= 1;
        digits[at] = b'0' + (rest % 10) as u8;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[at..]);
}

/// Appends `value` with display escapes. `scratch` is reused so a row does not
/// allocate per escaped value.
fn escape_into(value: &str, scratch: &mut Vec<u8>, out: &mut Vec<u8>) {
    scratch.clear();
    let mut run = 0usize;
    for (index, character) in value.char_indices() {
        let replacement: Option<&[u8]> = match character {
            '\\' => Some(b"\\\\"),
            '\t' => Some(b"\\t"),
            '\r' => Some(b"\\r"),
            '\n' => Some(b"\\n"),
            '\0' => Some(b"\\0"),
            _ => None,
        };
        let Some(replacement) = replacement else {
            continue;
        };
        scratch.extend_from_slice(&value.as_bytes()[run..index]);
        scratch.extend_from_slice(replacement);
        run = index + character.len_utf8();
    }
    scratch.extend_from_slice(&value.as_bytes()[run..]);
    out.extend_from_slice(scratch);
}

/// Streams a query result into the requested sink, one formatted chunk per root.
///
/// `key` is the JSON array name (`items` or `entries`). A sink failure stops the
/// scan through [`QueryError::WorkerLost`] and is reported as the write error,
/// never as a short successful result. An output file stays atomic: it appears
/// only after the epilogue was written and flushed.
pub(super) fn publish_table<F>(
    command: &str,
    key: &str,
    columns: &[&str],
    format: Format,
    input: &Path,
    output: Option<&Path>,
    run: F,
) -> Result<(), Failure>
where
    F: FnOnce(&mut ChunkSink<'_>) -> Result<(), Failure>,
{
    publish_with(output, input, |writer| {
        let mut buffered = std::io::BufWriter::with_capacity(1 << 18, writer);
        let mut sink =
            ChunkSink::new(&mut buffered, key, columns, format).map_err(Failure::output)?;
        sink.begin(command).map_err(Failure::output)?;
        let outcome = run(&mut sink);
        let write_failure = sink.failure.take();
        let finished = sink.finish().map_err(Failure::output);
        let flushed = buffered.flush().map_err(Failure::output);
        if let Some(error) = write_failure {
            return Err(Failure::output(error));
        }
        outcome?;
        finished?;
        flushed
    })
}

pub(super) struct ChunkSink<'w> {
    writer: &'w mut dyn Write,
    format: Format,
    columns: &'w [&'w str],
    items_key: &'w str,
    started: bool,
    /// The first write failure. It is sticky so a closed pipe is reported once,
    /// as the write error, instead of surfacing as a stopped query.
    failure: Option<std::io::Error>,
}

impl<'w> ChunkSink<'w> {
    fn new(
        writer: &'w mut dyn Write,
        items_key: &'w str,
        columns: &'w [&'w str],
        format: Format,
    ) -> std::io::Result<Self> {
        Ok(Self {
            writer,
            format,
            columns,
            items_key,
            started: false,
            failure: None,
        })
    }

    fn begin(&mut self, command: &str) -> std::io::Result<()> {
        match self.format {
            Format::Text => {
                for (index, column) in self.columns.iter().enumerate() {
                    if index > 0 {
                        self.writer.write_all(b"\t")?;
                    }
                    self.writer.write_all(column.as_bytes())?;
                }
                self.writer.write_all(b"\n")
            }
            Format::Json => {
                write!(
                    self.writer,
                    "{{\"schema_version\":{},\"command\":",
                    schema_version(command),
                )?;
                serde_json::to_writer(&mut *self.writer, command)?;
                self.writer.write_all(b",\"")?;
                self.writer.write_all(self.items_key.as_bytes())?;
                self.writer.write_all(b"\":[")
            }
        }
    }

    /// Writes one worker's formatted rows. The error only stops the scan.
    pub(super) fn chunk(&mut self, bytes: &[u8]) -> Result<(), QueryError> {
        let written = self.write_chunk(bytes);
        match written {
            Ok(()) => Ok(()),
            Err(error) => {
                self.failure = Some(error);
                Err(QueryError::WorkerLost)
            }
        }
    }

    fn write_chunk(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        // Rows already carry their own separators inside a chunk, so only the
        // boundary between two chunks needs one.
        if self.format == Format::Json && self.started {
            self.writer.write_all(b",")?;
        }
        self.writer.write_all(bytes)?;
        self.started = true;
        Ok(())
    }

    fn finish(self) -> std::io::Result<()> {
        if self.format == Format::Json {
            self.writer.write_all(b"]}")?;
        }
        Ok(())
    }
}

/// Xref/resource schema 3 distinguishes coverage from display truncation.
/// Strings retain schema 2; other listing tables retain schema 1.
fn schema_version(command: &str) -> u8 {
    match command {
        "xref" | "resources" => 3,
        "strings" => 2,
        _ => 1,
    }
}

/// Coverage of the already-selected display, separate from scan completeness.
pub(super) struct RowSummary<'a> {
    pub total: usize,
    pub warning: Option<&'a str>,
    pub issues: &'a [crate::analysis::xref::Issue],
}

/// Already selected cells; resource values can remain borrowed from the catalog.
pub(super) trait Cells {
    fn cells(&self) -> impl Iterator<Item = &str>;
}
impl Cells for Vec<String> {
    fn cells(&self) -> impl Iterator<Item = &str> {
        self.iter().map(String::as_str)
    }
}
struct ObjectRow<'a, R> {
    columns: &'a [&'a str],
    row: &'a R,
}
impl<R: Cells> serde::Serialize for ObjectRow<'_, R> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.columns.len()))?;
        for (column, value) in self.columns.iter().zip(self.row.cells()) {
            map.serialize_entry(column, value)?;
        }
        map.end()
    }
}
struct Items<'a, R> {
    columns: &'a [&'a str],
    rows: &'a [R],
}
impl<R: Cells> serde::Serialize for Items<'_, R> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut rows = serializer.serialize_seq(Some(self.rows.len()))?;
        for row in self.rows {
            rows.serialize_element(&ObjectRow {
                columns: self.columns,
                row,
            })?;
        }
        rows.end()
    }
}

/// Renders the existing selection directly to the atomic sink. No JSON value tree
/// or second full text buffer copies potentially large resource/metadata values.
pub(super) fn publish_rows<R: Cells>(
    rows: &[R],
    columns: &[&str],
    command: &str,
    format: Format,
    input: &Path,
    output: Option<&Path>,
    summary: RowSummary<'_>,
) -> Result<(), Failure> {
    for issue in summary.issues {
        eprintln!(
            "diana: warning: {}: {}: {}",
            issue.context, issue.kind, issue.reason
        );
    }
    publish_with(output, input, |writer| {
        let mut writer = std::io::BufWriter::with_capacity(1 << 18, writer);
        match format {
            Format::Text => {
                if let Some(warning) = summary.warning {
                    writeln!(writer, "{warning}").map_err(Failure::output)?;
                }
                if !summary.issues.is_empty() {
                    writeln!(
                        writer,
                        "//Diana Warn: coverage incomplete; counts describe recovered matches only"
                    )
                    .map_err(Failure::output)?;
                }
                write_cells(&mut writer, columns.iter().copied()).map_err(Failure::output)?;
                for row in rows {
                    write_cells(&mut writer, row.cells()).map_err(Failure::output)?;
                }
            }
            Format::Json => {
                let write = (|| -> Result<(), serde_json::Error> {
                    use serde::ser::SerializeMap;
                    let mut serializer = serde_json::Serializer::new(&mut writer);
                    let mut object = serde::Serializer::serialize_map(&mut serializer, Some(10))?;
                    object.serialize_entry("schema_version", &schema_version(command))?;
                    object.serialize_entry("command", command)?;
                    object.serialize_entry("items", &Items { columns, rows })?;
                    object.serialize_entry(
                        "total",
                        &summary.issues.is_empty().then_some(summary.total),
                    )?;
                    object.serialize_entry("recovered_total", &summary.total)?;
                    object.serialize_entry("complete", &summary.issues.is_empty())?;
                    object.serialize_entry("issues", summary.issues)?;
                    object.serialize_entry("shown", &rows.len())?;
                    object.serialize_entry("truncated", &(rows.len() < summary.total))?;
                    object.serialize_entry("warnings", summary.warning.as_slice())?;
                    object.end()
                })();
                write.map_err(|error| Failure::output(std::io::Error::other(error)))?;
            }
        }
        writer.flush().map_err(Failure::output)
    })
}
fn write_cells<'a>(
    writer: &mut dyn Write,
    cells: impl Iterator<Item = &'a str>,
) -> std::io::Result<()> {
    for (ordinal, value) in cells.enumerate() {
        if ordinal != 0 {
            writer.write_all(b"\t")?;
        }
        let mut start = 0;
        for (at, character) in value.char_indices() {
            let escaped: &[u8] = match character {
                '\\' => b"\\\\",
                '\t' => b"\\t",
                '\r' => b"\\r",
                '\n' => b"\\n",
                '\0' => b"\\0",
                _ => continue,
            };
            writer.write_all(&value.as_bytes()[start..at])?;
            writer.write_all(escaped)?;
            start = at + character.len_utf8();
        }
        writer.write_all(&value.as_bytes()[start..])?;
    }
    writer.write_all(b"\n")
}
