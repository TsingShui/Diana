use std::collections::HashMap;
use std::io::Write;
use std::sync::mpsc::{SyncSender, sync_channel};

use super::{ContextKey, Matched, RowView, Rows, context_key, matched_text, render_context};
use crate::analysis::{input::InputError, query::ImageInfo};
use crate::cli::output::{Failure, Format};
use crate::cli::table::encode_cell;

/// Each worker owns a contiguous part of the selected order and one bounded
/// channel. Draining those channels in order preserves both ranking and original
/// arrival order without a reorder buffer. Dropping receivers stops blocked sends.
pub(super) fn write(
    rows: &Rows,
    shown: usize,
    images: &[ImageInfo],
    jobs: usize,
    format: Format,
    writer: &mut dyn Write,
) -> Result<(), Failure> {
    if shown == 0 {
        return Ok(());
    }
    #[cfg(feature = "perf")]
    let render_span = fastrace::Span::enter_with_local_parent("xref.encode");
    let workers = jobs.max(1).min(shown);
    let width = shown.div_ceil(workers);
    std::thread::scope(|scope| {
        let mut receivers = Vec::new();
        receivers
            .try_reserve_exact(workers)
            .map_err(|error| Failure::input(InputError::Allocation(error)))?;
        for start in (0..shown).step_by(width) {
            let end = start.saturating_add(width).min(shown);
            let (sender, receiver) = sync_channel(1);
            #[cfg(feature = "perf")]
            let parent = &render_span;
            std::thread::Builder::new()
                .spawn_scoped(scope, move || {
                    #[cfg(feature = "perf")]
                    let span = fastrace::Span::enter_with_parent("xref.encode-worker", parent);
                    #[cfg(feature = "perf")]
                    let _guard = span.set_local_parent();
                    let result = encode(rows, start..end, images, format, &sender);
                    if let Err(error) = result {
                        let _ = sender.send(Err(error));
                    }
                })
                .map_err(Failure::output)?;
            receivers.push(receiver);
        }
        for receiver in receivers {
            for chunk in receiver {
                writer.write_all(&chunk?).map_err(Failure::output)?;
            }
        }
        Ok(())
    })
}

fn encode(
    rows: &Rows,
    range: std::ops::Range<usize>,
    images: &[ImageInfo],
    format: Format,
    sender: &SyncSender<Result<Vec<u8>, Failure>>,
) -> Result<(), Failure> {
    let mut encoder = Encoder {
        format,
        contexts: HashMap::new(),
        matches: HashMap::new(),
        uses: HashMap::new(),
    };
    let mut chunk = Vec::new();
    let first = range.start;
    for (index, row) in rows.range(range).enumerate() {
        encoder.row(row, images, &mut chunk, first == 0 && index == 0)?;
    }
    if !chunk.is_empty() {
        sender.send(Ok(chunk)).map_err(|_| stopped())?;
    }
    Ok(())
}

struct Encoder<'a> {
    format: Format,
    contexts: HashMap<ContextKey, Vec<u8>>,
    matches: HashMap<&'a Matched, Vec<u8>>,
    uses: HashMap<(&'a str, u32), Vec<u8>>,
}

impl<'a> Encoder<'a> {
    fn row(
        &mut self,
        row: RowView<'a>,
        images: &[ImageInfo],
        out: &mut Vec<u8>,
        first: bool,
    ) -> Result<(), Failure> {
        let key = context_key(row);
        if !self.contexts.contains_key(&key) {
            self.contexts
                .try_reserve(1)
                .map_err(|error| Failure::input(InputError::Allocation(error)))?;
            self.contexts
                .insert(key, encode_cell(&render_context(row, images), self.format)?);
        }
        if !self.matches.contains_key(row.matched) {
            self.matches
                .try_reserve(1)
                .map_err(|error| Failure::input(InputError::Allocation(error)))?;
            self.matches.insert(
                row.matched,
                encode_cell(&matched_text(row.matched), self.format)?,
            );
        }
        let use_key = (row.use_, row.depth);
        if !self.uses.contains_key(&use_key) {
            self.uses
                .try_reserve(1)
                .map_err(|error| Failure::input(InputError::Allocation(error)))?;
            self.uses.insert(
                use_key,
                encode_cell(&format!("{} depth {}", row.use_, row.depth), self.format)?,
            );
        }
        let context = &self.contexts[&key];
        let matched = &self.matches[row.matched];
        let use_ = &self.uses[&use_key];
        let size = context
            .len()
            .checked_add(matched.len())
            .and_then(|size| size.checked_add(use_.len()))
            .and_then(|size| size.checked_add(64))
            .ok_or_else(|| Failure::input(InputError::PlatformCapacity))?;
        out.try_reserve(size)
            .map_err(|error| Failure::input(InputError::Allocation(error)))?;
        if self.format == Format::Json {
            if !first {
                out.push(b',');
            }
            out.extend_from_slice(b"{\"context\":");
            if let Some(offset) = row.code_unit {
                // The cached JSON string includes its quotes. The offset is
                // ASCII, so it can be appended before the closing quote.
                out.extend_from_slice(&context[..context.len() - 1]);
                write!(out, "@cu:{offset:#06x}\"").map_err(Failure::output)?;
            } else {
                out.extend_from_slice(context);
            }
            out.extend_from_slice(b",\"match\":");
            out.extend_from_slice(matched);
            out.extend_from_slice(b",\"use\":");
            out.extend_from_slice(use_);
            out.push(b'}');
        } else {
            out.extend_from_slice(context);
            if let Some(offset) = row.code_unit {
                write!(out, "@cu:{offset:#06x}").map_err(Failure::output)?;
            }
            out.push(b'\t');
            out.extend_from_slice(matched);
            out.push(b'\t');
            out.extend_from_slice(use_);
            out.push(b'\n');
        }
        Ok(())
    }
}

fn stopped() -> Failure {
    Failure::output(std::io::Error::other("xref output consumer stopped"))
}
