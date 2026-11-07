//! Presentation-only relevance; original DEX identities and coverage are separate.

use std::cmp::Ordering;

use diana_decomp::ByteOffset;
use diana_decomp::input::InputError;

/// Raw nucleo contributions and candidate scalar length. Presentation normalizes
/// score by length so distant matches in long names do not overwhelm compact ones.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Relevance {
    pub score: u32,
    pub length: u64,
}

impl Relevance {
    pub fn combine(self, other: Self) -> Result<Self, InputError> {
        let capacity = || InputError::PlatformCapacity {
            offset_byte: ByteOffset(0),
        };
        Ok(Self {
            score: self.score.checked_add(other.score).ok_or_else(capacity)?,
            length: self.length.checked_add(other.length).ok_or_else(capacity)?,
        })
    }
}

/// Best first; absent relevance follows scored matches.
pub fn best_first(left: Option<Relevance>, right: Option<Relevance>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => {
            // Compare score/length exactly: u32 * u64 fits in u128, with no
            // floating-point rounding. Empty patterns use the neutral 0/1.
            let left_density = u128::from(left.score) * u128::from(right.length.max(1));
            let right_density = u128::from(right.score) * u128::from(left.length.max(1));
            right_density
                .cmp(&left_density)
                .then_with(|| right.score.cmp(&left.score))
                .then_with(|| left.length.cmp(&right.length))
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// A successful predicate can be unranked. Absence of this product means no match.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FilterMatch {
    pub relevance: Option<Relevance>,
}

impl FilterMatch {
    pub fn combine(self, other: Self) -> Result<Self, InputError> {
        Ok(Self {
            relevance: match (self.relevance, other.relevance) {
                (Some(left), Some(right)) => Some(left.combine(right)?),
                (left, right) => left.or(right),
            },
        })
    }
}
