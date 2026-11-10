//! Prints the owning P7 call-site view without recognizing protocols or operands.

use super::work::{ExprEvent, push_event};
use super::{ExpressionSource, Expressions};
use crate::p7::*;
use crate::p9::{Origin, RenderError};

impl<S: ExpressionSource> Expressions<'_, '_, S> {
    pub(super) fn concat_event(
        &mut self,
        work: &mut Vec<ExprEvent>,
        call: ExprId,
        reference: ReferenceId,
        concat: &Concatenation,
    ) -> Result<(), RenderError> {
        self.out.put("(")?;
        push_event(work, ExprEvent::ConcatEnd { call, reference })?;
        push_event(work, ExprEvent::Text(")"))?;
        for part in (0..concat.parts().len()).rev() {
            push_event(work, ExprEvent::ConcatPart { call, part })?;
            if part != 0 || concat.empty_prefix() {
                push_event(work, ExprEvent::Text(" + "))?;
            }
        }
        if concat.empty_prefix() {
            self.out.put("\"\"")?;
        }
        Ok(())
    }

    pub(super) fn concat_part(
        &mut self,
        work: &mut Vec<ExprEvent>,
        id: ExprId,
        index: usize,
    ) -> Result<(), RenderError> {
        let ExprKind::Call(call) = &self
            .code
            .expressions()
            .get(id.0 as usize)
            .ok_or(RenderError::Invariant("concat lost its original call"))?
            .kind
        else {
            return Err(RenderError::Invariant("concat lost its original call"));
        };
        let Some(ConcatOutcome::Inferred(concat)) = call.concatenation() else {
            return Err(RenderError::Invariant(
                "concat lost its supplied presentation",
            ));
        };
        match concat
            .parts()
            .get(index)
            .ok_or(RenderError::Invariant("concat part outside its product"))?
        {
            ConcatPart::Argument {
                expression,
                conversion,
            } => {
                let cast = match conversion {
                    ConcatConversion::Value => None,
                    ConcatConversion::Byte => Some("((byte) "),
                    ConcatConversion::Short => Some("((short) "),
                    ConcatConversion::Int => Some("((int) "),
                    ConcatConversion::Char => Some("((char) "),
                };
                if let Some(cast) = cast {
                    self.out.put(cast)?;
                    push_event(work, ExprEvent::Text(")"))?;
                }
                push_event(work, ExprEvent::Expression(*expression, true))?;
            }
            ConcatPart::Text { reference, units } => {
                let Ok(ReferenceDeclaration::String { units: original }) = &self
                    .code
                    .references()
                    .get(reference.0 as usize)
                    .ok_or(RenderError::Invariant("concat text lost its reference"))?
                    .declaration
                else {
                    return Err(RenderError::Invariant("concat text lost its String"));
                };
                let text = original
                    .get(units.clone())
                    .ok_or(RenderError::Invariant("concat text range unavailable"))?;
                let start = self.out.position();
                self.out.put("\"")?;
                self.out.units(text, false)?;
                self.out.put("\"")?;
                self.out.span(
                    start,
                    Origin::Reference {
                        method: self.method,
                        reference: *reference,
                    },
                )?;
            }
            ConcatPart::Constant { units, .. } => {
                let start = self.out.position();
                self.out.put("\"")?;
                self.out.units(units, false)?;
                self.out.put("\"")?;
                self.out.span(
                    start,
                    Origin::Reference {
                        method: self.method,
                        reference: call.target,
                    },
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn concat_end(
        &mut self,
        _call: ExprId,
        reference: ReferenceId,
    ) -> Result<(), RenderError> {
        let start = self.out.position();
        self.out.fmt(format_args!(
            " /* Diana: inferred StringConcatFactory, call_site r{} */",
            reference.0
        ))?;
        self.out.span(
            start,
            Origin::Reference {
                method: self.method,
                reference,
            },
        )
    }
}
