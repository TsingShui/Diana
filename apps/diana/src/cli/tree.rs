//! Package-tree presentation and CLI options over the shared class-query product.

use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;

use clap::Args;
use serde::Serialize;

use super::output::{Failure, Format, publish_with};
use crate::analysis::input::InputError;
use crate::analysis::request::RequestScope;
use crate::analysis::tree::{self, Package, PackageId, PackageTree};

#[derive(Args)]
pub(super) struct TreeArgs {
    /// Input file (APK, ZIP or bare DEX)
    input: PathBuf,
    /// Exact root package, in dotted or slash-separated form
    #[arg(long)]
    prefix: Option<String>,
    /// Visible package depth below the root; class leaves do not consume depth
    #[arg(short = 'L', long)]
    level: Option<NonZeroUsize>,
    /// Include original class definitions and their get-class locators
    #[arg(long)]
    classes: bool,
    /// Workers for independent DEX images (default: available CPUs)
    #[arg(long)]
    jobs: Option<NonZeroUsize>,
    /// Presentation of the same selected hierarchy
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// Write the result to this file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Serialize)]
struct Document<'a> {
    schema_version: u8,
    command: &'static str,
    complete: bool,
    tree: &'a PackageTree,
    issue: Option<Issue>,
}

#[derive(Serialize)]
struct Issue {
    kind: String,
    message: String,
}

pub(super) fn run(args: TreeArgs) -> Result<(), Failure> {
    let prefix = normalize_prefix(args.prefix.as_deref().unwrap_or(""))?;
    let scope = RequestScope::open(&args.input, &super::options()).map_err(Failure::input)?;
    let jobs = args
        .jobs
        .unwrap_or_else(|| NonZeroUsize::new(super::default_jobs()).unwrap_or(NonZeroUsize::MIN));
    let result = tree::build(&scope, &prefix, args.level, args.classes, jobs)
        .map_err(|e| Failure::query(e, 1))?;
    let document = Document {
        schema_version: 1,
        command: "tree",
        complete: result.issue.is_none(),
        tree: &result.tree,
        issue: result.issue.as_ref().map(|error| Issue {
            kind: format!("{:?}", error.kind()),
            message: error.to_string(),
        }),
    };
    publish_with(args.output.as_deref(), &args.input, |writer| {
        let mut writer = io::BufWriter::with_capacity(1 << 18, writer);
        match args.format {
            Format::Text => {
                write_tree(&mut writer, &result.tree).map_err(Failure::output)?;
                if let Some(issue) = &document.issue {
                    writeln!(writer, "[incomplete: {}: {}]", issue.kind, issue.message)
                        .map_err(Failure::output)?;
                }
            }
            Format::Json => serde_json::to_writer(&mut writer, &document)
                .map_err(|e| Failure::output(io::Error::other(e)))?,
        }
        writer.flush().map_err(Failure::output)
    })?;
    match result.issue {
        Some(error) => Err(Failure::query(error, 1)),
        None => Ok(()),
    }
}

fn normalize_prefix(value: &str) -> Result<Vec<u16>, Failure> {
    let value = value.trim_matches(['.', '/']);
    if value.split(['.', '/']).any(|part| part.is_empty()) && !value.is_empty() {
        return Err(Failure {
            code: 2,
            message: "package prefix contains an empty component".to_owned(),
        });
    }
    let mut units = Vec::new();
    units
        .try_reserve_exact(value.len())
        .map_err(|e| Failure::input(InputError::Allocation(e)))?;
    units.extend(value.encode_utf16().map(|unit| {
        if unit == u16::from(b'.') {
            u16::from(b'/')
        } else {
            unit
        }
    }));
    Ok(units)
}

struct Frame {
    package: PackageId,
    next: usize,
    total: usize,
}

fn width(package: &Package) -> io::Result<usize> {
    package
        .children
        .len()
        .checked_add(package.classes.len())
        .ok_or_else(|| io::Error::other("tree width is not representable"))
}

/// Iterative traversal: no call-stack depth depends on an input package path.
fn write_tree(writer: &mut impl Write, tree: &PackageTree) -> io::Result<()> {
    let root = &tree.packages[tree.root.0];
    writeln!(writer, "{} ({} classes)", root.name, root.class_count)?;
    let mut stack = Vec::new();
    stack.try_reserve(1).map_err(io::Error::other)?;
    stack.push(Frame {
        package: tree.root,
        next: 0,
        total: width(root)?,
    });
    while let Some(frame) = stack.last_mut() {
        if frame.next == frame.total {
            stack.pop();
            continue;
        }
        let package = frame.package;
        let index = frame.next;
        frame.next += 1;
        let last = frame.next == frame.total;
        for parent in &stack[..stack.len() - 1] {
            writer.write_all(if parent.next < parent.total {
                "│   ".as_bytes()
            } else {
                b"    "
            })?;
        }
        writer.write_all(if last {
            "└── ".as_bytes()
        } else {
            "├── ".as_bytes()
        })?;
        let node = &tree.packages[package.0];
        if let Some(&child) = node.children.get(index) {
            let child_node = &tree.packages[child.0];
            writeln!(
                writer,
                "{} ({} classes)",
                child_node.name, child_node.class_count
            )?;
            stack.try_reserve(1).map_err(io::Error::other)?;
            stack.push(Frame {
                package: child,
                next: 0,
                total: width(child_node)?,
            });
        } else {
            let leaf = &node.classes[index - node.children.len()];
            writeln!(writer, "{}.class  {}", leaf.name, leaf.at)?;
        }
    }
    Ok(())
}
