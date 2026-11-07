//! `xref`: original operand reference sites, one row per site.

use std::path::Path;

use super::output::{Failure, Format};
use super::table::{RowSummary, publish_rows};
use crate::analysis::at;
use crate::analysis::input::{InputError, InputOptions};
use crate::analysis::request::RequestScope;
use crate::analysis::selector;
use crate::analysis::xref::{Matched, Target, Xref, XrefPlan};

/// Default presentation cap, shared by text and JSON across all caller depths.
pub(super) const DEFAULT_LIMIT: usize = 50;

#[allow(clippy::too_many_arguments)]
pub fn run(
    input: &Path,
    kind: &str,
    target: &str,
    class: Option<&str>,
    transitive: u32,
    limit: usize,
    jobs: usize,
    format: Format,
    output: Option<&Path>,
) -> Result<(), Failure> {
    let target = parse_target(kind, target)?;
    let class = match class {
        Some(text) => Some(
            selector::class(text)
                .map_err(|error| Failure {
                    code: error.code(),
                    message: error.to_string(),
                })?
                .descriptor,
        ),
        None => None,
    };
    let scope =
        RequestScope::open(input, &InputOptions { allowed_root: None }).map_err(Failure::input)?;
    let first = Xref::new(
        &scope,
        XrefPlan {
            targets: vec![target.clone()],
            class: class.clone(),
            depth: 1,
            jobs,
        },
    )
    .map_err(|error| Failure::query(error, 1))?;
    let images = first.images().to_vec();

    let mut rows = Vec::new();
    first
        .run(&mut |row| {
            rows.push(row);
            Ok(())
        })
        .map_err(|error| Failure::query(error, 1))?;
    drop(first);

    // A transitive level repeats the same scan for every method reached by the
    // previous level. The class filter applies to the queried references only:
    // a caller's caller is matched wherever it lives.
    if transitive > 0 {
        let mut frontier = Vec::new();
        for row in &rows {
            // The edge is "caller invokes the scanned target", so the next level
            // asks who invokes this caller: its own method identity.
            frontier.push((
                row.class_descriptor.clone(),
                row.method_name.clone(),
                row.method_prototype.clone(),
            ));
        }
        frontier.sort_unstable();
        frontier.dedup();
        for depth in 2..=transitive.max(1) {
            if frontier.is_empty() {
                break;
            }
            let targets: Vec<Target> = frontier
                .iter()
                .map(|(owner, name, prototype)| Target::Method {
                    owner: owner.clone(),
                    name: name.clone(),
                    prototype: prototype.clone(),
                })
                .collect();
            let scan = Xref::new(
                &scope,
                XrefPlan {
                    targets,
                    class: None,
                    depth,
                    jobs,
                },
            )
            .map_err(|error| Failure::query(error, 1))?;
            let mut found = Vec::new();
            scan.run(&mut |row| {
                found.push(row);
                Ok(())
            })
            .map_err(|error| Failure::query(error, 1))?;
            frontier.clear();
            for row in &found {
                frontier.push((
                    row.class_descriptor.clone(),
                    row.method_name.clone(),
                    row.method_prototype.clone(),
                ));
            }
            rows.extend(found);
        }
    }
    // Select the presentation once, after every requested caller depth was read.
    // Hidden rows still contribute to the next frontier and the exact total.
    let total = rows.len();
    let shown = if limit == 0 { total } else { total.min(limit) };
    let warning = (shown < total).then(|| {
        format!(
            "//Diana Warn: Too much results, only show top {shown} , use --no-limit or --limit N"
        )
    });
    let mut output_rows = Vec::new();
    output_rows
        .try_reserve_exact(shown)
        .map_err(|error| Failure::input(InputError::Allocation(error)))?;
    for row in rows.iter().take(shown) {
        let image = &images[row.image.0];
        let context = format!(
            "{}@cu:{:#06x}",
            at::method(
                image.entry.map(|entry| entry.0),
                row.logical.0,
                row.class_def.0,
                row.method.0,
                &row.class_descriptor,
                &row.method_name,
                &row.method_prototype,
            ),
            row.code_unit
        );
        output_rows.push(vec![
            context,
            matched_text(&row.matched),
            format!("{} depth {}", row.use_, row.depth),
        ]);
    }
    publish_rows(
        &output_rows,
        &["context", "match", "use"],
        "xref",
        format,
        input,
        output,
        RowSummary {
            total,
            warning: warning.as_deref(),
        },
    )
}

fn parse_target(kind: &str, text: &str) -> Result<Target, Failure> {
    let invalid = |message: String| Failure { code: 2, message };
    match kind {
        "string" => Ok(Target::String(text.encode_utf16().collect())),
        "type" => Ok(Target::Type(
            selector::class(text)
                .map_err(|error| invalid(error.to_string()))?
                .descriptor,
        )),
        "method" => {
            let selector = selector::method(text).map_err(|error| invalid(error.to_string()))?;
            Ok(Target::Method {
                owner: selector.class.descriptor,
                name: selector.name,
                prototype: selector.prototype,
            })
        }
        "field" => {
            let selector = selector::field(text).map_err(|error| invalid(error.to_string()))?;
            Ok(Target::Field {
                owner: selector.class.descriptor,
                name: selector.name,
                field_type: selector.field_type,
            })
        }
        other => Err(invalid(format!(
            "unknown xref kind {other:?}; expected string|type|method|field"
        ))),
    }
}

fn matched_text(matched: &Matched) -> String {
    match matched {
        Matched::String(units) | Matched::Type(units) => at::escape_units(units),
        Matched::Method {
            owner,
            name,
            prototype,
        } => format!(
            "{}->{}{}",
            at::escape_units(owner),
            at::escape_units(name),
            at::escape_units(prototype)
        ),
        Matched::Field {
            owner,
            name,
            field_type,
        } => format!(
            "{}->{}:{}",
            at::escape_units(owner),
            at::escape_units(name),
            at::escape_units(field_type)
        ),
    }
}
