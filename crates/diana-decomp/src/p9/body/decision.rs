//! Kotlin `when` prints P6's already selected decision alternative.

use super::super::expressions::Expressions;
use super::super::{Origin, RenderError};
use super::{Event, push};
use crate::p7::*;

impl Expressions<'_, '_> {
    pub(super) fn switch_layout(
        &mut self,
        work: &mut Vec<Event>,
        id: ControlId,
        header: ControlId,
        bodies: &[ControlId],
        private_cases: bool,
        terminal: crate::p2::BlockId,
    ) -> Result<(), RenderError> {
        push(work, Event::Close)?;
        if private_cases {
            let TerminalKind::Switch { arms, .. } =
                &self.code.terminals()[terminal.0 as usize].kind
            else {
                return Err(RenderError::Invariant("switch lost its arms"));
            };
            for (&body, &arm) in bodies.iter().zip(arms).rev() {
                push(work, Event::Control(body))?;
                push(work, Event::Case(arm))?;
            }
        } else {
            for &body in bodies.iter().rev() {
                push(work, Event::Control(body))?;
            }
        }
        push(work, Event::Switch(id, terminal))?;
        push(work, Event::Control(header))?;
        Ok(())
    }

    pub(super) fn switch_header(
        &mut self,
        id: ControlId,
        terminal: crate::p2::BlockId,
    ) -> Result<(), RenderError> {
        let t = &self.code.terminals()[terminal.0 as usize];
        let TerminalKind::Switch { selector, arms } = &t.kind else {
            return Err(RenderError::Invariant("switch lost its selector"));
        };
        self.out.indent()?;
        self.copies(&self.code.emission().terminals()[terminal.0 as usize].before)?;
        let a = self.out.position();
        if self.code.emission().controls()[id.0 as usize].label_required {
            self.out
                .put(&self.code.emission().controls()[id.0 as usize].label)?;
            self.out.put(": ")?;
        }
        self.out.mark_outline("switch");
        self.out.put("switch (")?;
        self.operand(*selector)?;
        self.phases(
            &self.code.emission().terminals()[terminal.0 as usize].snapshots,
            "after_operands_before_effect",
        )?;
        self.out.put(")")?;
        self.out.span(
            a,
            Origin::Terminal {
                method: self.method,
                block: terminal,
            },
        )?;
        self.out.open()?;
        if !matches!(
            self.code.emission().controls()[id.0 as usize].layout,
            EmissionLayout::Switch {
                private_cases: true,
                ..
            }
        ) {
            self.switch_arms(arms)?;
        }
        Ok(())
    }

    pub(super) fn case_header(&mut self, arm: TransferId) -> Result<(), RenderError> {
        self.out.mark_outline("case");
        self.out.indent()?;
        match self.code.transfers()[arm.0 as usize].role {
            Some(crate::p2::NormalEdge::SwitchCase { key, .. }) => {
                self.out.fmt(format_args!("case {key}:"))?
            }
            Some(crate::p2::NormalEdge::SwitchDefault) => self.out.put("default:")?,
            _ => {
                return Err(RenderError::Invariant("switch arm lost its original role"));
            }
        }
        if !self.code.emission().transfers()[arm.0 as usize]
            .copies
            .is_empty()
        {
            self.out.put(" ")?;
        }
        self.selected(arm, false)?;
        self.out.put("\n")?;
        Ok(())
    }

    pub(super) fn decision(
        &mut self,
        control: ControlId,
        work: &mut Vec<Event>,
    ) -> Result<(), RenderError> {
        let plan = self.code.controls()[control.0 as usize]
            .decision
            .as_ref()
            .ok_or(RenderError::Invariant("when lost its P6 decision plan"))?;
        let first = plan
            .arms
            .first()
            .ok_or(RenderError::Invariant("when has no tests"))?;
        let header_start = self.out.position();
        self.block(first.predicate)?;
        self.out.indent()?;
        let emission = &self.code.emission().controls()[control.0 as usize];
        if emission.label_required {
            self.out.put(&emission.label)?;
            self.out.put(": ")?;
        }
        self.out.mark_outline("when");
        self.out.put("when (")?;
        self.operand(first.selector)?;
        self.out.span(
            header_start,
            Origin::Control {
                method: self.method,
                control: first.header,
            },
        )?;
        self.out.put(")")?;
        self.out.open()?;
        push(work, Event::Close)?;
        push(work, Event::Close)?;
        push(work, Event::Control(plan.default))?;
        push(work, Event::DecisionArm(control, plan.arms.len()))?;
        for (index, arm) in plan.arms.iter().enumerate().rev() {
            push(work, Event::Close)?;
            push(work, Event::Control(arm.body))?;
            push(work, Event::DecisionArm(control, index))?;
        }
        Ok(())
    }

    pub(super) fn decision_arm(
        &mut self,
        control: ControlId,
        index: usize,
    ) -> Result<(), RenderError> {
        let plan = self.code.controls()[control.0 as usize]
            .decision
            .as_ref()
            .ok_or(RenderError::Invariant("when arm lost its decision plan"))?;
        self.out.indent()?;
        let start = self.out.position();
        let edge = if let Some(arm) = plan.arms.get(index) {
            self.out.fmt(format_args!("{}", arm.key))?;
            if let Some(expression) = arm.key_expression {
                self.out.span(
                    start,
                    Origin::Expression {
                        method: self.method,
                        expression,
                    },
                )?;
            }
            self.out.span(
                start,
                Origin::Terminal {
                    method: self.method,
                    block: arm.terminal,
                },
            )?;
            // The first selector read already has its range in the when header.
            // Later pure reads disappear, not their original use identities.
            let position = self.out.position();
            if index > 0 {
                self.out.span(
                    position,
                    Origin::Expression {
                        method: self.method,
                        expression: arm.selector,
                    },
                )?;
            }
            if index > 0 {
                self.label(LabelId(arm.predicate.0))?;
                self.out.span(
                    position,
                    Origin::Control {
                        method: self.method,
                        control: arm.header,
                    },
                )?;
            }
            if arm.source != control {
                self.out.span(
                    position,
                    Origin::Control {
                        method: self.method,
                        control: arm.source,
                    },
                )?;
            }
            if index + 1 < plan.arms.len() {
                self.selected(arm.remaining, false)?;
            }
            arm.selected
        } else {
            self.out.put("else")?;
            plan.arms
                .last()
                .ok_or(RenderError::Invariant("when has no default edge"))?
                .remaining
        };
        self.out.put(" ->")?;
        self.out.open()?;
        if !self.code.emission().transfers()[edge.0 as usize]
            .copies
            .is_empty()
        {
            self.out.indent()?;
            self.selected(edge, false)?;
            self.out.put("\n")?;
        } else {
            self.selected(edge, false)?;
        }
        Ok(())
    }
}
