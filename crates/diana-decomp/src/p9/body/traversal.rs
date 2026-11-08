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
        if let Some(root) = self.code.root() {
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
                    omitted_false,
                } => self.if_header(test, chained, omitted_false)?,
                Event::ElseIf(edge) => self.else_if_header(edge)?,
                Event::Else(edge) => self.else_header(edge)?,
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
