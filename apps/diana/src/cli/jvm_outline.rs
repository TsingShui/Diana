//! Presentation of JVM call sites and followable original definitions.

use std::io::Write;

use serde::Serialize;

use super::outline::OutlineArgs;
use super::output::{Failure, Format, publish_with};
use super::table::encode_cell;
use crate::analysis::jvm::{JvmRoots, Location};
use crate::analysis::jvm_code::method_at;
use crate::analysis::jvm_outline::{self, Outline};
use crate::analysis::jvm_source;
use crate::analysis::query::QueryError;
use crate::analysis::request::RequestScope;
use crate::analysis::selector::{self, SelectorError};

#[derive(Serialize)]
struct Publication<'a> {
    schema_version: u32,
    command: &'static str,
    #[serde(flatten)]
    outline: &'a Outline,
}

pub(super) fn run(
    scope: &RequestScope,
    args: &OutlineArgs,
    class: bool,
    jobs: usize,
) -> Result<(), Failure> {
    let text = &args.selector;
    let depth = (!args.expand_all).then_some(args.call_depth.unwrap_or(0));
    let prefix = text
        .split_once(':')
        .map_or(text.as_str(), |(prefix, _)| prefix);
    let lookup_jobs = super::default_jobs();
    let (roots, location, method, issues) = if let Ok(location) = prefix.parse::<Location>() {
        let method = if class {
            if text != prefix {
                return Err(selection_failure(SelectorError::Syntax(
                    "JVM class locator names only a class",
                )));
            }
            None
        } else {
            Some(method_at(text).map_err(selection_failure)?.1)
        };
        let roots = if depth == Some(0) {
            JvmRoots::focused(scope, &location).map_err(query_failure)?
        } else {
            full_roots(scope, lookup_jobs)?
        };
        (roots, location, method, Vec::new())
    } else {
        let method = (!class)
            .then(|| selector::method(text))
            .transpose()
            .map_err(selection_failure)?;
        let owner = match &method {
            Some(method) => &method.class,
            None => &selector::class(text).map_err(selection_failure)?,
        };
        if owner.class_def.is_some() {
            return Err(selection_failure(SelectorError::Syntax(
                "JVM input requires a JVM locator",
            )));
        }
        let roots = full_roots(scope, lookup_jobs)?;
        let (location, mut issues) = jvm_source::find_class(&roots, &owner.descriptor, lookup_jobs)
            .map_err(retrieval_failure)?;
        let ordinal = if let Some(method) = method {
            let mut reader = roots.reader();
            let entry = roots
                .by_location(&location)
                .ok_or_else(|| selection_failure(SelectorError::InvalidIndex))?;
            let class = reader.read(entry).map_err(query_failure)?;
            let (ordinal, method_issues) =
                jvm_source::find_method(&class, &location, &method).map_err(retrieval_failure)?;
            issues.extend(method_issues);
            Some(ordinal)
        } else {
            None
        };
        let issues = issues
            .into_iter()
            .map(jvm_source::Issue::into_reference_issue)
            .collect();
        (roots, location, ordinal, issues)
    };
    let command = if class {
        "outline-class"
    } else {
        "outline-method"
    };
    let outline = jvm_outline::outline(&roots, location, method, depth, jobs, lookup_jobs, issues)
        .map_err(query_failure)?;
    publish_with(args.output.as_deref(), &args.input, |writer| {
        match args.format {
            Format::Json => serde_json::to_writer(
                writer,
                &Publication {
                    schema_version: 1,
                    command,
                    outline: &outline,
                },
            )
            .map_err(|error| Failure::output(std::io::Error::other(error))),
            Format::Text => {
                writeln!(writer, "{}", outline.root).map_err(Failure::output)?;
                for member in &outline.members {
                    write!(writer, "{} {}", member.kind, member.at).map_err(Failure::output)?;
                    if let Some(symbol) = &member.symbol {
                        writer.write_all(b" ").map_err(Failure::output)?;
                        writer
                            .write_all(&encode_cell(symbol, args.format)?)
                            .map_err(Failure::output)?;
                    }
                    writeln!(writer).map_err(Failure::output)?;
                }
                for call in &outline.calls {
                    write!(
                        writer,
                        "{}@{} {} ",
                        call.caller, call.offset_byte, call.mnemonic
                    )
                    .map_err(Failure::output)?;
                    writer
                        .write_all(&encode_cell(&call.symbol, args.format)?)
                        .map_err(Failure::output)?;
                    if let Some(target) = &call.target {
                        write!(writer, " [{target}]").map_err(Failure::output)?;
                    }
                    writeln!(writer, " {} depth {}", call.resolution, call.depth)
                        .map_err(Failure::output)?;
                }
                for issue in &outline.issues {
                    writer.write_all(b"! ").map_err(Failure::output)?;
                    for (index, cell) in [&issue.context, &issue.kind, &issue.reason]
                        .into_iter()
                        .enumerate()
                    {
                        if index != 0 {
                            writer.write_all(b": ").map_err(Failure::output)?;
                        }
                        writer
                            .write_all(&encode_cell(cell, args.format)?)
                            .map_err(Failure::output)?;
                    }
                    if let Some([start, end]) = issue.range_bytes {
                        write!(writer, " bytes {start}..{end}").map_err(Failure::output)?;
                    }
                    writeln!(writer).map_err(Failure::output)?;
                }
                Ok(())
            }
        }
    })?;
    if outline.issues.is_empty() {
        Ok(())
    } else {
        Err(Failure {
            code: 1,
            message: "outline contains local unavailability; recovered content was published"
                .to_owned(),
        })
    }
}

fn full_roots<'a>(scope: &'a RequestScope, jobs: usize) -> Result<JvmRoots<'a>, Failure> {
    JvmRoots::select_with_jobs(scope, jobs)
        .map_err(query_failure)?
        .ok_or_else(|| query_failure(QueryError::WorkerLost))
}

fn query_failure(error: QueryError) -> Failure {
    Failure::query(error, 1)
}

fn selection_failure(error: SelectorError) -> Failure {
    Failure {
        code: error.code(),
        message: error.to_string(),
    }
}

fn retrieval_failure(error: jvm_source::RetrievalError) -> Failure {
    Failure {
        code: error.code(),
        message: error.to_string(),
    }
}
