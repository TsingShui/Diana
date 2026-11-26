//! Transitive origins of SSA copies and Phi values.

use std::collections::{BTreeMap, BTreeSet};

use super::{SsaValueGraph, SsaVar};

#[derive(Debug, Clone, Default)]
pub struct SsaOrigins(BTreeMap<SsaVar, BTreeSet<SsaVar>>);

impl SsaOrigins {
    pub fn analyze(values: &SsaValueGraph) -> Self {
        let mut origins = BTreeMap::<SsaVar, BTreeSet<SsaVar>>::new();
        let mut dependents = BTreeMap::<SsaVar, Vec<SsaVar>>::new();
        for copy in values.copies() {
            origins.entry(copy.result).or_default();
            dependents.entry(copy.source).or_default().push(copy.result);
        }
        for phi in values.phis() {
            origins.entry(phi.result).or_default();
            for input in &phi.inputs {
                dependents.entry(input.value).or_default().push(phi.result);
            }
        }

        // Copies and Phis carry the union of their input roots. Send each
        // known root through the reverse dependencies once. Rootless cycles
        // and unknown inputs contribute no fact, just as an empty Phi does.
        let mut pending = Vec::new();
        for value in values.values() {
            let root = value.variable;
            let std::collections::btree_map::Entry::Vacant(entry) = origins.entry(root) else {
                continue;
            };
            entry.insert(BTreeSet::from([root]));
            pending.push(root);
            while let Some(source) = pending.pop() {
                for result in dependents.get(&source).into_iter().flatten() {
                    if origins.entry(*result).or_default().insert(root) {
                        pending.push(*result);
                    }
                }
            }
        }
        Self(origins)
    }

    pub fn origins(&self, value: SsaVar) -> Option<&BTreeSet<SsaVar>> {
        self.0.get(&value)
    }

    pub fn equivalent(&self, left: SsaVar, right: SsaVar) -> bool {
        match (self.origins(left), self.origins(right)) {
            (Some(left), Some(right)) => !left.is_empty() && left == right,
            _ => false,
        }
    }

    pub fn unique(&self, value: SsaVar) -> Option<SsaVar> {
        let roots = self.origins(value)?;
        (roots.len() == 1).then(|| roots.first().copied()).flatten()
    }
}
