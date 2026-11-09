//! Source ranges for printed and P7-omitted statements.

use super::super::expressions::Expressions;
use super::super::{Origin, RenderError, TextOffset};
use crate::p2::OpId;
use crate::p7::*;

impl Expressions<'_, '_> {
    pub(super) fn literal_origins(
        &mut self,
        op: OpId,
        original: &Operation,
        s: &Statement,
    ) -> Result<(), RenderError> {
        // Every use prints the literal itself, so this statement states
        // nothing; its original facts keep zero-length anchors here.
        let position = self.out.position();
        if let StatementKind::Move { expression, .. } | StatementKind::Evaluate { expression } =
            s.kind
        {
            self.out.span(
                position,
                Origin::Expression {
                    method: self.method,
                    expression,
                },
            )?;
        }
        self.out.span(
            position,
            Origin::Statement {
                method: self.method,
                statement: s.id,
            },
        )?;
        self.out.span(
            position,
            Origin::Operation {
                method: self.method,
                operation: op,
                pc: original.pc,
            },
        )?;
        if let Some(pc) = original.result_pc {
            self.out.span(
                position,
                Origin::ResultCommit {
                    method: self.method,
                    operation: op,
                    pc,
                },
            )?;
        }
        Ok(())
    }

    pub(super) fn nop_origins(
        &mut self,
        op: OpId,
        original: &Operation,
        s: &Statement,
    ) -> Result<(), RenderError> {
        // A `nop` performs nothing, so the plan prints no text for it and
        // keeps the statement, operation and PC as zero-length anchors.
        let position = self.out.position();
        for &site in &s.sites {
            self.out.span(
                position,
                Origin::Site {
                    method: self.method,
                    site,
                },
            )?;
        }
        self.out.span(
            position,
            Origin::Statement {
                method: self.method,
                statement: s.id,
            },
        )?;
        self.out.span(
            position,
            Origin::Operation {
                method: self.method,
                operation: op,
                pc: original.pc,
            },
        )?;
        Ok(())
    }

    pub(super) fn omitted_check_origins(
        &mut self,
        original: &Operation,
        s: &Statement,
        omitted: &OmittedCheck,
    ) -> Result<(), RenderError> {
        // P4 established from pre-check facts that this check cannot add
        // behavior at this position, so the plan prints no text for it. Its
        // statement, operation, routes, checked value and resolved target
        // name keep zero-length anchors here; its phases and definitions
        // were empty by admission.
        let position = self.out.position();
        let StatementKind::CheckCast { value, reference } = s.kind else {
            return Err(RenderError::Invariant(
                "omitted check lost its original check-cast kind",
            ));
        };
        self.out.span(
            position,
            Origin::Expression {
                method: self.method,
                expression: value,
            },
        )?;
        self.out.span(
            position,
            Origin::Reference {
                method: self.method,
                reference,
            },
        )?;
        // The printed check's delivered type name keeps its original Type
        // origin. P8 already maps this reference's method-scoped type use to
        // the named use it printed before omission; this adds no type fact.
        let declaration =
            self.code
                .references()
                .get(reference.0 as usize)
                .ok_or(RenderError::Invariant(
                    "omitted check lost its reference declaration",
                ))?;
        if let Ok(ReferenceDeclaration::Type { type_use }) = &declaration.declaration
            && let Some(name) = self.names.body.get(type_use.0 as usize).copied().flatten()
        {
            self.out.span(position, Origin::Type(name))?;
        }
        for &site in &s.sites {
            self.out.span(
                position,
                Origin::Site {
                    method: self.method,
                    site,
                },
            )?;
        }
        self.out.span(
            position,
            Origin::Statement {
                method: self.method,
                statement: s.id,
            },
        )?;
        self.out.span(
            position,
            Origin::Operation {
                method: self.method,
                operation: omitted.operation,
                pc: omitted.pc,
            },
        )?;
        if let Some(pc) = original.result_pc {
            self.out.span(
                position,
                Origin::ResultCommit {
                    method: self.method,
                    operation: omitted.operation,
                    pc,
                },
            )?;
        }
        Ok(())
    }

    pub(super) fn statement_origins(
        &mut self,
        op: OpId,
        original: &Operation,
        s: &Statement,
        start: TextOffset,
    ) -> Result<(), RenderError> {
        self.out.span(
            start,
            Origin::Statement {
                method: self.method,
                statement: s.id,
            },
        )?;
        if let StatementKind::Construction(g) = s.kind {
            let group = &self.code.constructions()[g.0 as usize];
            for &member in &group.members {
                if group
                    .prepared_arguments
                    .iter()
                    .any(|prepared| prepared.operation == member)
                {
                    continue; // Its composed expression supplies the original operation anchor.
                }
                let source = &self.code.operations()[member.0 as usize];
                if !matches!(source.emission, Emission::Construction(_)) {
                    continue;
                }
                self.out.span(
                    start,
                    Origin::Operation {
                        method: self.method,
                        operation: member,
                        pc: source.pc,
                    },
                )?;
            }
        } else {
            self.out.span(
                start,
                Origin::Operation {
                    method: self.method,
                    operation: op,
                    pc: original.pc,
                },
            )?;
        }
        if let Some(pc) = original.result_pc {
            self.out.span(
                start,
                Origin::ResultCommit {
                    method: self.method,
                    operation: op,
                    pc,
                },
            )?;
        }
        Ok(())
    }
}
