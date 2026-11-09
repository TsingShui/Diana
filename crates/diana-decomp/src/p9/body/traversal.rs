//! Iterative traversal of the delivered emission plan.

use super::super::RenderError;
use super::super::expressions::Expressions;
use super::{Event, push};

impl Expressions<'_, '_> {
    /// An explicit work stack, bounded by the delivered tree, prevents hostile
    /// region depth from consuming the native call stack. It does not plan CFGs.
    pub fn body(&mut self) -> Result<(), RenderError> {
        self.locals()?;
        self.exception_chains()?;
        self.bootstrap_definitions()?;
        if !self.code.emission().initial().is_empty() {
            self.out.indent()?;
            self.copies(self.code.emission().initial())?;
            self.out.put("\n")?;
        }
        let mut work = Vec::new();
        if let Some(flow) = self.code.emission().coroutine_flow() {
            push(&mut work, Event::CoroutineFlowControls)?;
            for &id in flow.source.iter().rev() {
                push(&mut work, Event::Control(id))?;
            }
            push(&mut work, Event::AbiMode(false))?;
            for &node in flow.abi.iter().rev() {
                push(&mut work, Event::Control(crate::p7::ControlId(node.0)))?;
            }
            push(&mut work, Event::AbiMode(true))?;
        } else if let Some(body) = self.code.emission().coroutine_loop() {
            push(&mut work, Event::CoroutineControls)?;
            for &node in body.tail.iter().rev() {
                push(&mut work, Event::Control(crate::p7::ControlId(node.0)))?;
                if body.abi_entries.contains(&node) {
                    push(&mut work, Event::CoroutineEntry(node))?;
                }
            }
            push(&mut work, Event::Close)?;
            for &node in body.body.iter().rev() {
                push(&mut work, Event::Control(crate::p7::ControlId(node.0)))?;
                if body.abi_entries.contains(&node) {
                    push(&mut work, Event::CoroutineEntry(node))?;
                }
            }
            push(&mut work, Event::CoroutineLoopHead(body.test))?;
            push(
                &mut work,
                Event::Control(crate::p7::ControlId(body.header.0)),
            )?;
            for &node in body.initial.iter().rev() {
                push(&mut work, Event::Control(crate::p7::ControlId(node.0)))?;
                if body.abi_entries.contains(&node) {
                    push(&mut work, Event::CoroutineEntry(node))?;
                }
            }
            push(&mut work, Event::AbiMode(false))?;
            for &node in body.abi.iter().rev() {
                push(&mut work, Event::Control(crate::p7::ControlId(node.0)))?;
            }
            push(&mut work, Event::AbiMode(true))?;
        } else if let Some(root) = self.code.root() {
            push(&mut work, Event::Control(root))?;
        }
        if let Some(body) = self.code.emission().coroutine_body() {
            push(&mut work, Event::EndAbi(body.initial))?;
            push(&mut work, Event::Control(body.invalid))?;
            push(
                &mut work,
                Event::BeginAbi("invalid-state arm", body.invalid),
            )?;
            push(&mut work, Event::Control(body.resume))?;
            push(
                &mut work,
                Event::BeginAbi("resumed completion", body.resume),
            )?;
            push(&mut work, Event::AbiDispatch)?;
            push(&mut work, Event::Control(body.prefix))?;
            push(
                &mut work,
                Event::BeginAbi("entry/reentry prefix", body.prefix),
            )?;
        }
        while let Some(event) = work.pop() {
            match event {
                Event::AbiMode(abi) => {
                    self.out.abi_comment = false;
                    self.out.line(if abi {
                        "// Diana: ABI entry/reentry, resume and invalid-state paths"
                    } else {
                        "// Diana: inferred suspend body; original ABI facts retained above"
                    })?;
                    self.out.abi_comment = abi;
                }
                Event::CoroutineLoopHead(test) => {
                    self.out.mark_outline("loop");
                    self.out.indent()?;
                    self.out.put("while (")?;
                    self.predicate(test)?;
                    self.out.put(")")?;
                    self.out.open()?;
                }
                Event::CoroutineControls => {
                    let position = self.out.position();
                    for control in self
                        .code
                        .controls()
                        .iter()
                        .filter(|c| !matches!(c.kind, crate::p7::ControlKind::Block { .. }))
                    {
                        self.out.span(
                            position,
                            super::super::Origin::Control {
                                method: self.method,
                                control: control.id,
                            },
                        )?;
                    }
                }
                Event::CoroutineFlowControls => {
                    let position = self.out.position();
                    for &control in &self
                        .code
                        .emission()
                        .coroutine_flow()
                        .ok_or(RenderError::Invariant("flow anchors lost their plan"))?
                        .anchors
                    {
                        self.out.span(
                            position,
                            super::super::Origin::Control {
                                method: self.method,
                                control,
                            },
                        )?;
                    }
                }
                Event::CoroutineEntry(node) => {
                    self.out.indent()?;
                    self.out.put("// Diana: original ABI target ")?;
                    self.label_name(crate::p7::LabelId(node.0))?;
                    self.out.put("\n")?;
                }
                Event::BeginAbi(section, control) => self.begin_abi(section, control)?,
                Event::EndAbi(initial) => self.end_abi(initial)?,
                Event::AbiDispatch => self.abi_dispatch()?,
                Event::OriginOnly(value) => self.out.origin_only = value,
                Event::Finally => self.finally_header()?,
                Event::Control(id) => self.control_event(&mut work, id, false)?,
                Event::ChainedControl(id) => self.control_event(&mut work, id, true)?,
                Event::End(id, start) => self.end_control(id, start)?,
                Event::Close => self.out.close()?,
                Event::DecisionArm(control, index) => self.decision_arm(control, index)?,
                Event::Condition { control, chained } => {
                    self.condition_header(&mut work, control, chained)?
                }
                Event::Anchors(control) => self.control_anchors(&mut work, control, false)?,
                Event::EmptyArmAnchors(control) => {
                    self.control_anchors(&mut work, control, true)?
                }
                Event::EmptyIf(test) => self.empty_if(test)?,
                Event::If {
                    test,
                    chained,
                    other_at_header,
                } => self.if_header(test, chained, other_at_header)?,
                Event::ElseIf(edge) => self.else_if_header(edge)?,
                Event::Else(edge) => self.else_header(edge)?,
                Event::GuardTail(edge) => self.guard_tail(edge)?,
                Event::ElseTransfer(edge) => self.else_transfer_header(edge)?,
                Event::LoopHead(test) => self.loop_head(test)?,
                Event::CompactLoopTail(predicate, test) => {
                    self.compact_loop_tail(predicate, test)?
                }
                Event::LoopTailStart => self.loop_tail_start()?,
                Event::LoopTailEnd(test) => self.loop_tail_end(test)?,
                Event::Switch(id, terminal) => self.switch_header(id, terminal)?,
                Event::Case(arm) => self.case_header(arm)?,
                Event::Catch(id, index) => self.catch_header(id, index)?,
                Event::Acquire(id, operation) => self.acquire_header(id, operation)?,
            }
        }
        Ok(())
    }
}
