//! `xref`: original operand sites and direct declaration relations.

use std::path::Path;

use super::matching;
use super::output::{Failure, Format};
use super::table::{RowSummary, publish_encoded};
use crate::analysis::at;
use crate::analysis::filter::{MatchMode, best_first};
use crate::analysis::input::{InputError, InputOptions};
use crate::analysis::query::QueryError;
use crate::analysis::request::RequestScope;
use crate::analysis::selector;
use crate::analysis::xref::{Matched, Relation, RowView, Site, Target, TextMatch, Xref, XrefPlan};

mod render;
mod rows;
use rows::Rows;

/// Default presentation cap, shared by text and JSON across all caller depths.
pub(super) const DEFAULT_LIMIT: usize = 50;

#[cfg_attr(feature = "perf", fastrace::trace(name = "xref.request"))]
#[allow(clippy::too_many_arguments)]
pub fn run(
    input: &Path,
    kind: &str,
    target: &str,
    mode: MatchMode,
    recursive: bool,
    typed_uses: bool,
    method_relation: crate::analysis::xref::MethodRelation,
    class: Option<&str>,
    transitive: u32,
    limit: usize,
    jobs: usize,
    format: Format,
    output: Option<&Path>,
) -> Result<(), Failure> {
    if recursive && kind != "impl" && kind != "extend" {
        return Err(invalid("--recursive requires impl or extend"));
    }
    if method_relation != crate::analysis::xref::MethodRelation::References && kind != "method" {
        return Err(invalid("--relation requires method"));
    }
    if transitive > 0 && method_relation != crate::analysis::xref::MethodRelation::References {
        return Err(invalid(
            "caller expansion cannot be combined with method relation selection",
        ));
    }
    if transitive > 0 && kind != "method" {
        return Err(invalid(
            "--transitive follows callers and requires the method kind",
        ));
    }
    let coordinate = if kind == "resource" || mode == MatchMode::Exact && kind == "string" {
        None
    } else {
        selector::locator(target).map_err(selection_failure)?
    };
    let target = match coordinate {
        Some(coordinate) => QueryTarget::Coordinate(coordinate),
        None if kind == "resource" => QueryTarget::Resource(target.to_owned()),
        None => QueryTarget::Pattern(parse_target(kind, target, mode)?),
    };
    let class = class
        .map(selector::class)
        .transpose()
        .map_err(selection_failure)?
        .map(|selector| selector.descriptor);
    let scope = {
        #[cfg(feature = "perf")]
        let _span = fastrace::local::LocalSpan::enter_with_local_parent("xref.open-input");
        RequestScope::open(input, &InputOptions { allowed_root: None }).map_err(Failure::input)?
    };
    let mut issues = Vec::new();
    let target = match target {
        QueryTarget::Resource(pattern) => {
            use crate::analysis::resources::{Catalog, ResourceTarget, numeric_id};
            let numeric = numeric_id(&pattern);
            match Catalog::read(&scope) {
                Ok(catalog) => {
                    let missing =
                        numeric.is_some_and(|id| !catalog.rows.iter().any(|row| row.id == id));
                    let target = catalog.target(&pattern, mode).map_err(query_failure)?;
                    issues = catalog.issues;
                    if missing {
                        issues
                            .try_reserve(1)
                            .map_err(|error| Failure::input(InputError::Allocation(error)))?;
                        issues.push(crate::analysis::xref::Issue::unknown(
                            format!("resource:{pattern}"),
                            "resource name absent from input; matching literal candidates only",
                        ));
                    }
                    Target::Resource(target)
                }
                Err(error) => {
                    let Some(id) = numeric else {
                        return Err(query_failure(error));
                    };
                    issues
                        .try_reserve(1)
                        .map_err(|error| Failure::input(InputError::Allocation(error)))?;
                    issues.push(crate::analysis::xref::Issue::new(
                        format!("resource:{pattern}"),
                        &error,
                        None,
                    ));
                    Target::Resource(ResourceTarget::unresolved(id).map_err(query_failure)?)
                }
            }
        }
        QueryTarget::Coordinate(coordinate) => {
            crate::analysis::xref::resolve_coordinate(&scope, kind, &coordinate)
                .map_err(selection_failure)?
        }
        QueryTarget::Pattern(target) => target,
    };
    let ranked = target.is_ranked();
    let first = Xref::new(
        &scope,
        XrefPlan {
            recursive,
            method_relation,
            targets: vec![target],
            class,
            depth: 1,
            jobs,
            typed_uses,
            retain_payloads: transitive > 1,
        },
    )
    .map_err(query_failure)?;
    let images = first.images().to_vec();
    let mut rows = Rows::default();
    first
        .run(&mut |event| rows.collect(&mut issues, event))
        .map_err(query_failure)?;
    drop(first);

    // Only the initial references use the class restriction and query predicate.
    // Caller expansion always asks for complete original method identities.
    if transitive > 1 {
        let mut frontier = caller_targets(&rows, 0).map_err(query_failure)?;
        for depth in 2..=transitive {
            if frontier.is_empty() {
                break;
            }
            let scan = Xref::new(
                &scope,
                XrefPlan {
                    recursive: false,
                    method_relation: crate::analysis::xref::MethodRelation::References,
                    targets: frontier,
                    class: None,
                    depth,
                    jobs,
                    typed_uses: false,
                    retain_payloads: true,
                },
            )
            .map_err(query_failure)?;
            let start = rows.len();
            scan.run(&mut |event| rows.collect(&mut issues, event))
                .map_err(query_failure)?;
            frontier = if depth < transitive {
                caller_targets(&rows, start).map_err(query_failure)?
            } else {
                Vec::new()
            };
        }
    }
    for issue in &mut issues {
        if let Some(scope) = issue.context.strip_prefix("image:") {
            let (ordinal, suffix) = scope
                .split_once(':')
                .map_or((scope, ""), |(ordinal, suffix)| (ordinal, suffix));
            if let Ok(ordinal) = ordinal.parse::<usize>()
                && let Some(image) = images.get(ordinal)
            {
                issue.context = format!(
                    "e{}{}{}",
                    image.entry.map_or(0, |entry| entry.0),
                    if suffix.is_empty() { "" } else { ":" },
                    suffix
                );
            }
        }
    }
    if ranked {
        rows.rank().map_err(query_failure)?;
    }
    // Hidden rows still contribute to the next frontier and the exact total.
    let total = rows.len();
    let shown = if limit == 0 { total } else { total.min(limit) };
    let warning = (shown < total).then(|| {
        format!(
            "//Diana Warn: Too much results, only show top {shown} , use --no-limit or --limit N"
        )
    });
    publish_encoded(
        shown,
        &["context", "match", "use"],
        "xref",
        format,
        input,
        output,
        RowSummary {
            total,
            warning: warning.as_deref(),
            issues: &issues,
        },
        |writer| render::write(&rows, shown, &images, jobs, format, writer),
    )
}

