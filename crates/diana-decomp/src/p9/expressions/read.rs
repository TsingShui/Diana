//! Leaf reads, inline checks and delivered composed uses.

use super::work::{ExprEvent, push_event};
use super::{ExpressionSource, Expressions};
use crate::p7::*;
use crate::p9::{Origin, RenderError};

impl<S: ExpressionSource> Expressions<'_, '_, S> {
    /// P4 already proved this view lossless. Original primitive units select
    /// notation only; no desired argument type establishes a new conversion.
    pub(super) fn narrow_view(
        &self,
        view: crate::p4::Interpretation,
    ) -> Result<Option<(TypeUseId, bool)>, RenderError> {
        let crate::p4::Interpretation::IntegralView { declared } = view else {
            return Ok(None);
        };
        let id = TypeUseId(declared.0);
        let ty = self
            .code
            .types()
            .get(id.0 as usize)
            .ok_or(RenderError::Invariant(
                "integral view lost its owned type use",
            ))?;
        let character = ty.units == [u16::from(b'C')];
        Ok(
            (character || ty.units == [u16::from(b'B')] || ty.units == [u16::from(b'S')])
                .then_some((id, character)),
        )
    }

    pub(super) fn read_event(
        &mut self,
        work: &mut Vec<ExprEvent>,
        id: ExprId,
        storage: StorageId,
        start: crate::p9::TextOffset,
    ) -> Result<(), RenderError> {
        if let Some(check) = self
            .check_carrier
            .and_then(|carrier| self.code.inline_check(carrier, id, storage))
        {
            // The check prints at its selected first use and keeps its
            // own origin; any other read of the same storage stays plain.
            if self.kotlin_notation() {
                self.out.put("(")?;
                self.local(storage)?;
                self.out.put(" as ")?;
                self.kotlin_cast_reference(check.reference)?;
                self.out.put(")")?;
            } else {
                self.out.put("((")?;
                self.reference(check.reference)?;
                self.out.put(") ")?;
                self.local(storage)?;
                self.out.put(")")?;
            }
            self.out.span(
                start,
                Origin::Statement {
                    method: self.method,
                    statement: check.check,
                },
            )?;
            self.out.span(
                start,
                Origin::Operation {
                    method: self.method,
                    operation: check.operation,
                    pc: check.pc,
                },
            )?;
            // A hidden route still keeps an anchor at the check's text.
            for &site in self.code.statement_sites(check.check) {
                self.out.span(
                    start,
                    Origin::Site {
                        method: self.method,
                        site,
                    },
                )?;
            }
        } else if let Some(composed) = self.code.composed_use(id) {
            // Inline checks belong to the producer statement. Restore
            // each enclosing carrier after its nested expression prints.
            let previous_carrier = self
                .check_carrier
                .replace(CheckCarrier::Statement(composed.statement));
            let previous_statement = self.statement.replace(composed.statement);
            push_event(
                work,
                ExprEvent::CompositionEnd {
                    composed,
                    start,
                    previous_carrier,
                    previous_statement,
                },
            )?;
            if let ComposedResult::Store(destination) = composed.result {
                self.out.put("(")?;
                self.local(destination)?;
                self.out.put(" = ")?;
                push_event(work, ExprEvent::Text(")"))?;
            }
            push_event(work, ExprEvent::Expression(composed.expression, false))?;
        } else if self.code.boolean_use(id) && !self.code.boolean_local(storage) {
            self.out.put("(")?;
            self.local(storage)?;
            self.out.put(" != 0)")?;
        } else {
            self.local(storage)?;
        }
        Ok(())
    }
}
