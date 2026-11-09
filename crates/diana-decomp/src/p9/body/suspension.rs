//! Prints only the P7-selected local continuation-protocol surface.
use super::super::expressions::Expressions;
use super::super::{Origin, RenderError};
use crate::p7::{ControlId, Suspension};

impl Expressions<'_, '_> {
    pub(super) fn coroutine_guard(
        &mut self,
        site: &crate::p7::CoroutineCall,
    ) -> Result<(), RenderError> {
        let terminal = &self.code.terminals()[site.predicate.0 as usize];
        let crate::p7::TerminalKind::If {
            condition,
            when_true,
            when_false,
        } = &terminal.kind
        else {
            return Err(RenderError::Invariant(
                "protocol guard lost original predicate",
            ));
        };
        let previous = std::mem::replace(&mut self.out.abi_comment, true);
        let rendered = (|| {
            self.out.indent()?;
            let start = self.out.position();
            self.out.put("if (")?;
            let carrier = self
                .check_carrier
                .replace(crate::p7::CheckCarrier::Terminal(site.predicate));
            let result = self.condition(condition, false);
            self.check_carrier = carrier;
            result?;
            self.out.put(") goto ")?;
            self.label_name(self.code.transfers()[when_true.0 as usize].destination)?;
            self.out.put("; else goto ")?;
            self.label_name(self.code.transfers()[when_false.0 as usize].destination)?;
            self.out.put(";\n")?;
            self.out.span(
                start,
                Origin::Terminal {
                    method: self.method,
                    block: site.predicate,
                },
            )?;
            self.out.span(
                start,
                Origin::Transfer {
                    method: self.method,
                    transfer: site.suspended,
                },
            )?;
            Ok::<_, RenderError>(())
        })();
        self.out.abi_comment = previous;
        rendered?;
        self.transfer(site.normal)
    }
    pub(super) fn begin_abi(
        &mut self,
        section: &'static str,
        control: ControlId,
    ) -> Result<(), RenderError> {
        self.out.abi_comment = false;
        self.out.indent()?;
        self.out
            .fmt(format_args!("// Diana: ABI {section}, entry "))?;
        let entry =
            self.code.controls()[control.0 as usize]
                .entry
                .ok_or(RenderError::Invariant(
                    "ABI attachment lost its delivered entry",
                ))?;
        self.label_name(entry)?;
        self.out.put("\n")?;
        self.out.abi_comment = true;
        Ok(())
    }

    pub(super) fn end_abi(&mut self, initial: ControlId) -> Result<(), RenderError> {
        self.out.abi_comment = false;
        self.out.indent()?;
        self.out.put("// Diana: suspend body, original entry ")?;
        if let Some(entry) = self.code.controls()[initial.0 as usize].entry {
            self.label_name(entry)?;
        }
        self.out.put("\n")?;
        Ok(())
    }

    pub(super) fn abi_operation(&mut self, operation: crate::p2::OpId) -> Result<(), RenderError> {
        let previous = std::mem::replace(&mut self.out.abi_comment, true);
        let rendered = self.statement(operation, None);
        self.out.abi_comment = previous;
        rendered
    }

    pub(super) fn abi_dispatch(&mut self) -> Result<(), RenderError> {
        let body = self
            .code
            .emission()
            .coroutine_body()
            .ok_or(RenderError::Invariant(
                "ABI dispatch lost its selected body",
            ))?;
        let decision = self.code.controls()[body.dispatch.0 as usize]
            .decision
            .as_ref()
            .ok_or(RenderError::Invariant(
                "ABI dispatch lost its original tests",
            ))?;
        for test in &decision.arms {
            let terminal = &self.code.terminals()[test.terminal.0 as usize];
            let crate::p7::TerminalKind::If {
                condition,
                when_true,
                when_false,
            } = &terminal.kind
            else {
                return Err(RenderError::Invariant("ABI test lost its branch"));
            };
            self.out.indent()?;
            let start = self.out.position();
            let entry = self.code.controls()[test.header.0 as usize]
                .entry
                .ok_or(RenderError::Invariant("ABI test lost its original entry"))?;
            self.label_name(entry)?;
            self.out.put(": if (")?;
            let previous = self
                .check_carrier
                .replace(crate::p7::CheckCarrier::Terminal(test.terminal));
            let rendered = self.condition(condition, false);
            self.check_carrier = previous;
            rendered?;
            self.out.put(") goto ")?;
            self.label_name(self.code.transfers()[when_true.0 as usize].destination)?;
            self.out.put("; else goto ")?;
            self.label_name(self.code.transfers()[when_false.0 as usize].destination)?;
            self.out.put(";\n")?;
            self.out.span(
                start,
                Origin::Terminal {
                    method: self.method,
                    block: test.terminal,
                },
            )?;
            for &transfer in [when_true, when_false] {
                self.out.span(
                    start,
                    Origin::Transfer {
                        method: self.method,
                        transfer,
                    },
                )?;
            }
            self.suspension_control_anchor(test.header)?;
            self.suspension_control_anchor(test.source)?;
        }
        Ok(())
    }

    pub(super) fn suspension_assignment(&mut self, plan: &Suspension) -> Result<(), RenderError> {
        let start = self.out.position();
        self.local(plan.merged)?;
        self.out.put(" = ")?;
        self.expr(plan.expression)?;
        self.out.span(
            start,
            Origin::Copy {
                method: self.method,
                copy: plan.normal_copy,
            },
        )?;
        self.out.span(
            start,
            Origin::Transfer {
                method: self.method,
                transfer: plan.links[0],
            },
        )?;
        let position = self.out.position();
        self.out.span(
            position,
            Origin::Expression {
                method: self.method,
                expression: plan.normal_source,
            },
        )
    }

    pub(super) fn suspension_tail(&mut self, index: u32) -> Result<(), RenderError> {
        let plan = self
            .code
            .emission()
            .suspensions()
            .get(index as usize)
            .ok_or(RenderError::Invariant(
                "suspension surface lost its delivered protocol",
            ))?;
        let position = self.out.position();
        for block in [plan.predicate, plan.marker_return] {
            let terminal = &self.code.terminals()[block.0 as usize];
            for &expression in &terminal.inputs {
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
                Origin::Terminal {
                    method: self.method,
                    block,
                },
            )?;
        }
        self.out.span(
            position,
            Origin::Transfer {
                method: self.method,
                transfer: plan.links[1],
            },
        )?;
        for control in [plan.header, plan.bodies[0], plan.bodies[1]] {
            self.suspension_control_anchor(control)?;
        }
        Ok(())
    }

    fn suspension_control_anchor(&mut self, control: ControlId) -> Result<(), RenderError> {
        let position = self.out.position();
        if let Some(label) = self.code.controls()[control.0 as usize].entry {
            self.out.span(
                position,
                Origin::Label {
                    method: self.method,
                    label,
                },
            )?;
        }
        self.out.span(
            position,
            Origin::Control {
                method: self.method,
                control,
            },
        )
    }
}
