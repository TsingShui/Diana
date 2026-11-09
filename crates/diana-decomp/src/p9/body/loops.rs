//! Loop scopes and predicate placement.

use super::super::RenderError;
use super::super::expressions::Expressions;
use super::{Event, push};
use crate::p7::*;

impl Expressions<'_, '_> {
    pub(super) fn loop_layout(
        &mut self,
        work: &mut Vec<Event>,
        p: &EmissionControl,
        body: ControlId,
        placement: &LoopPlacement,
    ) -> Result<(), RenderError> {
        self.out.mark_outline("loop");
        self.out.indent()?;
        if p.label_required {
            self.out.put(&p.label)?;
            self.out.put(": ")?;
        }
        match placement {
            LoopPlacement::DoWhile {
                predicate,
                test,
                compact,
            } => {
                self.out.put("do")?;
                self.out.open()?;
                if *compact {
                    push(work, Event::CompactLoopTail(*predicate, *test))?;
                } else {
                    push(work, Event::LoopTailEnd(*test))?;
                    push(work, Event::Control(*predicate))?;
                    push(work, Event::LoopTailStart)?;
                }
                push(work, Event::Control(body))?;
            }
            LoopPlacement::While {
                predicate,
                test,
                compact,
            } => {
                if *compact {
                    self.out.put("while (")?;
                    self.compact_predicate(*predicate, *test)?;
                    self.out.put(")")?;
                    self.out.open()?;
                    push(work, Event::Close)?;
                    push(work, Event::Control(body))?;
                } else {
                    // The predicate keeps its own statements, so
                    // the loop states them where they run and
                    // leaves through the original condition
                    // instead of an unprintable test block.
                    self.out.put("while (true)")?;
                    self.out.open()?;
                    push(work, Event::Close)?;
                    push(work, Event::Control(body))?;
                    push(work, Event::LoopHead(*test))?;
                    push(work, Event::Control(*predicate))?;
                }
            }
            LoopPlacement::Infinite => {
                self.out.put("while (true)")?;
                self.out.open()?;
                push(work, Event::Close)?;
                push(work, Event::Control(body))?;
            }
        }
        Ok(())
    }

    pub(super) fn loop_head(&mut self, test: PredicateEmission) -> Result<(), RenderError> {
        // `while (cond) body` keeps the predicate's statements in the
        // loop and tests them again after each iteration: the body
        // runs only when the original condition holds.
        self.out.indent()?;
        self.out.put("if (")?;
        self.predicate(PredicateEmission {
            negate: !test.negate,
            ..test
        })?;
        self.out.put(")")?;
        self.out.open()?;
        self.out.line("break;")?;
        self.out.close()?;
        Ok(())
    }

    pub(super) fn compact_loop_tail(
        &mut self,
        predicate: ControlId,
        test: PredicateEmission,
    ) -> Result<(), RenderError> {
        self.out.close()?;
        self.out.mark_outline("loop-condition");
        self.out.indent()?;
        self.out.put("while (")?;
        self.compact_predicate(predicate, test)?;
        self.out.put(");\n")?;
        Ok(())
    }

    pub(super) fn loop_tail_start(&mut self) -> Result<(), RenderError> {
        self.out.close()?;
        self.out.mark_outline("loop-condition");
        self.out.indent()?;
        self.out.put("while (test")?;
        self.out.open()?;
        Ok(())
    }

    pub(super) fn loop_tail_end(&mut self, test: PredicateEmission) -> Result<(), RenderError> {
        self.out.indent()?;
        self.out.put("yield ")?;
        self.predicate(test)?;
        self.out.put(";\n")?;
        self.out.close()?;
        self.out.line(");")?;
        Ok(())
    }
}
