//! Shared query predicates. Fuzzy selection retains nucleo relevance for ranked
//! presentation; exact/literal matching retains UTF-16 identities and regex uses
//! a scalar text projection.

use std::borrow::Cow;

use diana_decomp::FailureKind;
use diana_decomp::input::{DexString, InputError};
use regex::{Regex, RegexBuilder};

mod fuzzy;
mod relevance;
pub use fuzzy::FuzzyPattern;
pub use relevance::{FilterMatch, Relevance, best_first};

/// Absence of a filter selects a row without assigning relevance.
pub(super) fn select(
    filter: Option<&TextFilter>,
    value: DexString<'_>,
) -> Result<Option<FilterMatch>, InputError> {
    match filter {
        Some(filter) => filter.evaluate(value),
        None => Ok(Some(FilterMatch::default())),
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CaseSensitivity {
    Sensitive,
    Insensitive,
}

/// Matching semantics shared by list filters and reference targets.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MatchMode {
    Substring,
    #[default]
    Fuzzy,
    Regex,
    Exact,
}

#[derive(Clone, Debug)]
pub enum TextFilter {
    Exact(Vec<u16>),
    Fuzzy(FuzzyPattern),
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
    #[error(
        "fuzzy pattern exceeds nucleo-matcher's u16 score representation; use an exact query or a shorter pattern"
    )]
    MatcherCapacity,
}

impl FilterError {
    pub fn kind(&self) -> FailureKind {
        match self {
            Self::Regex(regex::Error::CompiledTooBig(_)) | Self::Allocation(_) => {
                FailureKind::ResourceExhausted
            }
            Self::Regex(_) | Self::MatcherCapacity => FailureKind::Unsupported,
        }
    }
}

impl TextFilter {
    pub fn compile(pattern: &str, mode: MatchMode) -> Result<Self, FilterError> {
        match mode {
            MatchMode::Substring => Self::literal(pattern, CaseSensitivity::Insensitive),
            MatchMode::Fuzzy => Self::fuzzy(pattern),
            MatchMode::Regex => Self::regex(pattern, CaseSensitivity::Insensitive),
            MatchMode::Exact => {
                let mut units = Vec::new();
                units
                    .try_reserve_exact(pattern.len())
                    .map_err(FilterError::Allocation)?;
                units.extend(pattern.encode_utf16());
                Ok(Self::Exact(units))
            }
        }
    }

    pub fn fuzzy(pattern: &str) -> Result<Self, FilterError> {
        Ok(Self::Fuzzy(FuzzyPattern::new(pattern)?))
    }

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
    /// Literal and fuzzy predicates stay borrowed; regex caches are worker-local.
    pub(super) fn for_worker(&self) -> Cow<'_, Self> {
        match self {
            Self::Exact(_) | Self::Fuzzy(_) | Self::Literal { .. } => Cow::Borrowed(self),
            Self::Regex(_) => Cow::Owned(self.clone()),
        }
    }

    pub fn is_fuzzy(&self) -> bool {
        matches!(self, Self::Fuzzy(_))
    }

    pub(super) fn evaluate(&self, value: DexString<'_>) -> Result<Option<FilterMatch>, InputError> {
        let matched = match self {
            Self::Exact(units) => value.matches(units)?,
            Self::Fuzzy(pattern) => {
                return Ok(pattern.evaluate(value)?.map(|relevance| FilterMatch {
                    relevance: Some(relevance),
                }));
            }
            Self::Literal {
                needle,
                case: CaseSensitivity::Insensitive,
            } => value.contains_ignore_case(needle)?,
            Self::Literal {
                needle,
                case: CaseSensitivity::Sensitive,
            } => super::dex::contains(&value.units()?, needle),
            Self::Regex(regex) => regex.is_match(&value.utf8_lossy()?),
        };
        Ok(matched.then_some(FilterMatch::default()))
    }

    pub(super) fn evaluate_units(
        &self,
        units: &[u16],
    ) -> Result<Option<FilterMatch>, super::query::QueryError> {
        if let Self::Exact(expected) = self {
            return Ok((units == expected).then_some(FilterMatch::default()));
        }
        if let Self::Literal { needle, case } = self {
            let hit = needle.is_empty()
                || units.windows(needle.len()).any(|window| {
                    window.iter().zip(needle).all(|(&left, &right)| {
                        left == right
                            || matches!(case, CaseSensitivity::Insensitive)
                                && match (
                                    char::from_u32(u32::from(left)),
                                    char::from_u32(u32::from(right)),
                                ) {
                                    (Some(left), Some(right)) => {
                                        left.to_lowercase().eq(right.to_lowercase())
                                    }
                                    _ => false,
                                }
                    })
                });
            return Ok(hit.then_some(FilterMatch::default()));
        }
        let mut text = String::new();
        for character in char::decode_utf16(units.iter().copied()) {
            let character = character.unwrap_or(char::REPLACEMENT_CHARACTER);
            text.try_reserve(character.len_utf8())
                .map_err(super::input::InputError::Allocation)?;
            text.push(character);
        }
        self.evaluate_text(&text)
    }

    pub(super) fn evaluate_text(
        &self,
        value: &str,
    ) -> Result<Option<FilterMatch>, super::query::QueryError> {
        let (needle, case) = match self {
            Self::Exact(units) => {
                return Ok(value
                    .encode_utf16()
                    .eq(units.iter().copied())
                    .then_some(FilterMatch::default()));
            }
            Self::Fuzzy(pattern) => {
                return Ok(pattern.evaluate_text(value)?.map(|relevance| FilterMatch {
                    relevance: Some(relevance),
                }));
            }
            Self::Regex(regex) => {
                return Ok(regex.is_match(value).then_some(FilterMatch::default()));
            }
            Self::Literal { needle, case } => (needle, case),
        };
        if needle.is_empty() {
            return Ok(Some(FilterMatch::default()));
        }
        let mut units = Vec::new();
        units
            .try_reserve_exact(value.len())
            .map_err(super::input::InputError::Allocation)?;
        units.extend(value.encode_utf16());
        let matched = match case {
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
        };
        Ok(matched.then_some(FilterMatch::default()))
    }
}
