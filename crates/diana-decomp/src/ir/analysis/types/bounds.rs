//! Ordered type evidence shared by constraint collection and fixed-point propagation.
use crate::ir::ArgType;
use smallvec::SmallVec;
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

/// Most SSA values have one or two distinct bounds. Keep their evidence inline and
/// promote high-cardinality values to a tree before insertion shifts become costly.
pub(super) enum TypeBounds {
    Small(SmallVec<[TypeBound; 2]>),
    Tree(BTreeSet<TypeBound>),
}

impl Default for TypeBounds {
    fn default() -> Self {
        Self::Small(SmallVec::new())
    }
}

impl TypeBounds {
    pub(super) fn insert(&mut self, bound: TypeBound) -> bool {
        match self {
            Self::Tree(tree) => tree.insert(bound),
            Self::Small(bounds) => match bounds.binary_search(&bound) {
                Ok(_) => false,
                Err(index) if bounds.len() < 12 => {
                    bounds.insert(index, bound);
                    true
                }
                Err(_) => {
                    let mut tree = BTreeSet::from_iter(std::mem::take(bounds));
                    tree.insert(bound);
                    *self = Self::Tree(tree);
                    true
                }
            },
        }
    }

    pub(super) fn merge(&mut self, other: Self) {
        match other {
            Self::Small(bounds) => {
                for bound in bounds {
                    self.insert(bound);
                }
            }
            Self::Tree(mut tree) => match self {
                Self::Tree(ours) => ours.append(&mut tree),
                Self::Small(ours) => {
                    tree.extend(std::mem::take(ours));
                    *self = Self::Tree(tree);
                }
            },
        }
    }

    pub(super) fn evidence(&self, kind: BoundKind) -> impl Iterator<Item = &ArgType> + Clone {
        let (small, large) = match self {
            Self::Small(bounds) => {
                let start = bounds.partition_point(|bound| bound.kind < kind);
                (&bounds[start..], None)
            }
            Self::Tree(bounds) => {
                // Void is the first ArgType in its derived total order.
                (
                    &[][..],
                    Some(bounds.range(TypeBound::new(kind, ArgType::VOID)..)),
                )
            }
        };
        small
            .iter()
            .chain(large.into_iter().flatten())
            .take_while(move |bound| bound.kind == kind)
            .map(|bound| &bound.ty)
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
