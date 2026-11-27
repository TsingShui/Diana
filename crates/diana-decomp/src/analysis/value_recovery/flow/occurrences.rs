//! Semantic occurrences grouped by value. Keep the identity index small and
//! store the first occurrence directly in the row arena; only repeated uses
//! or definitions of one identity need a separately allocated list.

use std::cell::OnceCell;
use std::collections::hash_map::Entry;

use foldhash::HashMap;
use smallvec::SmallVec;

use crate::ir::analysis::{SsaValueGraph, SsaVar};

pub(super) struct OccurrenceRows<'ir, T> {
    ssa_values: Option<&'ir SsaValueGraph>,
    first_unknown: Option<SsaVar>,
    repeated: bool,
    index: RowIndex,
    rows: Vec<Occurrences<T>>,
}

/// Collection appends occurrences in event order. Proofs visit distinct keys
/// in SSA order, independently of the stable row IDs used for point lookups.
enum RowIndex {
    Small(SmallVec<[(SsaVar, usize); 8]>),
    Large {
        by_key: HashMap<SsaVar, usize>,
        ordered: OnceCell<Box<[(SsaVar, usize)]>>,
    },
}

impl RowIndex {
    fn get_or_insert(&mut self, key: SsaVar, next: usize) -> usize {
        if let Self::Small(entries) = self {
            match entries.binary_search_by_key(&key, |entry| entry.0) {
                Ok(index) => return entries[index].1,
                Err(index) if entries.len() < entries.inline_size() => {
                    entries.insert(index, (key, next));
                    return next;
                }
                Err(_) => {
                    *self = Self::Large {
                        by_key: entries.iter().copied().collect(),
                        ordered: OnceCell::new(),
                    };
                }
            }
        }
        let Self::Large { by_key, ordered } = self else {
            unreachable!()
        };
        match by_key.entry(key) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                ordered.take();
                entry.insert(next);
                next
            }
        }
    }

    fn get(&self, key: &SsaVar) -> Option<usize> {
        match self {
            Self::Small(entries) => entries
                .binary_search_by_key(key, |entry| entry.0)
                .ok()
                .map(|i| entries[i].1),
            Self::Large { by_key, .. } => by_key.get(key).copied(),
        }
    }

    fn ordered(&self) -> &[(SsaVar, usize)] {
        match self {
            Self::Small(entries) => entries,
            Self::Large { by_key, ordered } => ordered.get_or_init(|| {
                let mut entries = by_key
                    .iter()
                    .map(|(&key, &row)| (key, row))
                    .collect::<Vec<_>>();
                entries.sort_unstable_by_key(|entry| entry.0);
                entries.into_boxed_slice()
            }),
        }
    }
}

enum Occurrences<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> Occurrences<T> {
    fn push(&mut self, value: T) {
        match self {
            Self::Many(values) => values.push(value),
            Self::One(_) => {
                let Self::One(first) = std::mem::replace(self, Self::Many(Vec::new())) else {
                    unreachable!("occupied singleton")
                };
                *self = Self::Many(vec![first, value]);
            }
        }
    }

    fn as_slice(&self) -> &[T] {
        match self {
            Self::One(value) => std::slice::from_ref(value),
            Self::Many(values) => values,
        }
    }
}

impl<T> Default for OccurrenceRows<'_, T> {
    fn default() -> Self {
        Self {
            ssa_values: None,
            first_unknown: None,
            repeated: false,
            index: RowIndex::Small(SmallVec::new()),
            rows: Vec::new(),
        }
    }
}

impl<'ir, T> OccurrenceRows<'ir, T> {
    pub(super) fn ssa(values: &'ir SsaValueGraph) -> Self {
        Self {
            ssa_values: Some(values),
            ..Default::default()
        }
    }
    pub(super) fn has_repeated(&self) -> bool {
        self.repeated
    }
    pub(super) fn first_unknown(&self) -> Option<SsaVar> {
        self.first_unknown
    }

    pub(super) fn push(&mut self, key: SsaVar, value: T) {
        let row = self.index.get_or_insert(key, self.rows.len());
        if row == self.rows.len() {
            if self
                .ssa_values
                .is_some_and(|values| values.value(key).is_none())
            {
                self.first_unknown =
                    Some(self.first_unknown.map_or(key, |previous| previous.min(key)));
            }
            self.rows.push(Occurrences::One(value));
        } else {
            self.repeated = true;
            self.rows[row].push(value);
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub(super) fn get(&self, key: &SsaVar) -> Option<&[T]> {
        self.index.get(key).map(|i| self.rows[i].as_slice())
    }

    pub(super) fn contains_key(&self, key: &SsaVar) -> bool {
        self.index.get(key).is_some()
    }

    pub(super) fn keys(&self) -> impl Iterator<Item = &SsaVar> {
        self.index.ordered().iter().map(|(key, _)| key)
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&SsaVar, &[T])> {
        self.index
            .ordered()
            .iter()
            .map(|(key, i)| (key, self.rows[*i].as_slice()))
    }

    pub(super) fn row(&self, key: &SsaVar) -> Option<usize> {
        self.index.get(key)
    }

    pub(super) fn values(&self) -> impl ExactSizeIterator<Item = &[T]> {
        self.rows.iter().map(Occurrences::as_slice)
    }
}

#[cfg(test)]
impl<T> std::ops::Index<&SsaVar> for OccurrenceRows<'_, T> {
    type Output = [T];
    fn index(&self, key: &SsaVar) -> &Self::Output {
        self.get(key).expect("fixture value has occurrences")
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/value_occurrence_index.rs"]
mod tests;
