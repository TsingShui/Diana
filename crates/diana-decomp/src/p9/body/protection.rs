//! Try, catch, finally and explicit monitor scopes.

use super::super::expressions::Expressions;
use super::super::{Origin, RenderError};
use super::{Event, push};
use crate::p7::*;

use crate::p2::OpId;

impl Expressions<'_, '_> {
    pub(super) fn try_layout(
        &mut self,
        work: &mut Vec<Event>,
        id: ControlId,
        body: ControlId,
        catches: &[ControlId],
        finally: Option<&FinallyEmission>,
    ) -> Result<(), RenderError> {
        let p = &self.code.emission().controls()[id.0 as usize];
        self.out.indent()?;
        if p.label_required {
            self.out.put(&p.label)?;
            self.out.put(": ")?;
        }
        self.out.mark_outline("try");
        self.out.put("try")?;
        self.out.open()?;
        push(work, Event::Close)?;
        if let Some(finally) = finally {
            push(work, Event::OriginOnly(self.out.origin_only))?;
            push(work, Event::Control(finally.duplicate))?;
            push(work, Event::OriginOnly(true))?;
            push(work, Event::Control(finally.cleanup))?;
            push(work, Event::Finally)?;
        } else {
            for (i, &handler) in catches.iter().enumerate().rev() {
                push(work, Event::Control(handler))?;
                push(work, Event::Catch(id, i))?;
            }
        }
        push(work, Event::Control(body))?;
        Ok(())
    }

    pub(super) fn finally_header(&mut self) -> Result<(), RenderError> {
        self.out.close()?;
        self.out.indent()?;
        self.out.mark_outline("finally");
        self.out.put("finally")?;
        self.out.open()?;
        Ok(())
    }

    pub(super) fn catch_header(&mut self, id: ControlId, index: usize) -> Result<(), RenderError> {
        self.out.close()?;
        self.out.indent()?;
        self.out.mark_outline("catch");
        self.out.put("catch (")?;
        let ControlKind::Try { chain, clauses, .. } = &self.code.controls()[id.0 as usize].kind
        else {
            return Err(RenderError::Invariant("catch lost its try"));
        };
        let clause = &self.code.chains()[chain.0 as usize].clauses[clauses.start as usize + index];
        // The handler block follows immediately, so the clause
        // prints only the type it catches; the route target is the
        // enclosing catch itself and needs no second mention.
        self.clause_type(clause)?;
        self.out.put(")")?;
        self.out.open()?;
        Ok(())
    }

    pub(super) fn acquire_header(
        &mut self,
        id: ControlId,
        operation: OpId,
    ) -> Result<(), RenderError> {
        let source = &self.code.operations()[operation.0 as usize];
        let statement = source
            .statement
            .ok_or(RenderError::Invariant("acquire lost its statement"))?;
        let ControlKind::Synchronized { lock, enter, .. } =
            &self.code.controls()[id.0 as usize].kind
        else {
            return Err(RenderError::Invariant("acquire lost its scope"));
        };
        let phase = &self.code.emission().operations()[operation.0 as usize];
        let control = &self.code.emission().controls()[id.0 as usize];
        self.out.indent()?;
        self.copies(&phase.before)?;
        let a = self.out.position();
        if control.label_required {
            self.out.put(&control.label)?;
            self.out.put(": ")?;
        }
        self.out.mark_outline("synchronized");
        self.out.put("synchronized (")?;
        self.operand(*lock)?;
        self.out.put(")")?;
        self.phases(&phase.snapshots, "after_operands_before_effect")?;
        self.sites(&self.code.statements()[statement.0 as usize].sites, a)?;
        self.out.span(
            a,
            Origin::Operation {
                method: self.method,
                operation,
                pc: source.pc,
            },
        )?;
        self.out.span(
            a,
            Origin::Statement {
                method: self.method,
                statement,
            },
        )?;
        // The closing brace is not a release: the delivered
        // `release synchronized` statements are the only releases.
        self.out.open_note("// Diana: releases are explicit")?;
        if !phase.after.is_empty()
            || !self.code.emission().transfers()[enter.0 as usize]
                .copies
                .is_empty()
        {
            self.out.indent()?;
            self.copies(&phase.after)?;
            self.selected(*enter, false)?;
            self.out.put("\n")?;
        } else {
            self.selected(*enter, false)?;
        }
        Ok(())
    }
}
