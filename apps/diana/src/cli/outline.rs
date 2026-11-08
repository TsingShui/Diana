//! Text/JSON presentations of one finite outline arena.
use super::output::{Failure, Format, publish_with};
use crate::analysis::{
    outline::{self, Document, Expansion},
    request::RequestScope,
    selector::{self, Selector},
};
use clap::Args;
use serde::Serialize;
use std::{io::Write, path::PathBuf};

#[derive(Args)]
pub struct OutlineArgs {
    /// Input APK, ZIP or bare DEX
    input: PathBuf,
    /// Original locator or exact class/method reference
    selector: String,
    /// Number of cross-method levels (default: current members only)
    #[arg(long, conflicts_with = "expand_all")]
    call_depth: Option<usize>,
    /// Expand all available referenced declaration bodies once; cycles are links
    #[arg(long)]
    expand_all: bool,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    #[arg(short, long)]
    output: Option<PathBuf>,
}
#[derive(Args)]
pub struct ClassArgs {
    #[command(flatten)]
    outline: OutlineArgs,
    /// Parallel method workers for the selected class (default: 1)
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    jobs: Option<u32>,
}

#[derive(Serialize)]
struct Publication<'a> {
    schema_version: u32,
    command: &'static str,
    #[serde(flatten)]
    outline: &'a Document,
}

pub fn class(args: ClassArgs) -> Result<(), Failure> {
    let jobs = usize::try_from(args.jobs.unwrap_or(1)).map_err(|_| Failure {
        code: 2,
        message: "method worker count is not representable".to_owned(),
    })?;
    run(args.outline, true, jobs)
}
pub fn method(args: OutlineArgs) -> Result<(), Failure> {
    run(args, false, 1)
}
fn run(args: OutlineArgs, class: bool, jobs: usize) -> Result<(), Failure> {
    let selected = if class {
        selector::class(&args.selector).map(Selector::Class)
    } else {
        selector::method(&args.selector).map(Selector::Method)
    }
    .map_err(|e| Failure {
        code: e.code(),
        message: e.to_string(),
    })?;
    let scope = RequestScope::open(&args.input, &super::options()).map_err(Failure::input)?;
    let expansion = if args.expand_all {
        Expansion::All
    } else {
        Expansion::Depth(args.call_depth.unwrap_or(0))
    };
    let document = outline::run(&scope, &selected, expansion, jobs).map_err(|error| Failure {
        code: error.code(),
        message: error.to_string(),
    })?;
    publish_with(args.output.as_deref(), &args.input, |writer| {
        match args.format {
            Format::Text => write_text(writer, &document),
            Format::Json => serde_json::to_writer(
                writer,
                &Publication {
                    schema_version: 1,
                    command: if class {
                        "outline-class"
                    } else {
                        "outline-method"
                    },
                    outline: &document,
                },
            )
            .map_err(|error| Failure {
                code: 1,
                message: format!("JSON output failed: {error}"),
            }),
        }
    })?;
    if !document.complete {
        return Err(Failure {
            code: 1,
            message: "outline contains local unavailability; recovered content was published"
                .to_owned(),
        });
    }
    Ok(())
}

fn allocation(error: std::collections::TryReserveError) -> Failure {
    Failure {
        code: 1,
        message: format!("outline presentation allocation failed: {error}"),
    }
}
fn write_text(writer: &mut dyn Write, document: &Document) -> Result<(), Failure> {
    let mut children: Vec<Vec<usize>> = Vec::new();
    children
        .try_reserve_exact(document.nodes.len())
        .map_err(allocation)?;
    children.resize_with(document.nodes.len(), Vec::new);
    for (id, node) in document.nodes.iter().enumerate() {
        if let Some(parent) = node.parent {
            let children = children.get_mut(parent).ok_or_else(|| Failure {
                code: 1,
                message: "outline parent is unavailable".to_owned(),
            })?;
            children.try_reserve(1).map_err(allocation)?;
            children.push(id);
        }
    }
    let mut calls = Vec::new();
    calls
        .try_reserve_exact(document.nodes.len())
        .map_err(allocation)?;
    calls.resize(document.nodes.len(), None);
    for call in &document.calls {
        calls[call.node] = Some(call);
    }
    let mut work = Vec::new();
    work.try_reserve(1).map_err(allocation)?;
    work.push((document.root, 0usize, true));
    let mut continuation = Vec::new();
    while let Some((id, depth, last)) = work.pop() {
        let node = &document.nodes[id];
        if depth > 0 {
            for &more in continuation.iter().take(depth - 1) {
                writer
                    .write_all(if more { "│  ".as_bytes() } else { b"   " })
                    .map_err(Failure::output)?;
            }
            writer
                .write_all(if last {
                    "└─ ".as_bytes()
                } else {
                    "├─ ".as_bytes()
                })
                .map_err(Failure::output)?;
        }
        write!(writer, "{}", node.text).map_err(Failure::output)?;
        if node.kind == "method" || node.kind == "class" {
            write!(writer, " #{id}").map_err(Failure::output)?;
        }
        if let Some(at) = &node.at {
            write!(writer, " [{at}]").map_err(Failure::output)?;
        }
        if let Some(call) = calls[id] {
            write!(writer, " [{:?}; {}", call.dispatch, call.state).map_err(Failure::output)?;
            if call.origin_only {
                writer
                    .write_all(b"; origin-only attachment, not another execution")
                    .map_err(Failure::output)?;
            }
            if call.abi_attachment {
                writer
                    .write_all(b"; ABI attachment")
                    .map_err(Failure::output)?;
            }
            if let Some(body) = call.body {
                write!(writer, " → #{body}").map_err(Failure::output)?;
            }
            if let Some(uncertainty) = call.uncertainty {
                write!(writer, "; {uncertainty}").map_err(Failure::output)?;
            }
            writer.write_all(b"]").map_err(Failure::output)?;
        }
        writer.write_all(b"\n").map_err(Failure::output)?;
        continuation.truncate(depth.saturating_sub(1));
        if depth > 0 {
            continuation.try_reserve(1).map_err(allocation)?;
            continuation.push(!last);
        }
        let next_depth = depth.checked_add(1).ok_or_else(|| Failure {
            code: 1,
            message: "outline depth is not representable".to_owned(),
        })?;
        work.try_reserve(children[id].len()).map_err(allocation)?;
        for (index, &child) in children[id].iter().enumerate().rev() {
            work.push((child, next_depth, index + 1 == children[id].len()));
        }
    }
    for issue in &document.issues {
        write!(writer, "! ").map_err(Failure::output)?;
        if let Some(at) = &issue.at {
            write!(writer, "{at}: ").map_err(Failure::output)?;
        }
        writeln!(writer, "{}", issue.detail).map_err(Failure::output)?;
    }
    Ok(())
}
