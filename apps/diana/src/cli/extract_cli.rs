//! `extract`: one physical entry, binary XML decoded by default.

use std::path::Path;

use super::output::Failure;
use crate::analysis::extract::{self, ExtractError, Request};
use crate::analysis::jvm::Location;

#[allow(clippy::too_many_arguments)]
pub fn run(
    input: &Path,
    entry: &str,
    index: Option<usize>,
    at: bool,
    output: &Path,
    raw: bool,
    force: bool,
    strict: bool,
) -> Result<(), Failure> {
    let scope = super::open_scope(input, Some(output))?;
    if at {
        let at = entry.parse::<Location>().map_err(|error| Failure {
            code: 2,
            message: error.to_string(),
        })?;
        return extract::publish_jvm(&scope, &at, output, input, force).map_err(extract_failure);
    }
    let extracted = extract::prepare(
        &scope,
        &Request {
            name: entry,
            index,
            raw,
        },
    )
    .map_err(extract_failure)?;
    for warning in &extracted.warnings {
        eprintln!(
            "diana: warning: byte {}: {}",
            warning.offset,
            warning.kind.describe()
        );
    }
    extract::publish(&extracted, output, input, force).map_err(extract_failure)?;
    if strict && !extracted.warnings.is_empty() {
        return Err(Failure {
            code: 1,
            message: "the decoded entry is partial and --strict was requested".to_owned(),
        });
    }
    Ok(())
}

fn extract_failure(error: ExtractError) -> Failure {
    Failure {
        code: error.code(),
        message: error.to_string(),
    }
}
