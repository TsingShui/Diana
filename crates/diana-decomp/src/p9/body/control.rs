//! Blocks and the labels a printed transfer names.

use super::super::expressions::Expressions;
use super::super::{Origin, RenderError};
use super::{Event, push};
use crate::p7::*;

impl Expressions<'_, '_> {
    pub(super) fn end_control(
        &mut self,
        id: ControlId,
        start: super::super::TextOffset,
    ) -> Result<(), RenderError> {
        if let Some(completion) = self.code.emission().controls()[id.0 as usize].continuation {
            self.out.indent()?;
            match completion {
                ControlCompletion::Goto(label) => {
                    self.out.put("goto ")?;
                    self.label_name(label)?;
                    self.out.put("; // planned continuation\n")?;
                }
                ControlCompletion::Break(scope) | ControlCompletion::Continue(scope) => {
                    self.out
                        .put(if matches!(completion, ControlCompletion::Break(_)) {
                            "break "
                        } else {
                            "continue "
                        })?;
                    self.out
                        .put(&self.code.emission().controls()[scope.0 as usize].label)?;
                    self.out.put(";\n")?;
                }
                ControlCompletion::BreakHere(_) => self.out.put("break;\n")?,
                ControlCompletion::ContinueHere(_) => self.out.put("continue;\n")?,
            }
        }
        if self.code.emission().deferred_control(id) {
            let slot = self
                .pending_controls
                .as_deref_mut()
                .and_then(|pending| pending.get_mut(id.0 as usize))
                .ok_or(RenderError::Invariant("deferred control is unavailable"))?;
            if slot.replace(start).is_some() {
                return Err(RenderError::Invariant("control was deferred twice"));
            }
        } else {
            self.out.span(
                start,
                Origin::Control {
                    method: self.method,
                    control: id,
                },
            )?;
        }
        Ok(())
    }

    pub(super) fn control_anchors(
        &mut self,
        work: &mut Vec<Event>,
        control: ControlId,
        empty_arm: bool,
    ) -> Result<(), RenderError> {
        let start = self.out.position();
        self.out.span(
            start,
            Origin::Control {
                method: self.method,
                control,
            },
        )?;
        let node = match self.code.emission().controls()[control.0 as usize].layout {
            EmissionLayout::Block(node) => Some(node),
            _ => None,
        };
        if let Some(node) = node {
            let start = self.out.position();
            self.out.span(
                start,
                Origin::Label {
                    method: self.method,
                    label: LabelId(node.0),
                },
            )?;
            let transfers = match &self.code.emission().blocks()[node.0 as usize].end {
                BlockEnd::Transfers(links) => links.len(),
                _ => 0,
            };
            for at in 0..transfers {
                let link = match &self.code.emission().blocks()[node.0 as usize].end {
                    BlockEnd::Transfers(links) => links[at],
                    _ => {
                        return Err(RenderError::Invariant("anchored block lost its transfer"));
                    }
                };
                let start = self.out.position();
                let copies = self.code.emission().transfers()[link.0 as usize]
                    .copies
                    .len();
                for index in 0..copies {
                    let copy = self.code.emission().transfers()[link.0 as usize].copies[index];
                    self.copies(std::slice::from_ref(&copy))?;
                }
                self.out.span(
                    start,
                    Origin::Transfer {
                        method: self.method,
                        transfer: link,
                    },
                )?;
            }
            let terminal = match &self.code.emission().blocks()[node.0 as usize].end {
                BlockEnd::Terminal(terminal) => Some(*terminal),
                _ => None,
            };
            if let Some(terminal) = terminal {
                let start = self.out.position();
                match self.code.terminals()[terminal.0 as usize].kind {
                    TerminalKind::Goto(link) if empty_arm => {
                        self.selected(link, false)?;
                    }
                    TerminalKind::If {
                        when_true,
                        when_false,
                        ..
                    } if empty_arm
                        && self.code.emission().if_display(terminal) == IfDisplay::Omitted =>
                    {
                        self.selected(when_true, false)?;
                        self.selected(when_false, false)?;
                    }
                    _ => {}
                }
                self.out.span(
                    start,
                    Origin::Terminal {
                        method: self.method,
                        block: terminal,
                    },
                )?;
            }
        } else {
            let children = self.code.emission().controls()[control.0 as usize]
                .children(&self.code.controls()[control.0 as usize]);
            for &child in children.iter().rev() {
                push(
                    work,
                    if empty_arm {
                        Event::EmptyArmAnchors(child)
                    } else {
                        Event::Anchors(child)
                    },
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn block(&mut self, node: crate::p6::NodeId) -> Result<(), RenderError> {
        self.label(LabelId(node.0))?;
        let b = &self.code.emission().blocks()[node.0 as usize];
        for action in &b.actions {
            match *action {
                BodyAction::Statement { operation, .. } => {
                    let previous = self.out.origin_only;
                    self.out.origin_only |= self.code.emission().origin_only_operation(operation);
                    let result = self.statement(operation, None);
                    self.out.origin_only = previous;
                    result?;
                }
                BodyAction::AbiOperation { operation, .. } => self.abi_operation(operation)?,
                BodyAction::Release {
                    operation, scope, ..
                } => self.statement(operation, Some(scope))?,
                BodyAction::Receive { handler } => {
                    let h = &self.code.handlers()[handler as usize];
                    if !h.receives.is_empty() {
                        self.out.indent()?;
                        self.out.put("on_exception_entry {")?;
                        self.copies(&h.receives)?;
                        self.out.put("}\n")?;
                    }
                }
            }
        }
        match &b.end {
            BlockEnd::None => Ok(()),
            BlockEnd::Terminal(t) => {
                let previous = self.out.origin_only;
                self.out.origin_only |= self.code.emission().origin_only_terminal(*t);
                let result = self.terminal(*t);
                self.out.origin_only = previous;
                result
            }
            BlockEnd::Transfers(ts) => {
                for &t in ts {
                    self.transfer(t)?;
                }
                Ok(())
            }
        }
    }

    pub(super) fn label(&mut self, id: LabelId) -> Result<(), RenderError> {
        let start = self.out.position();
        if self.code.emission().blocks()[id.0 as usize].show_label {
            self.out.indent()?;
            self.label_name(id)?;
            self.out.put(":\n")?;
        }
        self.out.span(
            start,
            Origin::Label {
                method: self.method,
                label: id,
            },
        )
    }

    pub(super) fn label_name(&mut self, id: LabelId) -> Result<(), RenderError> {
        self.out.put(&self.code.labels()[id.0 as usize].name)
    }
}
