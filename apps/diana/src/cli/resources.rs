use super::{
    matching::MatchArgs,
    output::{Failure, Format},
    table::{RowSummary, publish_rows},
};
use crate::analysis::{input::InputOptions, request::RequestScope, resources::Catalog};
use std::path::Path;

pub(super) fn run(
    input: &Path,
    filter: Option<&str>,
    matching: MatchArgs,
    format: Format,
    output: Option<&Path>,
) -> Result<(), Failure> {
    let filter = matching.filter(filter)?;
    let scope = RequestScope::open(input, &InputOptions::default()).map_err(Failure::input)?;
    let catalog = Catalog::read(&scope).map_err(|error| Failure::query(error, 1))?;
    let selected = catalog
        .select(filter.as_ref())
        .map_err(|error| Failure::query(error, 1))?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(selected.len())
        .map_err(|error| Failure::input(crate::analysis::input::InputError::Allocation(error)))?;
    for (row, _) in selected {
        rows.push(vec![
            format!(
                "e{}:resources.arsc@byte:{:#x}",
                row.entry.0, row.range_bytes[0]
            ),
            format!("{:#010x}", row.id.0),
            row.name.clone(),
            row.config.clone(),
            row.value.clone(),
        ]);
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
