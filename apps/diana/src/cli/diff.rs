use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::output::{Failure, Format, publish_for_inputs};
use crate::analysis::diff::{self, MethodLocation, Report, Site};
use crate::analysis::request::RequestScope;

#[derive(clap::Args)]
pub(super) struct Args {
    /// Original APK, DEX, JAR, JVM class or JVM project
    old: PathBuf,
    /// Updated APK, DEX, JAR, JVM class or JVM project
    new: PathBuf,
    /// Maximum concurrent input workers
    #[arg(long)]
    jobs: Option<NonZeroUsize>,
    /// Presentation of the same result
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// Write the comparison to this file
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Serialize)]
struct Inputs<'a> {
    old: &'a str,
    new: &'a str,
}

#[derive(Serialize)]
struct Document<'a> {
    command: &'static str,
    inputs: Inputs<'a>,
    #[serde(flatten)]
    report: &'a Report,
}

pub(super) fn run(args: Args) -> Result<(), Failure> {
    let old = open(&args.old, "old")?;
    let new = open(&args.new, "new")?;
    super::output::protect_project_targets(&old, args.output.as_deref())?;
    super::output::protect_project_targets(&new, args.output.as_deref())?;

    let jobs = args
        .jobs
        .map_or_else(super::default_jobs, NonZeroUsize::get);
    let report = diff::compare(&old, &new, jobs).map_err(|error| Failure {
        code: 1,
        message: error.to_string(),
    })?;
    let old_path = args.old.to_string_lossy();
    let new_path = args.new.to_string_lossy();
    let document = Document {
        command: "diff",
        inputs: Inputs {
            old: &old_path,
            new: &new_path,
        },
        report: &report,
    };
    publish_for_inputs(args.output.as_deref(), &[&args.old, &args.new], |writer| {
        match args.format {
            Format::Text => write_text(writer, &document),
            Format::Json => write_json(writer, &document),
        }
        .map_err(Failure::output)
    })?;
    if !report.issues.is_empty() {
        return Err(Failure {
            code: 1,
            message: format!("partial comparison with {} issue(s)", report.issues.len()),
        });
    }
    Ok(())
}

fn open(path: &Path, side: &str) -> Result<RequestScope, Failure> {
    RequestScope::open(path, &super::options()).map_err(|error| Failure {
        code: 1,
        message: format!("{side} input {}: {error}", path.display()),
    })
}

fn write_json(writer: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    serde_json::to_writer(writer, value).map_err(io::Error::from)
}

fn write_text(writer: &mut impl Write, document: &Document<'_>) -> io::Result<()> {
    let report = document.report;
    writeln!(
        writer,
        "diana {} (schema {})",
        document.command, report.schema
    )?;
    write!(writer, "old input ")?;
    write_json(writer, &document.inputs.old)?;
    write!(writer, "\nnew input ")?;
    write_json(writer, &document.inputs.new)?;
    writeln!(
        writer,
        "\n{} old methods, {} new methods, {} candidate pairs",
        report.summary.old_methods, report.summary.new_methods, report.summary.candidate_pairs,
    )?;
    if let Some(unit) = report.offset_unit {
        writeln!(writer, "method offset unit {unit}")?;
    }
    for pair in &report.matches {
        writeln!(writer, "\nmatch {}", pair.status)?;
        write_location(writer, "old", &pair.old)?;
        write_location(writer, "new", &pair.new)?;
        write!(writer, "  evidence ")?;
        write_json(writer, &pair.evidence)?;
        writeln!(writer)?;
        for change in &pair.changes {
            writeln!(writer, "  change {}", change.kind)?;
            write_site(writer, "old", change.old.as_ref())?;
            write_site(writer, "new", change.new.as_ref())?;
        }
    }
    for group in &report.unmatched {
        writeln!(writer, "\nunmatched group {}", group.status)?;
        for method in &group.old {
            write_location(writer, "old", method)?;
        }
        for method in &group.new {
            write_location(writer, "new", method)?;
        }
    }
    for fragment in &report.fragments {
        writeln!(
            writer,
            "\nfragment ({} instructions)",
            fragment.instruction_count
        )?;
        write_location(writer, "old", &fragment.old)?;
        writeln!(writer, "    offset {}", fragment.old_offset)?;
        write_location(writer, "new", &fragment.new)?;
        writeln!(writer, "    offset {}", fragment.new_offset)?;
    }
    for issue in &report.issues {
        write!(writer, "\nissue {}", issue.side)?;
        if let Some(at) = &issue.at {
            write!(writer, " at ")?;
            write_json(writer, at)?;
        }
        write!(writer, " ")?;
        write_json(writer, &issue.reason)?;
        writeln!(writer)?;
    }
    Ok(())
}

fn write_location(writer: &mut impl Write, side: &str, method: &MethodLocation) -> io::Result<()> {
    write!(writer, "  {side} ")?;
    write_json(writer, &method.signature)?;
    write!(writer, " at ")?;
    write_json(writer, &method.at)?;
    writeln!(writer)
}

fn write_site(writer: &mut impl Write, side: &str, site: Option<&Site>) -> io::Result<()> {
    if let Some(site) = site {
        write!(writer, "    {side} ")?;
        if let Some(offset) = site.offset {
            write!(writer, "@{offset} ")?;
        }
        write_json(writer, &site.text)?;
        writeln!(writer)?;
    }
    Ok(())
}