/// Identity behind one rendered locator: the class, or one member of it.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum ContextKey {
    Class {
        image: usize,
        logical: u32,
        class_def: u32,
    },
    Method {
        image: usize,
        logical: u32,
        class_def: u32,
        method: u32,
    },
    Field {
        image: usize,
        logical: u32,
        class_def: u32,
        field: u32,
    },
}

fn context_key(row: RowView<'_>) -> ContextKey {
    let image = row.image.0;
    let logical = row.logical.0;
    let class_def = row.class_def.0;
    match row.site {
        Site::Class => ContextKey::Class {
            image,
            logical,
            class_def,
        },
        Site::UnknownField(id) => ContextKey::Field {
            image,
            logical,
            class_def,
            field: id.0,
        },
        Site::Field { field, .. } => ContextKey::Field {
            image,
            logical,
            class_def,
            field: field.0,
        },
        Site::UnknownMethod(id) => ContextKey::Method {
            image,
            logical,
            class_def,
            method: id.0,
        },
        Site::Method { method, .. } | Site::Code { method, .. } => ContextKey::Method {
            image,
            logical,
            class_def,
            method: method.0,
        },
    }
}

fn render_context(row: RowView<'_>, images: &[crate::analysis::query::ImageInfo]) -> String {
    let image = &images[row.image.0];
    let entry = image.entry.map(|entry| entry.0);
    match row.site {
        Site::Class => at::class(entry, row.logical.0, row.class_def.0, row.class_descriptor),
        Site::UnknownField(id) => format!(
            "e{}:l{}:c{}:f{}",
            entry.unwrap_or(0),
            row.logical.0,
            row.class_def.0,
            id.0
        ),
        Site::UnknownMethod(id) => format!(
            "e{}:l{}:c{}:m{}",
            entry.unwrap_or(0),
            row.logical.0,
            row.class_def.0,
            id.0
        ),
        Site::Field {
            field,
            name,
            field_type,
        } => at::field(
            entry,
            row.logical.0,
            row.class_def.0,
            field.0,
            row.class_descriptor,
            name,
            field_type,
        ),
        Site::Method {
            method,
            name,
            prototype,
        } => at::method(
            entry,
            row.logical.0,
            row.class_def.0,
            method.0,
            row.class_descriptor,
            name,
            prototype,
        ),
        Site::Code {
            method,
            name,
            prototype,
            ..
        } => at::method(
            entry,
            row.logical.0,
            row.class_def.0,
            method.0,
            row.class_descriptor,
            name,
            prototype,
        ),
    }
}

fn row_order(row: RowView<'_>) -> (usize, u32, u32, u32, u32) {
    let (method, code_unit) = match row.site {
        Site::Class => (0, 0),
        Site::UnknownField(id) => (id.0, 0),
        Site::UnknownMethod(id) => (id.0, 0),
        Site::Field { field, .. } => (field.0, 0),
        Site::Method { method, .. } => (method.0, 0),
        Site::Code {
            method, code_unit, ..
        } => (method.0, *code_unit),
    };
    (
        row.image.0,
        row.logical.0,
        row.class_def.0,
        method,
        row.code_unit.unwrap_or(code_unit),
    )
}

