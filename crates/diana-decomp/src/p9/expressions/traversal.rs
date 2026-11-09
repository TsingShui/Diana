//! Iterative scheduling of delivered expressions without native-stack recursion.

use super::literals::{binary, literal, unary};
use super::work::{ExprEvent, push_event};
use super::{ExpressionSource, Expressions};
use crate::p7::*;
use crate::p9::{Origin, RenderError};

impl<S: ExpressionSource> Expressions<'_, '_, S> {
    fn require_operand(&self, id: ExprId) -> Result<(), RenderError> {
        if !matches!(
            self.code.expressions().get(id.0 as usize).map(|e| &e.kind),
            Some(
                ExprKind::Read { .. }
                    | ExprKind::Snapshot { .. }
                    | ExprKind::Literal { .. }
                    | ExprKind::Unavailable { .. }
            )
        ) {
            return Err(RenderError::Invariant(
                "operand is not a delivered leaf read",
            ));
        }
        Ok(())
    }

    fn unavailable_arguments(&mut self, id: ExprId) -> Result<(), RenderError> {
        let args = match &self
            .code
            .expressions()
            .get(id.0 as usize)
            .ok_or(RenderError::Invariant(
                "argument owner is outside its arena",
            ))?
            .kind
        {
            ExprKind::Call(call) => &call.arguments,
            ExprKind::FilledArray { elements, .. } => elements,
            _ => {
                return Err(RenderError::Invariant("argument owner lost its arguments"));
            }
        };
        let Arguments::Unavailable { gap, words } = args else {
            return Err(RenderError::Invariant("unavailable arguments became known"));
        };
        self.out.fmt(format_args!(
            "unavailable_arguments<{gap:?}; original_words={words:?}>"
        ))?;
        Ok(())
    }

    pub fn operand(&mut self, id: ExprId) -> Result<(), RenderError> {
        self.require_operand(id)?;
        self.expr(id)
    }

    pub fn expr(&mut self, id: ExprId) -> Result<(), RenderError> {
        let mut work = Vec::new();
        push_event(&mut work, ExprEvent::Expression(id, false))?;
        while let Some(event) = work.pop() {
            match event {
                ExprEvent::Expression(id, operand) => {
                    if operand {
                        self.require_operand(id)?;
                    }
                    self.expression_event(id, &mut work)?;
                }
                ExprEvent::End(id, start) => self.out.span(
                    start,
                    Origin::Expression {
                        method: self.method,
                        expression: id,
                    },
                )?,
                ExprEvent::Anchor(expression) => {
                    let position = self.out.position();
                    self.out.span(
                        position,
                        Origin::Expression {
                            method: self.method,
                            expression,
                        },
                    )?;
                }
                ExprEvent::Text(value) => self.out.put(value)?,
                ExprEvent::Reference(reference) => self.reference(reference)?,
                ExprEvent::Member(reference, receiver, signature) => {
                    self.member_event(&mut work, reference, receiver, signature)?
                }
                ExprEvent::MemberEnd(reference, receiver, signature, start) => {
                    self.member_end(reference, receiver, signature, start)?;
                }
                ExprEvent::SuperMember(reference, receiver, qualification) => {
                    self.super_member_event(reference, receiver, qualification)?
                }
                ExprEvent::CallMember(reference, receiver, qualification) => {
                    self.call_member_event(&mut work, reference, receiver, qualification)?
                }
                ExprEvent::CallMemberEnd(reference, receiver, qualification, start) => {
                    self.call_member_end(reference, receiver, qualification, start)?;
                }
                ExprEvent::Via(owner) => self.via(owner)?,
                ExprEvent::InvocationPrototype(reference) => {
                    self.out.put(" [invocation prototype: ")?;
                    self.reference(reference)?;
                    self.out.put("]")?;
                }
                ExprEvent::UnavailableArguments(id) => self.unavailable_arguments(id)?,
                ExprEvent::ConcatPart { call, part } => self.concat_part(&mut work, call, part)?,
                ExprEvent::ConcatEnd { call, reference } => self.concat_end(call, reference)?,
                ExprEvent::ConcatUnavailable(reason) => {
                    self.out.fmt(format_args!(
                        " /* Diana: concat unavailable: {} */",
                        reason.message()
                    ))?;
                }
                ExprEvent::ArrayEnd(kind) => self.out.fmt(format_args!("] /* {kind:?} load */"))?,
                ExprEvent::Dimensions(dimensions) => {
                    self.out.put("]")?;
                    for _ in 1..dimensions {
                        self.out.put("[]")?;
                    }
                }
                ExprEvent::Initializer(reference, constructor) => {
                    if !self.redundant_initializer(reference, constructor) {
                        self.out.put(" [initializer: ")?;
                        self.member(constructor, None, true)?;
                        self.out.put("]")?;
                    }
                }
                ExprEvent::CompositionEnd {
                    composed,
                    start,
                    previous_carrier,
                    previous_statement,
                } => {
                    self.check_carrier = previous_carrier;
                    self.statement = previous_statement;
                    self.composition_end(composed, start)?;
                }
            }
        }
        Ok(())
    }

