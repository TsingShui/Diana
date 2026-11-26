//! Semantic occurrences grouped by value. Keep the identity index small and
//! store the first occurrence directly in the row arena; only repeated uses
//! or definitions of one identity need a separately allocated list.

use std::collections::{btree_map::Entry, BTreeMap};

use crate::ir::analysis::SsaVar;

pub(super) struct OccurrenceRows<T> {
    index: BTreeMap<SsaVar, usize>,
    rows: Vec<Occurrences<T>>,
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

impl<T> Default for OccurrenceRows<T> {
    fn default() -> Self {
        Self {
            index: BTreeMap::new(),
            rows: Vec::new(),
        }
    }
}

impl<T> OccurrenceRows<T> {
    pub(super) fn push(&mut self, key: SsaVar, value: T) {
        match self.index.entry(key) {
            Entry::Occupied(row) => self.rows[*row.get()].push(value),
            Entry::Vacant(slot) => {
                slot.insert(self.rows.len());
                self.rows.push(Occurrences::One(value));
            }
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub(super) fn get(&self, key: &SsaVar) -> Option<&[T]> {
        self.index.get(key).map(|&i| self.rows[i].as_slice())
    }

    pub(super) fn contains_key(&self, key: &SsaVar) -> bool {
        self.index.contains_key(key)
    }

    pub(super) fn keys(&self) -> impl Iterator<Item = &SsaVar> {
        self.index.keys()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&SsaVar, &[T])> {
        self.index
            .iter()
            .map(|(key, &i)| (key, self.rows[i].as_slice()))
    }

    pub(super) fn values(&self) -> impl Iterator<Item = &[T]> {
        self.index.values().map(|&i| self.rows[i].as_slice())
    }
}

#[cfg(test)]
impl<T> std::ops::Index<&SsaVar> for OccurrenceRows<T> {
    type Output = [T];
    fn index(&self, key: &SsaVar) -> &Self::Output {
        self.get(key).expect("fixture value has occurrences")
    }
}
