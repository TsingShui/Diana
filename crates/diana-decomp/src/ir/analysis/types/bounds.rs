//! Ordered type evidence shared by constraint collection and fixed-point propagation.
use crate::ir::ArgType;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum BoundKind {
    Domain,
    Fallback,
    Upper,
    Lower,
    Exact,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct TypeBound {
    pub(super) kind: BoundKind,
    pub(super) ty: ArgType,
}

impl TypeBound {
    pub(super) fn new(kind: BoundKind, ty: ArgType) -> Self {
        Self { kind, ty }
    }
}

#[derive(Clone, Copy)]
struct BoundKinds(u8);

impl BoundKinds {
    fn one(kind: BoundKind) -> Self {
        Self(1 << kind as u8)
    }

    fn has(self, kind: BoundKind) -> bool {
        self.0 & Self::one(kind).0 != 0
    }

    fn extend(&mut self, other: Self) -> bool {
        let before = self.0;
        self.0 |= other.0;
        self.0 != before
    }

    fn insert_into(self, tree: &mut BTreeSet<TypeBound>, ty: Cow<'_, ArgType>) -> bool {
        let mut kinds = [
            BoundKind::Domain,
            BoundKind::Fallback,
            BoundKind::Upper,
            BoundKind::Lower,
            BoundKind::Exact,
        ]
        .into_iter()
        .filter(|&kind| self.has(kind));
        let Some(mut kind) = kinds.next() else {
            return false;
        };
        let mut changed = false;
        for next in kinds {
            changed |= tree.insert(TypeBound::new(kind, ty.as_ref().clone()));
            kind = next;
        }
        changed | tree.insert(TypeBound::new(kind, ty.into_owned()))
    }
}

struct TypeEvidence {
    ty: ArgType,
    kinds: BoundKinds,
}

/// Register observations frequently contribute the same type to several kinds
/// of evidence. Keep that type once while the row is small. A high-cardinality
/// row uses a kind-first tree for ordered range queries.
pub(super) struct TypeBounds(BoundStorage);

enum BoundStorage {
    Small(SmallVec<[TypeEvidence; 2]>),
    Tree(BTreeSet<TypeBound>),
}

impl Default for TypeBounds {
    fn default() -> Self {
        Self(BoundStorage::Small(SmallVec::new()))
    }
}

impl TypeBounds {
    pub(super) fn sole_type(&self) -> Option<&ArgType> {
        match &self.0 {
            BoundStorage::Small(types) => match types.as_slice() {
                [entry] => Some(&entry.ty),
                _ => None,
            },
            BoundStorage::Tree(_) => None,
        }
    }

    pub(super) fn observe(&mut self, ty: &ArgType) {
        let mut kinds = BoundKinds::one(BoundKind::Domain);
        if ty.is_known() {
            kinds.extend(BoundKinds::one(BoundKind::Fallback));
        }
        self.insert_kinds(kinds, Cow::Borrowed(ty));
    }

    pub(super) fn insert(&mut self, bound: TypeBound) -> bool {
        self.insert_kinds(BoundKinds::one(bound.kind), Cow::Owned(bound.ty))
    }

    pub(super) fn insert_ref(&mut self, kind: BoundKind, ty: &ArgType) -> bool {
        self.insert_kinds(BoundKinds::one(kind), Cow::Borrowed(ty))
    }

    fn insert_kinds(&mut self, kinds: BoundKinds, ty: Cow<'_, ArgType>) -> bool {
        match &mut self.0 {
            BoundStorage::Tree(tree) => kinds.insert_into(tree, ty),
            BoundStorage::Small(types) => {
                match types.binary_search_by(|entry| entry.ty.cmp(ty.as_ref())) {
                    Ok(index) => types[index].kinds.extend(kinds),
                    Err(index) if types.len() < 12 => {
                        types.insert(
                            index,
                            TypeEvidence {
                                ty: ty.into_owned(),
                                kinds,
                            },
                        );
                        true
                    }
                    Err(_) => {
                        let mut tree = BTreeSet::new();
                        for entry in std::mem::take(types) {
                            entry.kinds.insert_into(&mut tree, Cow::Owned(entry.ty));
                        }
                        kinds.insert_into(&mut tree, ty);
                        self.0 = BoundStorage::Tree(tree);
                        true
                    }
                }
            }
        }
    }

    pub(super) fn merge(&mut self, other: Self) {
        match other.0 {
            BoundStorage::Small(types) => {
                for entry in types {
                    self.insert_kinds(entry.kinds, Cow::Owned(entry.ty));
                }
            }
            BoundStorage::Tree(mut tree) => match &mut self.0 {
                BoundStorage::Tree(ours) => ours.append(&mut tree),
                BoundStorage::Small(ours) => {
                    for entry in std::mem::take(ours) {
                        entry.kinds.insert_into(&mut tree, Cow::Owned(entry.ty));
                    }
                    self.0 = BoundStorage::Tree(tree);
                }
            },
        }
    }

    pub(super) fn evidence(&self, kind: BoundKind) -> impl Iterator<Item = &ArgType> + Clone {
        let (small, large) = match &self.0 {
            BoundStorage::Small(types) => (&types[..], None),
            // Void is the first ArgType in its derived total order.
            BoundStorage::Tree(tree) => (
                &[][..],
                Some(tree.range(TypeBound::new(kind, ArgType::VOID)..)),
            ),
        };
        small
            .iter()
            .filter(move |entry| entry.kinds.has(kind))
            .map(|entry| &entry.ty)
            .chain(
                large
                    .into_iter()
                    .flatten()
                    .take_while(move |bound| bound.kind == kind)
                    .map(|bound| &bound.ty),
            )
    }

    #[cfg(test)]
    pub(super) fn contains(&self, bound: &TypeBound) -> bool {
        self.evidence(bound.kind).any(|ty| ty == &bound.ty)
    }
}

#[cfg(test)]
impl<const N: usize> From<[TypeBound; N]> for TypeBounds {
    fn from(bounds: [TypeBound; N]) -> Self {
        let mut state = Self::default();
        for bound in bounds {
            state.insert(bound);
        }
        state
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/type_bound_groups.rs"]
mod tests;
