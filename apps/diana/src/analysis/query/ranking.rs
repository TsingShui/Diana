//! Worker chunks share one scan. Only fuzzy results retain formatted
//! rows for global ranking; ordinary chunks keep the streaming/backpressure path.

use super::{CHUNK_BYTES, QueryError, Row};
use crate::analysis::filter::{Relevance, best_first};
use crate::analysis::input::InputError;

pub(super) enum Batch {
    Streaming(Vec<u8>),
    Ranked(Vec<RankedRow>),
}

pub(super) struct RankedRow {
    relevance: Relevance,
    order: (usize, u32, u32, u32),
    bytes: Vec<u8>,
}

pub(super) struct Buffer {
    ranked: bool,
    bytes: Vec<u8>,
    rows: Vec<RankedRow>,
    size: usize,
    first: bool,
}

impl Buffer {
    pub(super) fn new(ranked: bool) -> Self {
        Self {
            ranked,
            bytes: Vec::new(),
            rows: Vec::new(),
            size: 0,
            first: true,
        }
    }

    pub(super) fn push<R>(
        &mut self,
        row: Row<'_>,
        render: &R,
        limit: usize,
        emitted: &std::sync::atomic::AtomicUsize,
        truncated: &std::sync::atomic::AtomicBool,
        emit: &mut dyn FnMut(Batch) -> Result<(), QueryError>,
    ) -> Result<(), QueryError>
    where
        R: for<'r> Fn(Row<'r>, &mut Vec<u8>, bool) -> Result<(), QueryError>,
    {
        if limit != 0 && emitted.fetch_add(1, std::sync::atomic::Ordering::Relaxed) >= limit {
            truncated.store(true, std::sync::atomic::Ordering::Relaxed);
            return Ok(());
        }
        if self.ranked {
            let relevance = row.relevance().ok_or({
                QueryError::Engine(diana_decomp::input::InputError::InternalInvariant(
                    "fuzzy row lost relevance",
                ))
            })?;
            let order = row.order_key();
            let mut bytes = Vec::new();
            // Each ranked row is a self-contained chunk. The existing sink owns
            // JSON separators, so text and JSON use the same sorted products.
            render(row, &mut bytes, true)?;
            self.size = self
                .size
                .checked_add(bytes.len())
                .ok_or(InputError::PlatformCapacity)?;
            self.rows.try_reserve(1).map_err(InputError::Allocation)?;
            self.rows.push(RankedRow {
                relevance,
                order,
                bytes,
            });
        } else {
            render(row, &mut self.bytes, self.first)?;
            self.first = false;
            self.size = self.bytes.len();
        }
        if self.size >= CHUNK_BYTES {
            self.flush(emit)?;
        }
        Ok(())
    }

    pub(super) fn flush(
        &mut self,
        emit: &mut dyn FnMut(Batch) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        if self.ranked {
            if !self.rows.is_empty() {
                emit(Batch::Ranked(std::mem::take(&mut self.rows)))?;
            }
        } else if !self.bytes.is_empty() {
            emit(Batch::Streaming(std::mem::take(&mut self.bytes)))?;
        }
        self.size = 0;
        self.first = true;
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct Collected {
    rows: Vec<RankedRow>,
}

impl Collected {
    pub(super) fn accept(
        &mut self,
        batch: Batch,
        sink: &mut impl FnMut(Vec<u8>) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        match batch {
            Batch::Streaming(bytes) => sink(bytes),
            Batch::Ranked(mut rows) => {
                self.rows
                    .try_reserve(rows.len())
                    .map_err(InputError::Allocation)?;
                self.rows.append(&mut rows);
                Ok(())
            }
        }
    }

    pub(super) fn finish(
        mut self,
        sink: &mut impl FnMut(Vec<u8>) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        self.rows.sort_unstable_by(|left, right| {
            best_first(Some(left.relevance), Some(right.relevance))
                .then_with(|| left.order.cmp(&right.order))
        });
        for row in self.rows {
            sink(row.bytes)?;
        }
        Ok(())
    }
}
