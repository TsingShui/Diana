//! Both input versions use the same request-local namespace. Index workers
//! intern symbolic operands per DEX pool entry. After all scans, comparators
//! borrow their actual values without acquiring locks or trusting a digest.
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

const SHARDS: usize = 32;
const SHARD_BITS: u32 = SHARDS.trailing_zeros();

type Table = [Vec<Arc<str>>; SHARDS];

#[derive(Debug, Clone, Default)]
pub(super) struct Names(Arc<OnceLock<Table>>);

impl Names {
    pub(super) fn get(&self, id: u32) -> Option<&str> {
        self.0
            .get()?
            .get(id as usize % SHARDS)?
            .get((id >> SHARD_BITS) as usize)
            .map(AsRef::as_ref)
    }
}

#[derive(Default)]
struct Shard {
    ids: HashMap<Arc<str>, u32>,
    values: Vec<Arc<str>>,
}

#[derive(Default)]
pub(in crate::analysis) struct Symbols {
    shards: [Mutex<Shard>; SHARDS],
    names: Names,
}

impl Symbols {
    pub(super) fn names(&self) -> Names {
        self.names.clone()
    }

    pub(super) fn intern(&self, tag: u8, text: String) -> Result<super::reference::Symbol, String> {
        let mut hash = DefaultHasher::new();
        (tag, text.as_str()).hash(&mut hash);
        let hash = hash.finish();
        let anchor = (tag == b's' || text.starts_with("Landroid/") || text.starts_with("Ljava/"))
            .then_some(hash);
        let shard = hash as usize % SHARDS;
        let mut state = self.shards[shard]
            .lock()
            .map_err(|_| "symbol state unavailable")?;
        let id = if let Some(&id) = state.ids.get(text.as_str()) {
            id
        } else {
            let offset = u32::try_from(state.values.len())
                .ok()
                .filter(|&n| n <= u32::MAX >> SHARD_BITS)
                .ok_or("too many distinct DEX references")?;
            let id = (offset << SHARD_BITS) | shard as u32;
            state.values.try_reserve(1).map_err(|e| e.to_string())?;
            state.ids.try_reserve(1).map_err(|e| e.to_string())?;
            let text = Arc::<str>::from(text);
            state.values.push(text.clone());
            state.ids.insert(text, id);
            id
        };
        Ok(super::reference::Symbol { tag, id, anchor })
    }

    pub(in crate::analysis) fn finish(self) {
        let table = self
            .shards
            .map(|m| m.into_inner().unwrap_or_else(|e| e.into_inner()).values);
        let _ = self.names.0.set(table);
    }
}
