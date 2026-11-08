//! Copy bundles and their delivered phase positions.

use super::{ExpressionSource, Expressions};
use crate::p7::*;
use crate::p9::{Origin, RenderError};

impl<S: ExpressionSource> Expressions<'_, '_, S> {
    pub fn copy(&mut self, id: CopyPlanId) -> Result<(), RenderError> {
        let bundle = &self.code.copies()[id.0 as usize];
        let start = self.out.position();
        for (index, step) in bundle.steps.iter().enumerate() {
            if index > 0 {
                self.out.put(" ")?;
            }
            match step {
                CopyStatement::Assign {
                    destination,
                    expression,
                    ..
                } => {
                    self.local(*destination)?;
                    self.out.put(" = ")?;
                    // Only this step's right-hand side reads the checked value.
                    // Another step of the same bundle and the gap reports below
                    // stay at their own positions with no carrier.
                    let step = u32::try_from(index).map_err(|_| RenderError::Capacity)?;
                    let previous = self
                        .check_carrier
                        .replace(CheckCarrier::Copy { bundle: id, step });
                    let rendered = self.operand(*expression);
                    self.check_carrier = previous;
                    rendered?;
                }
                CopyStatement::Save {
                    destination,
                    source,
                } => {
                    self.local(*destination)?;
                    self.out.put(" = ")?;
                    self.local(*source)?;
                }
            }
            self.out.put(";")?;
        }
        for (index, req) in bundle.assignments.iter().enumerate() {
            if let Some(gap) = req.issue() {
                if !bundle.steps.is_empty() || index > 0 {
                    self.out.put(" ")?;
                }
                self.out.fmt(format_args!("unavailable_copy<{gap:?}>;"))?;
            }
        }
        self.out.span(
            start,
            Origin::Copy {
                method: self.method,
                copy: id,
            },
        )
    }

    pub fn copies(&mut self, ids: &[CopyPlanId]) -> Result<(), RenderError> {
        for &id in ids {
            self.copy(id)?;
        }
        Ok(())
    }

    pub fn phases(&mut self, ids: &[CopyPlanId], tag: &str) -> Result<(), RenderError> {
        if !ids.is_empty() {
            self.out.put(" [")?;
            self.out.put(tag)?;
            self.out.put(" {")?;
            self.copies(ids)?;
            self.out.put("}]")?;
        }
        Ok(())
    }
}
