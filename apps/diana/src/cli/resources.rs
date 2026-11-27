use super::{
    matching::MatchArgs,
    output::{Failure, Format},
    table::{RowSummary, publish_rows},
};
use crate::analysis::{filter::TextFilter, jvm::JvmRoots, jvm_resources, resources::Catalog};
use std::path::Path;

struct ResourceCells<'a> {
    context: String,
    id: String,
    row: &'a crate::analysis::resources::ResourceRow,
}
impl super::table::Cells for ResourceCells<'_> {
    fn cells(&self) -> impl Iterator<Item = &str> {
        [
            self.context.as_str(),
            self.id.as_str(),
            self.row.name.as_str(),
            self.row.config.as_str(),
            self.row.value.as_str(),
        ]
        .into_iter()
    }
}

struct JvmResourceCells<'a> {
    at: String,
    name: &'a str,
    compressed_size: String,
    uncompressed_size: String,
    compression: String,
}

impl super::table::Cells for JvmResourceCells<'_> {
    fn cells(&self) -> impl Iterator<Item = &str> {
        [
            self.at.as_str(),
            self.name,
            self.compressed_size.as_str(),
            self.uncompressed_size.as_str(),
            self.compression.as_str(),
        ]
        .into_iter()
    }
}

pub(super) fn run(
    input: &Path,
    filter: Option<&str>,
    matching: MatchArgs,
    format: Format,
    output: Option<&Path>,
) -> Result<(), Failure> {
    let filter = matching.filter(filter)?;
    let scope = super::open_scope(input, output)?;
    if let Some(roots) = JvmRoots::select_with_jobs(&scope, super::default_jobs())
        .map_err(|error| Failure::query(error, 1))?
    {
        return publish_jvm(&roots, "resources", filter.as_ref(), input, format, output);
    }
    let catalog = Catalog::read(&scope).map_err(|error| Failure::query(error, 1))?;
    let selected = catalog
        .select(filter.as_ref())
        .map_err(|error| Failure::query(error, 1))?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(selected.len())
        .map_err(|error| Failure::input(crate::analysis::input::InputError::Allocation(error)))?;
    for (row, _) in selected {
        rows.push(ResourceCells {
            context: format!(
                "e{}:resources.arsc@byte:{:#x}",
                row.entry.0, row.range_bytes[0]
            ),
            id: format!("{:#010x}", row.id.0),
            row,
        });
    }
    publish_rows(
        &rows,
        &["context", "id", "name", "config", "value"],
        "resources",
        format,
        input,
        output,
        RowSummary {
            total: rows.len(),
            warning: None,
            issues: &catalog.issues,
        },
    )
}

pub(super) fn publish_jvm(
    roots: &JvmRoots<'_>,
    command: &str,
    filter: Option<&TextFilter>,
    input: &Path,
    format: Format,
    output: Option<&Path>,
) -> Result<(), Failure> {
    let selected = jvm_resources::select(roots, filter, command == "resources")
        .map_err(|error| Failure::query(error, 1))?;
    let rows = selected.iter().map(|resource| JvmResourceCells {
        at: resource.at.to_string(),
        name: resource.entry.name(),
        compressed_size: resource.entry.compressed_size().0.to_string(),
        uncompressed_size: resource.entry.uncompressed_size().0.to_string(),
        compression: resource.entry.method().to_string(),
    });
    let mut issues = Vec::new();
    issues
        .try_reserve_exact(roots.issues().len())
        .map_err(|error| Failure::input(crate::analysis::input::InputError::Allocation(error)))?;
    issues.extend(
        roots
            .issues()
            .iter()
            .map(|issue| crate::analysis::xref::Issue {
                context: issue.context.clone(),
                kind: issue.kind.clone(),
                reason: issue.reason.clone(),
                range_bytes: None,
            }),
    );
    publish_rows(
        rows,
        &[
            "at",
            "name",
            "compressed_size",
            "uncompressed_size",
            "compression",
        ],
        command,
        format,
        input,
        output,
        RowSummary {
            total: selected.len(),
            warning: None,
            issues: &issues,
        },
    )
}
