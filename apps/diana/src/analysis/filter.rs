//! Compiled query predicates. Literal matching keeps the existing UTF-16 rules;
//! regex matching consumes a Unicode text view without changing delivered facts.

use std::borrow::Cow;

use diana_decomp::FailureKind;
use diana_decomp::input::{DexString, InputError};
use regex::{Regex, RegexBuilder};

#[derive(Clone, Copy, Debug)]
pub enum CaseSensitivity {
    Sensitive,
    Insensitive,
}

#[derive(Clone, Debug)]
pub enum TextFilter {
    Literal {
        needle: Vec<u16>,
        case: CaseSensitivity,
    },
    Regex(Regex),
}

#[derive(Debug, thiserror::Error)]
pub enum FilterError {
    #[error("filter regular expression could not be compiled: {0}")]
    Regex(#[from] regex::Error),
    #[error("filter allocation failed: {0}")]
    Allocation(#[source] std::collections::TryReserveError),
}

impl FilterError {
    pub fn kind(&self) -> FailureKind {
        match self {
            Self::Regex(regex::Error::CompiledTooBig(_)) | Self::Allocation(_) => {
                FailureKind::ResourceExhausted
            }
            Self::Regex(_) => FailureKind::Unsupported,
        }
    }
}

impl TextFilter {
    pub fn literal(pattern: &str, case: CaseSensitivity) -> Result<Self, FilterError> {
        let mut needle = Vec::new();
        needle
            .try_reserve_exact(pattern.len())
            .map_err(FilterError::Allocation)?;
        needle.extend(pattern.encode_utf16());
        Ok(Self::Literal { needle, case })
    }

    /// Compiles once, before input scanning. Rust regex syntax has no backreferences
    /// or look-around; syntax/compilation failures are not empty matches.
    pub fn regex(pattern: &str, case: CaseSensitivity) -> Result<Self, FilterError> {
        Ok(Self::Regex(
            RegexBuilder::new(pattern)
                .case_insensitive(matches!(case, CaseSensitivity::Insensitive))
                .build()?,
        ))
    }

    /// Regex clones share the compiled program but have worker-local search caches.
    /// Literals stay borrowed, preserving their allocation-free query path.
    pub(super) fn for_worker(&self) -> Cow<'_, Self> {
        match self {
            Self::Literal { .. } => Cow::Borrowed(self),
            Self::Regex(_) => Cow::Owned(self.clone()),
        }
    }

    pub(super) fn matches(&self, value: DexString<'_>) -> Result<bool, InputError> {
        match self {
            Self::Literal {
                needle,
                case: CaseSensitivity::Insensitive,
            } => value.contains_ignore_case(needle),
            Self::Literal {
                needle,
                case: CaseSensitivity::Sensitive,
            } => Ok(super::dex::contains(&value.units()?, needle)),
            Self::Regex(regex) => Ok(regex.is_match(&value.utf8_lossy()?)),
        }
    }

    pub(super) fn matches_text(&self, value: &str) -> Result<bool, super::input::InputError> {
        let (needle, case) = match self {
            Self::Regex(regex) => return Ok(regex.is_match(value)),
            Self::Literal { needle, case } => (needle, case),
        };
        if needle.is_empty() {
            return Ok(true);
        }
        let mut units = Vec::new();
        units
            .try_reserve_exact(value.len())
            .map_err(super::input::InputError::Allocation)?;
        units.extend(value.encode_utf16());
        Ok(match case {
            CaseSensitivity::Sensitive => super::dex::contains(&units, needle),
            CaseSensitivity::Insensitive => units.windows(needle.len()).any(|window| {
                window.iter().zip(needle).all(|(&left, &right)| {
                    left == right
                        || match (
                            char::from_u32(u32::from(left)),
                            char::from_u32(u32::from(right)),
                        ) {
                            (Some(left), Some(right)) => {
                                left.to_lowercase().eq(right.to_lowercase())
                            }
                            _ => false,
                        }
                })
            }),
        })
    }
}
