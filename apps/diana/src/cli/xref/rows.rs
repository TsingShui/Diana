use crate::analysis::input::InputError;
use crate::analysis::query::QueryError;
use crate::analysis::xref::{Event, Issue, RowView, XrefRow, code};

#[derive(Clone, Copy)]
enum RowId {
    Declaration(usize),
    Code { batch: usize, hit: usize },
}

/// Arrival order and ranking move integer row handles, never owned identities.
#[derive(Default)]
pub(super) struct Rows {
    declarations: Vec<XrefRow>,
    batches: Vec<code::Batch>,
    order: Vec<RowId>,
}

impl Rows {
    pub fn len(&self) -> usize {
        self.order.len()
    }

    fn row(&self, id: RowId) -> RowView<'_> {
        match id {
            RowId::Declaration(index) => self.declarations[index].view(),
            RowId::Code { batch, hit } => self.batches[batch].row(hit),
        }
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = RowView<'_>> + Clone {
        self.order.iter().map(|id| self.row(*id))
    }

    pub fn range(
        &self,
        range: std::ops::Range<usize>,
    ) -> impl ExactSizeIterator<Item = RowView<'_>> {
        self.order[range].iter().map(|id| self.row(*id))
    }

    #[cfg_attr(feature = "perf", fastrace::trace(name = "xref.rank"))]
    pub fn rank(&mut self) -> Result<(), QueryError> {
        use crate::analysis::filter::best_first;
        // Sorting must compare cached scalar keys. Following both identity
        // tables on every comparison defeats the compact hit representation.
        let mut ranked = Vec::new();
        ranked
            .try_reserve_exact(self.len())
            .map_err(InputError::Allocation)?;
        for id in &self.order {
            let row = self.row(*id);
            ranked.push((row.relevance, row.depth, super::row_order(row), *id));
        }
        ranked.sort_unstable_by(|left, right| {
            best_first(left.0, right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
        });
        for (id, (_, _, _, selected)) in self.order.iter_mut().zip(ranked) {
            *id = selected;
        }
        Ok(())
    }

    pub fn collect(&mut self, issues: &mut Vec<Issue>, event: Event) -> Result<(), QueryError> {
        match event {
            Event::Row(row) => {
                self.declarations
                    .try_reserve(1)
                    .map_err(InputError::Allocation)?;
                self.order.try_reserve(1).map_err(InputError::Allocation)?;
                let id = RowId::Declaration(self.declarations.len());
                self.declarations.push(row);
                self.order.push(id);
            }
            Event::Code(batch) => {
                self.batches
                    .try_reserve(1)
                    .map_err(InputError::Allocation)?;
                self.order
                    .try_reserve(batch.len())
                    .map_err(InputError::Allocation)?;
                let index = self.batches.len();
                for hit in 0..batch.len() {
                    self.order.push(RowId::Code { batch: index, hit });
                }
                self.batches.push(batch);
            }
            Event::Issue(issue) => {
                issues.try_reserve(1).map_err(InputError::Allocation)?;
                issues.push(issue);
            }
        }
        Ok(())
    }
}
