//! Mutable adjacency indexed by the block numbers assigned by the splitter.
//! Region projections may keep widely separated physical IDs in the same CFG.
use smallvec::SmallVec;
use std::collections::{btree_map, BTreeMap};

use super::{BlockId, EdgeKind};

type Edges = SmallVec<[(BlockId, EdgeKind); 2]>;

#[derive(Debug, Clone, Default)]
pub(super) struct EdgeRows {
    prefix: Vec<Option<Edges>>,
    // All keys are outside the prefix. An arbitrary input ID costs one row.
    sparse: BTreeMap<BlockId, Edges>,
    count: usize,
}

impl EdgeRows {
    pub(super) fn len(&self) -> usize {
        self.count
    }

    pub(super) fn get(&self, id: BlockId) -> Option<&Edges> {
        match self.prefix.get(id.0 as usize) {
            Some(slot) => slot.as_ref(),
            None => self.sparse.get(&id),
        }
    }

    pub(super) fn get_mut(&mut self, id: BlockId) -> Option<&mut Edges> {
        match self.prefix.get_mut(id.0 as usize) {
            Some(slot) => slot.as_mut(),
            None => self.sparse.get_mut(&id),
        }
    }

    pub(super) fn row(&mut self, id: BlockId) -> &mut Edges {
        let i = id.0 as usize;
        if i >= self.prefix.len() {
            // A short, dense extension admits forward branch targets and bounds
            // the prefix by live rows, including after arbitrary graph edits.
            if i - self.prefix.len() > 4 || i >= self.count.saturating_mul(2).saturating_add(8) {
                return self.sparse.entry(id).or_insert_with(|| {
                    self.count += 1;
                    Edges::new()
                });
            }
            for number in self.prefix.len()..=i {
                self.prefix
                    .push(self.sparse.remove(&BlockId(number as u32)));
            }
        }
        self.prefix[i].get_or_insert_with(|| {
            self.count += 1;
            Edges::new()
        })
    }

    pub(super) fn remove(&mut self, id: BlockId) -> Option<Edges> {
        let removed = match self.prefix.get_mut(id.0 as usize) {
            Some(slot) => slot.take(),
            None => self.sparse.remove(&id),
        };
        if removed.is_some() {
            self.count -= 1;
        }
        removed
    }

    pub(super) fn iter(&self) -> Rows<'_> {
        Rows {
            prefix: self.prefix.iter().enumerate(),
            sparse: self.sparse.iter(),
            remaining: self.count,
        }
    }

    pub(super) fn retain(&mut self, mut keep: impl FnMut(&mut Edges) -> bool) {
        for slot in &mut self.prefix {
            if slot.as_mut().is_some_and(|row| !keep(row)) {
                *slot = None;
                self.count -= 1;
            }
        }
        self.sparse.retain(|_, row| {
            let kept = keep(row);
            self.count -= usize::from(!kept);
            kept
        });
    }
}

#[derive(Clone)]
pub(super) struct Rows<'a> {
    prefix: std::iter::Enumerate<std::slice::Iter<'a, Option<Edges>>>,
    sparse: btree_map::Iter<'a, BlockId, Edges>,
    remaining: usize,
}

impl<'a> Iterator for Rows<'a> {
    type Item = (BlockId, &'a Edges);

    fn next(&mut self) -> Option<Self::Item> {
        let row = self
            .prefix
            .find_map(|(i, slot)| Some((BlockId(i as u32), slot.as_ref()?)))
            .or_else(|| self.sparse.next().map(|(&id, edges)| (id, edges)))?;
        self.remaining -= 1;
        Some(row)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for Rows<'_> {}

#[cfg(test)]
#[path = "../../../tests/internal/cfg_adjacency.rs"]
mod tests;
