//! One compiled literal fuzzy pattern and thread-local nucleo matching scratch.

use std::cell::RefCell;

use diana_decomp::ByteOffset;
use diana_decomp::input::{DexString, InputError};
use nucleo_matcher::{Config, Matcher, Utf32Str, chars::to_lower_case};

use super::{FilterError, Relevance};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Needle {
    Ascii(Vec<u8>),
    Unicode(Vec<char>),
}

/// Case-insensitive fuzzy matching of one literal pattern. No operator parsing,
/// Unicode normalization or result sorting changes the original input identities.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FuzzyPattern {
    needle: Needle,
}

struct Scratch {
    matcher: Matcher,
    unicode: Vec<char>,
}

impl Scratch {
    fn new() -> Self {
        let mut config = Config::DEFAULT;
        config.normalize = false;
        config.ignore_case = true;
        config.prefer_prefix = false;
        Self {
            matcher: Matcher::new(config),
            unicode: Vec::new(),
        }
    }
}

thread_local! {
    // Matcher owns fixed library scratch. Sharing only within a thread avoids
    // allocating it for every reference and serializing independent DEX workers.
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch::new());
}

impl FuzzyPattern {
    /// Compiles using nucleo's simple Unicode case folding. Allocation failures
    /// and patterns outside the pinned matcher's score representation are errors.
    pub fn new(pattern: &str) -> Result<Self, FilterError> {
        let count = pattern.chars().count();
        // nucleo-matcher 0.3.1 score.rs/config.rs: match <=16, bonus <=10,
        // first bonus doubled, prefer_prefix disabled. Its u16 score additions
        // are unchecked, including on its long-input greedy path. This checked
        // representation bound prevents overflow; it is not a result limit.
        if count
            .checked_mul(26)
            .and_then(|score| score.checked_add(10))
            .is_none_or(|score| score > usize::from(u16::MAX))
        {
            return Err(FilterError::MatcherCapacity);
        }
        let needle = if pattern.is_ascii() {
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(pattern.len())
                .map_err(FilterError::Allocation)?;
            bytes.extend(pattern.bytes().map(|byte| byte.to_ascii_lowercase()));
            Needle::Ascii(bytes)
        } else {
            let mut chars = Vec::new();
            chars
                .try_reserve_exact(count)
                .map_err(FilterError::Allocation)?;
            chars.extend(pattern.chars().map(to_lower_case));
            if chars.iter().all(char::is_ascii) {
                let mut bytes = Vec::new();
                bytes
                    .try_reserve_exact(chars.len())
                    .map_err(FilterError::Allocation)?;
                // Each scalar was proved ASCII above.
                bytes.extend(chars.into_iter().map(|character| character as u8));
                Needle::Ascii(bytes)
            } else {
                Needle::Unicode(chars)
            }
        };
        Ok(Self { needle })
    }

    /// Fuzzy matching uses the reader's scalar text projection, like regex;
    /// exact queries and published values retain the original UTF-16 units.
    pub(crate) fn evaluate(&self, value: DexString<'_>) -> Result<Option<Relevance>, InputError> {
        self.evaluate_text(&value.utf8_lossy()?)
    }

    pub(super) fn evaluate_text(&self, value: &str) -> Result<Option<Relevance>, InputError> {
        let needle = match &self.needle {
            Needle::Ascii(bytes) => Utf32Str::Ascii(bytes),
            Needle::Unicode(chars) => Utf32Str::Unicode(chars),
        };
        if needle.is_empty() {
            return Ok(Some(Relevance::default()));
        }
        let count = if value.is_ascii() {
            value.len()
        } else {
            value.chars().count()
        };
        let length = u32::try_from(count).map_err(|_| InputError::PlatformCapacity {
            offset_byte: ByteOffset(0),
        })?;
        SCRATCH.with(|scratch| {
            let mut scratch = scratch
                .try_borrow_mut()
                .map_err(|_| InputError::InternalInvariant("fuzzy matcher reentered"))?;
            let Scratch { matcher, unicode } = &mut *scratch;
            let haystack = if value.is_ascii() {
                Utf32Str::Ascii(value.as_bytes())
            } else {
                unicode.clear();
                unicode
                    .try_reserve(count)
                    .map_err(|_| InputError::Allocation {
                        offset_byte: ByteOffset(0),
                    })?;
                unicode.extend(value.chars());
                Utf32Str::Unicode(unicode)
            };
            Ok(matcher
                .fuzzy_match(haystack, needle)
                .map(|score| Relevance {
                    score: u32::from(score),
                    length: u64::from(length),
                }))
        })
    }
}
