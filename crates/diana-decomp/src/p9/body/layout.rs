//! Schedules children of the selected control layout without reconstructing it.

use super::super::RenderError;
use super::super::expressions::Expressions;
use super::{Event, push};
use crate::p7::*;

impl Expressions<'_, '_> {
    fn scope_layout(
        &mut self,
        work: &mut Vec<Event>,
        c: &Control,
        p: &EmissionControl,
    ) -> Result<(), RenderError> {
        let graph = matches!(p.layout, EmissionLayout::Graph);
        self.out.indent()?;
        // A control name is printed only when a transfer still
        // names it; Sequence reaches here only when required.
        if p.label_required {
            self.out.put(&p.label)?;
            // The graph header continues on this line; a plain
            // block opens immediately after the colon.
            self.out.put(if graph { ": " } else { ":" })?;
        }
        if graph {
            self.out.mark_outline("graph");
            if let ControlKind::Graph {
                reason, entries, ..
            } = &c.kind
            {
                self.out.fmt(format_args!("graph<{reason:?}> entries ["))?;
                for (i, &entry) in entries.iter().enumerate() {
                    if i > 0 {
                        self.out.put(", ")?;
                    }
                    self.label_name(entry)?;
                }
                self.out.put("]")?;
            }
            if let Some(entry) = c.entry {
                self.out.put(" entry ")?;
                self.label_name(entry)?;
            } else {
                self.out.put(" no_implicit_entry")?;
            }
        }
        if graph || p.label_required {
            self.out.open()?;
        } else {
            // A bare break still names the innermost printed
            // scope, so only the unused name is omitted.
            self.out.open_plain()?;
        }
        push(work, Event::Close)?;
        for &child in p.children(c).iter().rev() {
            push(work, Event::Control(child))?;
        }
        Ok(())
    }

    /// Prints one delivered control and schedules its children. The plan already
    /// decided the shape, so this only walks it.
    pub(super) fn control_event(
        &mut self,
        work: &mut Vec<Event>,
        id: ControlId,
        chained: bool,
    ) -> Result<(), RenderError> {
        if self.code.emission().omitted(id) {
            return Ok(());
        }
        let c = &self.code.controls()[id.0 as usize];
        let p = &self.code.emission().controls()[id.0 as usize];
        let start = self.out.position();
        push(work, Event::End(id, start))?;
        match &p.layout {
            EmissionLayout::Block(node) => self.block(*node)?,
            EmissionLayout::Suspension { plan } => self.suspension_tail(*plan)?,
            EmissionLayout::Sequence if !p.label_required => {
                for &child in p.children(c).iter().rev() {
                    push(
                        work,
                        if chained {
                            Event::ChainedControl(child)
                        } else {
                            Event::Control(child)
                        },
                    )?;
                }
            }
            EmissionLayout::Labelled if p.transparent => {
                for &child in p.children(c).iter().rev() {
                    push(work, Event::Control(child))?;
                }
            }
            EmissionLayout::Sequence | EmissionLayout::Labelled | EmissionLayout::Graph => {
                self.scope_layout(work, c, p)?;
            }
            EmissionLayout::If {
                header,
                yes,
                no,
                test,
                shape,
            } => {
                if p.label_required {
                    self.out.indent()?;
                    self.out.put(&p.label)?;
                    self.out.put(":")?;
                    self.out.open()?;
                    push(work, Event::Close)?;
                }
                self.if_arms(work, *yes, *no, *test, *shape, chained)?;
                push(work, Event::Control(*header))?;
            }
            EmissionLayout::Condition { header, .. } => {
                push(
                    work,
                    Event::Condition {
                        control: id,
                        chained,
                    },
                )?;
                push(work, Event::Control(*header))?;
            }
            EmissionLayout::Loop { body, placement } => {
                self.loop_layout(work, p, *body, placement)?;
            }
            EmissionLayout::When { .. } => self.decision(id, work)?,
            EmissionLayout::Switch {
                header,
                bodies,
                private_cases,
                terminal,
            } => {
                self.switch_layout(work, id, *header, bodies, *private_cases, *terminal)?;
            }
            EmissionLayout::Try {
                body,
                catches,
                finally,
            } => {
                self.try_layout(work, id, *body, catches, finally.as_ref())?;
            }
            EmissionLayout::Synchronized {
                entry,
                body,
                acquire,
            } => {
                push(work, Event::Close)?;
                push(work, Event::Control(*body))?;
                push(work, Event::Acquire(id, *acquire))?;
                push(work, Event::Control(*entry))?;
            }
        }
        Ok(())
    }
}
