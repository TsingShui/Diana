use super::{
    matching::MatchArgs,
    output::{Failure, Format},
    table::{RowSummary, publish_rows},
};
use crate::analysis::{input::InputOptions, request::RequestScope, resources::Catalog};
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
