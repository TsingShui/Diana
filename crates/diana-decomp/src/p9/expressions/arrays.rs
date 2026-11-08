//! Array allocation and selected literal payloads.

use super::literals::element_literal;
use super::work::{ExprEvent, push_event};
use super::{ExpressionSource, Expressions};
use crate::p7::*;
use crate::p9::{Origin, RenderError};

impl<S: ExpressionSource> Expressions<'_, '_, S> {
    pub(super) fn new_array_event(
        &mut self,
        work: &mut Vec<ExprEvent>,
        reference: ReferenceId,
        length: ExprId,
        start: crate::p9::TextOffset,
    ) -> Result<(), RenderError> {
        self.out.put("new ")?;
        let literal = self
            .statement
            .and_then(|statement| self.code.array_literal(statement));
        if let Some(literal) = literal {
            // The delivered array type already carries its one dimension;
            // the payload states every element, in original order and bits.
            self.reference(reference)?;
            self.array_literal(&literal)?;
            // The fill statement prints no text of its own, so its
            // statement, operation and hidden route origins stay at the
            // literal that states its effect.
            self.out.span(
                start,
                Origin::Statement {
                    method: self.method,
                    statement: literal.fill,
                },
            )?;
            self.out.span(
                start,
                Origin::Operation {
                    method: self.method,
                    operation: literal.operation,
                    pc: literal.pc,
                },
            )?;
            for &site in self.code.statement_sites(literal.fill) {
                self.out.span(
                    start,
                    Origin::Site {
                        method: self.method,
                        site,
                    },
                )?;
            }
        } else {
            // The delivered type is the array type; the instruction supplies
            // the first length and every remaining dimension is empty.
            let element = match &self.code.references()[reference.0 as usize].declaration {
                Ok(ReferenceDeclaration::Type { type_use }) => self
                    .names
                    .body
                    .get(type_use.0 as usize)
                    .copied()
                    .flatten()
                    .and_then(|id| self.lookup.dimensions(id).map(|d| (id, d))),
                _ => None,
            };
            match element {
                Some((type_use, dimensions)) if dimensions > 0 => {
                    self.lookup.print_element(self.out, type_use)?;
                    self.out.put("[")?;
                    push_event(work, ExprEvent::Dimensions(dimensions))?;
                    push_event(work, ExprEvent::Expression(length, true))?;
                }
                _ => {
                    self.reference(reference)?;
                    self.out.put("[")?;
                    push_event(work, ExprEvent::Text("]"))?;
                    push_event(work, ExprEvent::Expression(length, true))?;
                }
            }
        }
        Ok(())
    }
    /// The payload the allocation prints, between its delivered array type and
    /// the closing brace: every element in original order and exact bits.
    fn array_literal(&mut self, literal: &ArrayLiteral) -> Result<(), RenderError> {
        let payload = self
            .code
            .payloads()
            .get(literal.payload.0 as usize)
            .ok_or(RenderError::Invariant("array literal lost its payload"))?;
        let Ok(element) = payload.element else {
            return Err(RenderError::Invariant(
                "array literal lost its element interpretation",
            ));
        };
        self.out.put("{")?;
        for (index, &bits) in payload.bits.iter().enumerate() {
            if index > 0 {
                self.out.put(", ")?;
            }
            element_literal(self.out, element, bits)?;
        }
        self.out.put("}")
    }
}
