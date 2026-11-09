//! If arms and ordered condition headers.

use super::super::expressions::Expressions;
use super::super::{Origin, RenderError};
use super::{Event, push};
use crate::p7::*;

impl Expressions<'_, '_> {
    pub(super) fn if_arms(
        &mut self,
        work: &mut Vec<Event>,
        yes: ControlId,
        no: ControlId,
        test: PredicateEmission,
        shape: IfShape,
        chained: bool,
    ) -> Result<(), RenderError> {
        match shape {
            IfShape::Empty => push(work, Event::EmptyIf(test))?,
            IfShape::Inverted => {
                push(work, Event::Close)?;
                push(work, Event::Control(no))?;
                // The omitted empty arm still owns controls,
                // labels and any silent original transfers.
                push(work, Event::EmptyArmAnchors(yes))?;
                push(
                    work,
                    Event::If {
                        test,
                        chained,
                        omitted_false: false,
                    },
                )?;
            }
            IfShape::IfElse | IfShape::ElseIf => {
                if matches!(shape, IfShape::ElseIf) {
                    push(work, Event::ChainedControl(no))?;
                    push(work, Event::ElseIf(test.when_false))?;
                } else {
                    push(work, Event::Close)?;
                    // A false arm that prints nothing keeps only its
                    // origins; the `else` block would state no work.
                    let silent_false = self.code.emission().omitted(no)
                        || self.code.emission().if_display(test.terminal) == IfDisplay::NoElse;
                    if !self.code.emission().omitted(no) {
                        if silent_false {
                            push(work, Event::EmptyArmAnchors(no))?;
                        } else {
                            push(work, Event::Control(no))?;
                            push(work, Event::Else(test.when_false))?;
                        }
                    }
                }
                push(work, Event::Control(yes))?;
                push(
                    work,
                    Event::If {
                        test,
                        chained,
                        omitted_false: self.code.emission().omitted(no)
                            || self.code.emission().if_display(test.terminal) == IfDisplay::NoElse,
                    },
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn condition_header(
        &mut self,
        work: &mut Vec<Event>,
        control: ControlId,
        chained: bool,
    ) -> Result<(), RenderError> {
        let (count, yes, no, fall) =
            match &self.code.emission().controls()[control.0 as usize].layout {
                EmissionLayout::Condition {
                    tests,
                    yes,
                    no,
                    fall,
                    ..
                } => (tests.len(), *yes, *no, *fall),
                _ => {
                    return Err(RenderError::Invariant(
                        "condition control lost its delivered layout",
                    ));
                }
            };
        if !chained {
            self.out.indent()?;
        }
        self.out.mark_outline("if");
        self.out.put("if (")?;
        for i in 0..count {
            if i > 0 {
                self.out.put(" || ")?;
            }
            let test = match &self.code.emission().controls()[control.0 as usize].layout {
                EmissionLayout::Condition { tests, .. } => tests[i],
                _ => {
                    return Err(RenderError::Invariant(
                        "condition control lost its delivered layout",
                    ));
                }
            };
            let start = self.out.position();
            self.predicate_expression(test, false)?;
            self.out.span(
                start,
                Origin::Terminal {
                    method: self.method,
                    block: test.terminal,
                },
            )?;
            // Both original test edges keep their identity at the
            // test's own evaluation position. The final other edge
            // is already the printed `else` route when that arm
            // exists; anchoring it twice would duplicate a source
            // span for one original transfer.
            let (select, other) = if test.negate {
                (test.when_false, test.when_true)
            } else {
                (test.when_true, test.when_false)
            };
            self.selected(select, false)?;
            if i + 1 < count || matches!(no, ConditionElse::None) {
                self.selected(other, false)?;
            }
        }
        self.out.put(")")?;
        self.out.open()?;
        push(work, Event::Close)?;
        // Later test runs print no words of their own. Their
        // delivered controls, labels, routes and copies keep
        // zero-width anchors at the shared arm.
        let run_count = count.min(self.code.controls()[control.0 as usize].children.len());
        for i in (1..run_count).rev() {
            let run = self.code.controls()[control.0 as usize].children[i];
            push(work, Event::Anchors(run))?;
        }
        match no {
            ConditionElse::Body(no) => {
                push(work, Event::Control(no))?;
                push(work, Event::Else(fall))?;
            }
            ConditionElse::Copies => push(work, Event::Else(fall))?,
            ConditionElse::Transfer => push(work, Event::ElseTransfer(fall))?,
            ConditionElse::None => {}
        }
        push(work, Event::Control(yes))?;
        Ok(())
    }

    pub(super) fn empty_if(&mut self, test: PredicateEmission) -> Result<(), RenderError> {
        let start = self.out.position();
        self.selected(test.when_true, false)?;
        self.selected(test.when_false, false)?;
        self.out.span(
            start,
            Origin::Terminal {
                method: self.method,
                block: test.terminal,
            },
        )?;
        Ok(())
    }

    pub(super) fn if_header(
        &mut self,
        test: PredicateEmission,
        chained: bool,
        omitted_false: bool,
    ) -> Result<(), RenderError> {
        if !chained {
            self.out.indent()?;
        }
        self.copies(&self.code.emission().terminals()[test.terminal.0 as usize].before)?;
        let a = self.out.position();
        self.out.mark_outline("if");
        self.out.put("if (")?;
        self.predicate_expression(test, false)?;
        self.out.put(")")?;
        self.out.span(
            a,
            Origin::Terminal {
                method: self.method,
                block: test.terminal,
            },
        )?;
        if omitted_false {
            self.selected(test.when_false, false)?;
        }
        if test.negate {
            // P7 admits only nonprinting true-edge bundles here;
            // their plan IDs retain zero-length original origins.
            self.selected(test.when_true, false)?;
        }
        self.out.open()?;
        let edge = if test.negate {
            test.when_false
        } else {
            test.when_true
        };
        if !self.copies_print_no_words(&self.code.emission().transfers()[edge.0 as usize].copies) {
            self.out.indent()?;
            self.selected(edge, false)?;
            self.out.put("\n")?;
        } else {
            self.selected(edge, false)?;
        }
        Ok(())
    }

    pub(super) fn else_if_header(&mut self, edge: TransferId) -> Result<(), RenderError> {
        self.out.close()?;
        self.out.indent()?;
        self.out.put("else ")?;
        self.selected(edge, false)?;
        Ok(())
    }

    pub(super) fn else_transfer_header(&mut self, edge: TransferId) -> Result<(), RenderError> {
        self.else_open()?;
        self.branch_transfer(edge, true)
    }

    fn else_open(&mut self) -> Result<(), RenderError> {
        self.out.close()?;
        self.out.indent()?;
        self.out.mark_outline("else");
        self.out.put("else")?;
        self.out.open()
    }

    pub(super) fn else_header(&mut self, edge: TransferId) -> Result<(), RenderError> {
        self.else_open()?;
        if !self.copies_print_no_words(&self.code.emission().transfers()[edge.0 as usize].copies) {
            self.out.indent()?;
            self.selected(edge, false)?;
            self.out.put("\n")?;
        } else {
            self.selected(edge, false)?;
        }
        Ok(())
    }
}