    fn expression_event(
        &mut self,
        id: ExprId,
        work: &mut Vec<ExprEvent>,
    ) -> Result<(), RenderError> {
        let e = self
            .code
            .expressions()
            .get(id.0 as usize)
            .ok_or(RenderError::Invariant(
                "expression reference is outside its arena",
            ))?;
        let start = self.out.position();
        push_event(work, ExprEvent::End(id, start))?;
        match &e.kind {
            ExprKind::Read { storage, .. } => self.read_event(work, id, *storage, start)?,
            ExprKind::Snapshot { storage } => self.local(*storage)?,
            ExprKind::Literal { bits, view } => {
                if self.code.boolean_use(id) {
                    self.out
                        .put(if bits.value == 0 { "false" } else { "true" })?;
                } else {
                    literal(self.out, *bits, *view)?;
                }
            }
            ExprKind::Unavailable { gap } => self.out.fmt(format_args!("unavailable<{gap:?}>"))?,
            ExprKind::Unary { op, value } => {
                self.out.put("(")?;
                self.out.put(unary(*op))?;
                push_event(work, ExprEvent::Text(")"))?;
                push_event(work, ExprEvent::Expression(*value, true))?;
            }
            ExprKind::Binary { op, left, right } => {
                self.out.put("(")?;
                push_event(work, ExprEvent::Text(")"))?;
                push_event(work, ExprEvent::Expression(*right, true))?;
                push_event(work, ExprEvent::Text(binary(*op)))?;
                push_event(work, ExprEvent::Expression(*left, true))?;
            }
            ExprKind::Compare { op, left, right } => {
                self.out.fmt(format_args!("compare<{op:?}>("))?;
                push_event(work, ExprEvent::Text(")"))?;
                push_event(work, ExprEvent::Expression(*right, true))?;
                push_event(work, ExprEvent::Text(", "))?;
                push_event(work, ExprEvent::Expression(*left, true))?;
            }
            ExprKind::String(r) => self.reference(*r)?,
            ExprKind::Class(r) => {
                self.reference(*r)?;
                self.out.put(".class")?;
            }
            ExprKind::MethodHandle(r) | ExprKind::MethodType(r) => self.reference(*r)?,
            ExprKind::Field {
                reference,
                receiver,
            } => {
                let qualified = self.code.field_qualification(*reference);
                push_event(work, ExprEvent::Member(*reference, *receiver, qualified))?;
            }
            ExprKind::Array { array, index, kind } => {
                push_event(work, ExprEvent::ArrayEnd(*kind))?;
                push_event(work, ExprEvent::Expression(*index, true))?;
                push_event(work, ExprEvent::Text("["))?;
                push_event(work, ExprEvent::Expression(*array, true))?;
            }
            ExprKind::ArrayLength(a) => {
                push_event(work, ExprEvent::Text(".length"))?;
                push_event(work, ExprEvent::Expression(*a, true))?;
            }
            ExprKind::InstanceOf { value, reference } => {
                self.out.put("(")?;
                push_event(work, ExprEvent::Text(")"))?;
                push_event(work, ExprEvent::Reference(*reference))?;
                push_event(work, ExprEvent::Text(" instanceof "))?;
                push_event(work, ExprEvent::Expression(*value, true))?;
            }
            ExprKind::AllocateInstance { reference } => {
                self.out.put("allocate_uninitialized ")?;
                self.reference(*reference)?;
            }
            ExprKind::NewArray { reference, length } => {
                self.new_array_event(work, *reference, *length, start)?
            }
            ExprKind::FilledArray {
                reference,
                elements,
            } => {
                self.out.put("new ")?;
                self.reference(*reference)?;
                self.out.put(" {")?;
                push_event(work, ExprEvent::Text("}"))?;
                self.argument_events(work, id, elements)?;
            }
            ExprKind::Call(call) => self.call_event(work, id, call)?,
            ExprKind::New {
                reference,
                constructor,
                arguments,
            } => {
                self.outline_call(*constructor, crate::p2::InvokeKind::Direct)?;
                if !self.kotlin_notation() {
                    self.out.put("new ")?;
                }
                self.reference(*reference)?;
                self.out.put("(")?;
                push_event(work, ExprEvent::Initializer(*reference, *constructor))?;
                push_event(work, ExprEvent::Text(")"))?;
                self.known_argument_events(work, arguments)?;
            }
        }
        Ok(())
    }
}
