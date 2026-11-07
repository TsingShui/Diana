//! Shared CLI switches for semantic text matching.

use clap::Args;

use super::output::Failure;
use crate::analysis::filter::{FilterError, MatchMode, TextFilter};

#[derive(Args, Clone, Copy, Debug, Default)]
#[group(id = "match_mode", multiple = false)]
pub(super) struct MatchArgs {
    /// Case-insensitive contiguous substring
    #[arg(long)]
    substring: bool,
    /// Case-insensitive fuzzy subsequence, ranked by relevance (default)
    #[arg(long)]
    fuzzy: bool,
    /// Case-insensitive Rust regular expression; (?-i) enables case sensitivity
    #[arg(long)]
    regex: bool,
    /// Complete value equality, preserving case
    #[arg(long)]
    exact: bool,
}

impl MatchArgs {
    pub(super) fn mode(self) -> MatchMode {
        if self.substring {
            MatchMode::Substring
        } else if self.regex {
            MatchMode::Regex
        } else if self.exact {
            MatchMode::Exact
        } else {
            MatchMode::default()
        }
    }

    pub(super) fn filter(self, pattern: Option<&str>) -> Result<Option<TextFilter>, Failure> {
        match pattern {
            Some(pattern) => Ok(Some(
                TextFilter::compile(pattern, self.mode()).map_err(failure)?,
            )),
            None if self.substring || self.fuzzy || self.regex || self.exact => Err(Failure {
                code: 2,
                message: "matching mode switches require --filter".to_owned(),
            }),
            None => Ok(None),
        }
    }
}

pub(super) fn failure(error: FilterError) -> Failure {
    Failure {
        code: if error.kind() == diana_decomp::FailureKind::Unsupported {
            2
        } else {
            1
        },
        message: error.to_string(),
    }
}