enum QueryTarget {
    Resource(String),
    Coordinate(selector::Selector),
    Pattern(Target),
}

fn caller_targets(rows: &Rows, start: usize) -> Result<Vec<Target>, QueryError> {
    let mut callers = Vec::new();
    callers
        .try_reserve_exact(rows.len() - start)
        .map_err(InputError::Allocation)?;
    for row in rows.iter().skip(start) {
        if let Some(key) = row.caller_key() {
            callers.push((key, row));
        }
    }
    callers.sort_unstable_by(|(left, left_row), (right, right_row)| {
        left.cmp(right)
            .then_with(|| best_first(left_row.relevance, right_row.relevance))
    });
    callers.dedup_by_key(|(key, _)| *key);
    let mut targets = Vec::new();
    targets
        .try_reserve_exact(callers.len())
        .map_err(InputError::Allocation)?;
    for (_, row) in callers {
        if let Some(target) = row.caller()? {
            targets.push(target);
        }
    }
    Ok(targets)
}

fn invalid(message: impl Into<String>) -> Failure {
    Failure {
        code: 2,
        message: message.into(),
    }
}
fn selection_failure(error: selector::SelectorError) -> Failure {
    Failure {
        code: error.code(),
        message: error.to_string(),
    }
}
fn query_failure(error: QueryError) -> Failure {
    Failure::query(error, 1)
}

fn text_match(text: &str, mode: MatchMode) -> Result<TextMatch, Failure> {
    TextMatch::compile(text, mode).map_err(matching::failure)
}

fn type_match(text: &str, mode: MatchMode) -> Result<TextMatch, Failure> {
    if mode == MatchMode::Exact {
        Ok(TextMatch::Exact(
            selector::class(text).map_err(selection_failure)?.descriptor,
        ))
    } else {
        if text.is_empty() {
            return Err(invalid("empty type/owner pattern"));
        }
        // Regex punctuation is syntax; literal dotted fragments are descriptors.
        if mode == MatchMode::Regex {
            text_match(text, mode)
        } else {
            text_match(&text.replace('.', "/"), mode)
        }
    }
}

fn parse_target(kind: &str, text: &str, mode: MatchMode) -> Result<Target, Failure> {
    match kind {
        "string" => Ok(Target::String(text_match(text, mode)?)),
        "type" => Ok(Target::Type(type_match(text, mode)?)),
        "impl" | "extend" => Ok(Target::Hierarchy {
            relation: if kind == "impl" {
                Relation::Impl
            } else {
                Relation::Extend
            },
            target: type_match(text, mode)?,
        }),
        "method" if mode == MatchMode::Exact => {
            let selector = selector::method(text).map_err(selection_failure)?;
            Ok(Target::Method {
                owner: Some(TextMatch::Exact(selector.class.descriptor)),
                name: TextMatch::Exact(selector.name),
                prototype: Some(selector.prototype),
                relevance: None,
            })
        }
        "field" if mode == MatchMode::Exact => {
            let selector = selector::field(text).map_err(selection_failure)?;
            Ok(Target::Field {
                owner: Some(TextMatch::Exact(selector.class.descriptor)),
                name: TextMatch::Exact(selector.name),
                field_type: Some(selector.field_type),
            })
        }
        "method" | "field" => {
            let (owner, member) = match text.split_once("->") {
                Some((owner, member)) => (Some(type_match(owner, mode)?), member),
                None => (None, text),
            };
            let (name, signature) = if mode == MatchMode::Regex {
                // Parentheses/colons belong to regex syntax, not DEX signatures.
                (member, None)
            } else if kind == "method" {
                match member.find('(') {
                    Some(open) => {
                        let signature = &member[open..];
                        if !signature.contains(')') || signature.ends_with(')') {
                            return Err(invalid("method signature requires (parameters)return"));
                        }
                        (&member[..open], Some(signature.encode_utf16().collect()))
                    }
                    None => (member, None),
                }
            } else {
                match member.split_once(':') {
                    Some((name, signature)) => {
                        if signature.is_empty() {
                            return Err(invalid("field signature requires a type"));
                        }
                        (name, Some(signature.encode_utf16().collect()))
                    }
                    None => (member, None),
                }
            };
            if owner.is_none() && name.is_empty() {
                return Err(invalid("supply a member name or owner->"));
            }
            let name = text_match(name, mode)?;
            Ok(if kind == "method" {
                Target::Method {
                    owner,
                    name,
                    prototype: signature,
                    relevance: None,
                }
            } else {
                Target::Field {
                    owner,
                    name,
                    field_type: signature,
                }
            })
        }
        other => Err(invalid(format!(
            "unknown xref kind {other:?}; expected string|type|method|field|impl|extend|resource"
        ))),
    }
}

fn matched_text(matched: &Matched) -> String {
    match matched {
        Matched::Resource { id, name } => format!("{name} ({:#010x})", id.0),
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
