//! Captured DEX block membership is independent of later CFG edits. A dense
//! physical block range has implicit empty coverage until a handler covers
//! it. Rows record protected blocks, out-of-range identities and removals.

use super::{BTreeMap, BTreeSet, Block, BlockId};

#[derive(Debug, Clone, Default)]
pub(super) struct ExceptionCoverage {
    dense: Option<(BlockId, BlockId)>,
    rows: BTreeMap<BlockId, Option<BTreeSet<(u32, u32)>>>,
}

impl ExceptionCoverage {
    pub(super) fn capture(blocks: &BTreeMap<BlockId, Block>) -> Self {
        let mut coverage = Self::default();
        if let (Some((&first, _)), Some((&last, _))) =
            (blocks.first_key_value(), blocks.last_key_value())
        {
            if u64::from(last.0 - first.0) + 1 == blocks.len() as u64 {
                coverage.dense = Some((first, last));
            } else {
                coverage
                    .rows
                    .extend(blocks.keys().map(|&id| (id, Some(BTreeSet::new()))));
            }
        }
        coverage
    }

    fn implicit(&self, id: BlockId) -> bool {
        self.dense
            .is_some_and(|(first, last)| first <= id && id <= last)
    }

    pub(super) fn protect(&mut self, id: BlockId, range: (u32, u32)) {
        self.rows
            .entry(id)
            .or_default()
            .get_or_insert_with(BTreeSet::new)
            .insert(range);
    }

    pub(super) fn contains(&self, id: BlockId) -> bool {
        self.rows
            .get(&id)
            .map_or_else(|| self.implicit(id), Option::is_some)
    }

    pub(super) fn ranges(&self, id: BlockId) -> &BTreeSet<(u32, u32)> {
        static EMPTY: BTreeSet<(u32, u32)> = BTreeSet::new();
        self.rows
            .get(&id)
            .and_then(Option::as_ref)
            .unwrap_or(&EMPTY)
    }

    pub(super) fn set(&mut self, id: BlockId, ranges: BTreeSet<(u32, u32)>) {
        if ranges.is_empty() && self.implicit(id) {
            self.rows.remove(&id);
        } else {
            self.rows.insert(id, Some(ranges));
        }
    }

    pub(super) fn remove(&mut self, id: BlockId) {
        if self.implicit(id) {
            self.rows.insert(id, None);
        } else {
            self.rows.remove(&id);
        }
    }
}
