//! Method-local SSA ordinals. Only nonempty words beyond the first allocate.
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct LiveSet {
    low: u64,
    high: BTreeMap<usize, u64>,
}

impl LiveSet {
    pub(super) fn word(&self, index: usize) -> u64 {
        if index == 0 {
            self.low
        } else {
            self.high.get(&index).copied().unwrap_or(0)
        }
    }

    /// Return precisely the newly live bits for propagation to earlier blocks.
    pub(super) fn add(&mut self, index: usize, bits: u64) -> u64 {
        if bits == 0 {
            return 0;
        }
        let word = if index == 0 {
            &mut self.low
        } else {
            self.high.entry(index).or_default()
        };
        let added = bits & !*word;
        *word |= bits;
        added
    }

    pub(super) fn insert(&mut self, id: usize) {
        self.add(id / 64, 1 << (id % 64));
    }

    pub(super) fn remove(&mut self, id: usize) {
        let index = id / 64;
        let keep = !(1 << (id % 64));
        if index == 0 {
            self.low &= keep;
        } else if let std::collections::btree_map::Entry::Occupied(mut row) = self.high.entry(index)
        {
            *row.get_mut() &= keep;
            if *row.get() == 0 {
                row.remove();
            }
        }
    }

    pub(super) fn contains(&self, id: usize) -> bool {
        self.word(id / 64) & (1 << (id % 64)) != 0
    }

    pub(super) fn words(&self) -> impl Iterator<Item = (usize, u64)> + Clone + '_ {
        (self.low != 0)
            .then_some((0, self.low))
            .into_iter()
            .chain(self.high.iter().map(|(&i, &bits)| (i, bits)))
    }

    pub(super) fn ids(&self) -> impl Iterator<Item = usize> + Clone + '_ {
        self.words().flat_map(|(index, mut bits)| {
            std::iter::from_fn(move || {
                if bits == 0 {
                    return None;
                }
                let bit = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                Some(index * 64 + bit)
            })
        })
    }
}
