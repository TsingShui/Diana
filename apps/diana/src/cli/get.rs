//! Deliver the same Engine publication in either presentation.

use std::path::Path;

use serde::Serialize;

use super::output::{Failure, Format, publish_with};
use crate::analysis::decompile::{self, DecompileIssue, Decompiled};
use crate::analysis::request::RequestScope;
use crate::analysis::selector::{self, Selector};

pub enum Kind {
    Class { jobs: u32 },
    Method,
}

const SCHEMA_VERSION: u32 = 5;

#[derive(Serialize)]
struct Publication<'a> {
    schema_version: u32,
    command: &'static str,
    #[serde(flatten)]
    analysis: &'a Decompiled,
}

pub fn run(
    input: &Path,
    text: &str,
    kind: Kind,
    format: Format,
    output: Option<&Path>,
) -> Result<(), Failure> {
    #[cfg(feature = "perf")]
    crate::analysis::perf::reset();
    #[cfg(feature = "perf")]
    let request_started = std::time::Instant::now();
    let parsed = match kind {
        Kind::Class { .. } => selector::class(text).map(Selector::Class),
        Kind::Method => selector::method(text).map(Selector::Method),
    };
    let selector = parsed.map_err(|error| Failure {
        code: error.code(),
        message: error.to_string(),
    })?;
    let scope = RequestScope::open(input, &super::options()).map_err(Failure::input)?;
    let (jobs, command) = match kind {
        Kind::Class { jobs } => (
            usize::try_from(jobs).map_err(|_| Failure {
                code: 2,
                message: "method worker count is not representable on this platform".to_owned(),
            })?,
            "get-class",
        ),
        Kind::Method => (1, "get-method"),
    };
    let decompiled =
        decompile::run_with_jobs(&scope, &selector, jobs).map_err(|error| Failure {
            code: error.code(),
            message: error.to_string(),
        })?;
    let incomplete = incomplete_failure(&decompiled.issues);
    let result = {
        #[cfg(feature = "perf")]
        let _timer = crate::analysis::perf::Timer::new(&crate::analysis::perf::PUBLISH_NS);
        publish_with(output, input, |mut writer| match format {
            Format::Text => decompiled
                .rendered
                .write_text(&mut writer)
                .map_err(|error| Failure {
                    code: 1,
                    message: error.to_string(),
                }),
            Format::Json => serde_json::to_writer(
                writer,
                &Publication {
                    schema_version: SCHEMA_VERSION,
                    command,
                    analysis: &decompiled,
                },
            )
            .map_err(|error| Failure {
                code: 1,
                message: format!("JSON output failed: {error}"),
            }),
        })
    };
    #[cfg(feature = "perf")]
    {
        crate::analysis::perf::add_elapsed(&crate::analysis::perf::REQUEST_NS, request_started);
        crate::analysis::perf::print(command);
    }
    match result {
        Ok(()) => incomplete.map_or(Ok(()), Err),
        Err(error) => Err(error),
    }
}

fn incomplete_failure(issues: &[DecompileIssue]) -> Option<Failure> {
    issues.first().map(|issue| Failure {
        code: 1,
        message: format!(
            "semantic recovery incomplete for {} method(s): {}{}: {}",
            issues.len(),
            issue.name,
            issue.prototype,
            issue.reason
        ),
    })
}
