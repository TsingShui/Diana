//! Origins and temporary context for delivered composed expressions.

use super::{ExpressionSource, Expressions};
use crate::p7::*;
use crate::p9::{Origin, RenderError};

impl<S: ExpressionSource> Expressions<'_, '_, S> {
    /// P7 selected a relation over the original cmp-long operands. Both the
    /// numeric producer and its consumed read keep their expression origins.
    pub(in crate::p9) fn comparison_predicate(
        &mut self,
        carrier: ExprId,
        composed: ComposedExpression,
        left: ExprId,
        right: ExprId,
        operator: &'static str,
    ) -> Result<(), RenderError> {
        let start = self.out.position();
        let previous_carrier = self
            .check_carrier
            .replace(CheckCarrier::Statement(composed.statement));
        let previous_statement = self.statement.replace(composed.statement);
        let rendered = (|| {
            self.operand(left)?;
            self.out.put(operator)?;
            self.operand(right)
        })();
        self.check_carrier = previous_carrier;
        self.statement = previous_statement;
        rendered?;
        for expression in [composed.expression, carrier] {
            self.out.span(
                start,
                Origin::Expression {
                    method: self.method,
                    expression,
                },
            )?;
        }
        self.composition_end(composed, start)
    }

    pub(super) fn composition_end(
        &mut self,
        composed: ComposedExpression,
        start: crate::p9::TextOffset,
    ) -> Result<(), RenderError> {
        self.out.span(
            start,
            Origin::Statement {
                method: self.method,
                statement: composed.statement,
            },
        )?;
        if let Some(group) = self.code.statement_construction(composed.statement) {
            for &operation in &group.members {
                if group
                    .prepared_arguments
                    .iter()
                    .any(|prepared| prepared.operation == operation)
                {
                    continue; // The operand expression already owns this operation anchor.
                }
                let pc = self
                    .code
                    .operation_pc(operation)
                    .ok_or(RenderError::Invariant(
                        "composed construction lost an operation",
                    ))?;
                self.out.span(
                    start,
                    Origin::Operation {
                        method: self.method,
                        operation,
                        pc,
                    },
                )?;
            }
            for &site in self.code.statement_sites(composed.statement) {
                self.out.span(
                    start,
                    Origin::Site {
                        method: self.method,
                        site,
                    },
                )?;
            }
        } else {
            self.out.span(
                start,
                Origin::Operation {
                    method: self.method,
                    operation: composed.operation,
                    pc: composed.pc,
                },
            )?;
        }
        if let Some(control) = composed.source_control {
            let start = self
                .pending_controls
                .as_deref_mut()
                .and_then(|pending| pending.get_mut(control.0 as usize))
                .and_then(Option::take)
                .ok_or(RenderError::Invariant("composed control was not opened"))?;
            self.out.span(
                start,
                Origin::Control {
                    method: self.method,
                    control,
                },
            )?;
        }
        if let Some(site) = composed.site {
            let position = self.out.position();
            self.out.span(
                position,
                Origin::Site {
                    method: self.method,
                    site,
                },
            )?;
        }
        if let Some(pc) = composed.result_pc {
            self.out.span(
                start,
                Origin::ResultCommit {
                    method: self.method,
                    operation: composed.operation,
                    pc,
                },
            )?;
        }
        Ok(())
    }
}
