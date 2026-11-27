//! Deliver the same Engine publication in either presentation.

use std::io::Write;
use std::path::Path;

use serde::Serialize;

use super::output::{Failure, Format, publish_with};
use crate::analysis::decompile::{self, DecompileIssue};
use crate::analysis::jvm_source;
use crate::analysis::selector::{self, Selector};

pub enum Kind {
    Class { jobs: u32 },
    Method,
}

const SCHEMA_VERSION: u32 = 5;

#[derive(Serialize)]
struct Publication<'a, T> {
    schema_version: u32,
    command: &'static str,
    #[serde(flatten)]
    analysis: &'a T,
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
    let scope = super::open_scope(input, output)?;
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
    let jvm = jvm_source::run_if_jvm(&scope, text, matches!(kind, Kind::Method), jobs).map_err(
        |error| Failure {
            code: error.code(),
            message: error.to_string(),
        },
    )?;
    let result = if let Some(decompiled) = jvm {
        let incomplete = decompiled.issues.first().map(|issue| Failure {
            code: 1,
            message: format!(
                "semantic recovery incomplete at {}: {}",
                issue.context, issue.reason
            ),
        });
        publish(
            input,
            output,
            format,
            command,
            &decompiled,
            &decompiled.result.text,
        )
        .and_then(|()| incomplete.map_or(Ok(()), Err))
    } else {
        let selector = match kind {
            Kind::Class { .. } => selector::class(text).map(Selector::Class),
            Kind::Method => selector::method(text).map(Selector::Method),
        }
        .map_err(|error| Failure {
            code: error.code(),
            message: error.to_string(),
        })?;
        let decompiled =
            decompile::run_with_jobs(&scope, &selector, jobs).map_err(|error| Failure {
                code: error.code(),
                message: error.to_string(),
            })?;
        let incomplete = incomplete_failure(&decompiled.issues);
        publish(
            input,
            output,
            format,
            command,
            &decompiled,
            decompiled.rendered.text(),
        )
        .and_then(|()| incomplete.map_or(Ok(()), Err))
    };
    #[cfg(feature = "perf")]
    {
        crate::analysis::perf::add_elapsed(&crate::analysis::perf::REQUEST_NS, request_started);
        crate::analysis::perf::print(command);
    }
    result
}

fn publish<T: Serialize>(
    input: &Path,
    output: Option<&Path>,
    format: Format,
    command: &'static str,
    analysis: &T,
    text: &str,
) -> Result<(), Failure> {
    #[cfg(feature = "perf")]
    let _timer = crate::analysis::perf::Timer::new(&crate::analysis::perf::PUBLISH_NS);
    publish_with(output, input, |writer| match format {
        Format::Text => writer.write_all(text.as_bytes()).map_err(|error| Failure {
            code: 1,
            message: error.to_string(),
        }),
        Format::Json => serde_json::to_writer(
            writer,
            &Publication {
                schema_version: SCHEMA_VERSION,
                command,
                analysis,
            },
        )
        .map_err(|error| Failure {
            code: 1,
            message: format!("JSON output failed: {error}"),
        }),
    })
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
