//! Member qualification, call signatures and ordered argument events.

use super::work::{ExprEvent, push_event};
use super::{ExpressionSource, Expressions};
use crate::p7::*;
use crate::p9::{Origin, RenderError};

impl<S: ExpressionSource> Expressions<'_, '_, S> {
    pub(super) fn call_event(
        &mut self,
        work: &mut Vec<ExprEvent>,
        id: ExprId,
        call: &Call,
    ) -> Result<(), RenderError> {
        self.outline_call(call.target, call.kind)?;
        if matches!(
            call.kind,
            crate::p2::InvokeKind::Custom | crate::p2::InvokeKind::Polymorphic
        ) {
            self.out.fmt(format_args!("invoke<{:?}> ", call.kind))?;
        }

        push_event(work, ExprEvent::Text(")"))?;
        if !self.out.abi_comment
            && let Some((argument, continuation)) = self.code.implicit_continuation(id)
        {
            let Arguments::Known(arguments) = &call.arguments else {
                return Err(RenderError::Invariant(
                    "suspension lost its known arguments",
                ));
            };
            let visible = arguments
                .get(..argument as usize)
                .ok_or(RenderError::Invariant(
                    "suspension lost its argument projection",
                ))?;
            push_event(work, ExprEvent::Anchor(continuation))?;
            self.known_argument_events(work, visible)?;
        } else {
            self.argument_events(work, id, &call.arguments)?;
        }
        push_event(work, ExprEvent::Text("("))?;
        if let Some(proto) = call.invocation_prototype {
            push_event(work, ExprEvent::InvocationPrototype(proto))?;
        }
        match (call.kind, call.receiver) {
            (crate::p2::InvokeKind::Super, Some(receiver)) => push_event(
                work,
                ExprEvent::SuperMember(call.target, receiver, call.qualification),
            )?,
            _ => push_event(
                work,
                ExprEvent::CallMember(call.target, call.receiver, call.qualification),
            )?,
        }
        Ok(())
    }

    pub(super) fn member_event(
        &mut self,
        work: &mut Vec<ExprEvent>,
        reference: ReferenceId,
        receiver: Option<ExprId>,
        signature: bool,
    ) -> Result<(), RenderError> {
        let start = self.out.position();
        push_event(
            work,
            ExprEvent::MemberEnd(reference, receiver.is_some(), signature, start),
        )?;
        if let Some(receiver) = receiver {
            push_event(work, ExprEvent::Text("."))?;
            if let Some(owner) = self.code.field_receiver_owner(reference) {
                self.out.put("(")?;
                push_event(work, ExprEvent::Via(owner))?;
            }
            push_event(work, ExprEvent::Expression(receiver, true))?;
        }
        Ok(())
    }

    pub(super) fn super_member_event(
        &mut self,
        reference: ReferenceId,
        receiver: ExprId,
        qualification: Qualification,
    ) -> Result<(), RenderError> {
        let start = self.out.position();
        let owner = self.call_owner(reference, qualification);
        if owner.is_some() {
            self.out.put("(")?;
        }
        let receiver_start = self.out.position();
        self.out.put("super")?;
        self.out.span(
            receiver_start,
            Origin::Expression {
                method: self.method,
                expression: receiver,
            },
        )?;
        if let Some(owner) = owner {
            self.via(owner)?;
        }
        self.out.put(".")?;
        self.call_member_end(reference, true, qualification, start)?;
        Ok(())
    }

    pub(super) fn call_member_event(
        &mut self,
        work: &mut Vec<ExprEvent>,
        reference: ReferenceId,
        receiver: Option<ExprId>,
        qualification: Qualification,
    ) -> Result<(), RenderError> {
        let start = self.out.position();
        push_event(
            work,
            ExprEvent::CallMemberEnd(reference, receiver.is_some(), qualification, start),
        )?;
        if let Some(receiver) = receiver {
            push_event(work, ExprEvent::Text("."))?;
            if let Some(owner) = self.call_owner(reference, qualification) {
                self.out.put("(")?;
                push_event(work, ExprEvent::Via(owner))?;
            }
            push_event(work, ExprEvent::Expression(receiver, true))?;
        }
        Ok(())
    }

    pub fn args(&mut self, args: &Arguments) -> Result<(), RenderError> {
        match args {
            Arguments::Known(args) => self.args_known(args),
            Arguments::Unavailable { gap, words } => self.out.fmt(format_args!(
                "unavailable_arguments<{gap:?}; original_words={words:?}>"
            )),
        }
    }

    pub fn args_known(&mut self, args: &[ExprId]) -> Result<(), RenderError> {
        for (i, &arg) in args.iter().enumerate() {
            if i != 0 {
                self.out.put(", ")?;
            }
            self.operand(arg)?;
        }
        Ok(())
    }

    pub(super) fn argument_events(
        &self,
        work: &mut Vec<ExprEvent>,
        owner: ExprId,
        arguments: &Arguments,
    ) -> Result<(), RenderError> {
        match arguments {
            Arguments::Known(arguments) => self.known_argument_events(work, arguments),
            Arguments::Unavailable { .. } => {
                push_event(work, ExprEvent::UnavailableArguments(owner))
            }
        }
    }

    pub(super) fn known_argument_events(
        &self,
        work: &mut Vec<ExprEvent>,
        arguments: &[ExprId],
    ) -> Result<(), RenderError> {
        for (index, &argument) in arguments.iter().enumerate().rev() {
            push_event(work, ExprEvent::Expression(argument, true))?;
            if index != 0 {
                push_event(work, ExprEvent::Text(", "))?;
            }
        }
        Ok(())
    }

    pub(super) fn redundant_initializer(
        &self,
        reference: ReferenceId,
        constructor: ReferenceId,
    ) -> bool {
        let (
            Ok(ReferenceDeclaration::Type { type_use }),
            Ok(ReferenceDeclaration::Member(MemberDeclaration::Method { owner, name, .. })),
        ) = (
            &self.code.references()[reference.0 as usize].declaration,
            &self.code.references()[constructor.0 as usize].declaration,
        )
        else {
            return false;
        };
        let initializer = name.units.iter().copied().eq("<init>".encode_utf16());
        initializer
            && self.code.types()[owner.0 as usize].units
                == self.code.types()[type_use.0 as usize].units
    }

    pub fn signature(&mut self, p: &Prototype) -> Result<(), RenderError> {
        self.signature_parts(p, true)
    }

    fn signature_parts(&mut self, p: &Prototype, returns: bool) -> Result<(), RenderError> {
        self.out.put("<(")?;
        for (i, &ty) in p.parameters.iter().enumerate() {
            if i > 0 {
                self.out.put(", ")?;
            }
            self.ty(ty)?;
        }
        self.out.put(")")?;
        if returns {
            self.out.put(" -> ")?;
            self.ty(p.returns)?;
        }
        self.out.put(">")
    }

    pub fn member(
        &mut self,
        r: ReferenceId,
        receiver: Option<ExprId>,
        signature: bool,
    ) -> Result<(), RenderError> {
        let start = self.out.position();
        if let Some(recv) = receiver {
            let owner = self.code.field_receiver_owner(r);
            if owner.is_some() {
                self.out.put("(")?;
            }
            self.operand(recv)?;
            if let Some(owner) = owner {
                self.via(owner)?;
            }
            self.out.put(".")?;
        }
        self.member_end(r, receiver.is_some(), signature, start)
    }

    pub(super) fn call_owner(
        &self,
        r: ReferenceId,
        qualification: Qualification,
    ) -> Option<TypeUseId> {
        if !matches!(
            qualification,
            Qualification::Owner
                | Qualification::Parameters { owner: true }
                | Qualification::FullSignature { owner: true }
        ) {
            return None;
        }
        match &self.code.references()[r.0 as usize].declaration {
            Ok(ReferenceDeclaration::Member(MemberDeclaration::Method { owner, .. })) => {
                Some(*owner)
            }
            _ => None,
        }
    }

    pub(super) fn via(&mut self, owner: TypeUseId) -> Result<(), RenderError> {
        self.out.put(" via ")?;
        self.ty(owner)?;
        self.out.put(")")
    }

    pub(super) fn call_signature(
        &mut self,
        r: ReferenceId,
        qualification: Qualification,
    ) -> Result<(), RenderError> {
        if matches!(qualification, Qualification::Natural | Qualification::Owner) {
            return Ok(());
        }
        match &self.code.references()[r.0 as usize].declaration {
            Ok(ReferenceDeclaration::Member(MemberDeclaration::Method { prototype, .. })) => {
                self.signature_parts(
                    prototype,
                    matches!(qualification, Qualification::FullSignature { .. }),
                )?;
            }
            _ => {
                self.out.put("<")?;
                self.reference(r)?;
                self.out.put(">")?;
            }
        }
        Ok(())
    }

    pub(super) fn call_use_issue(&mut self, r: ReferenceId) -> Result<(), RenderError> {
        let reference = &self.code.references()[r.0 as usize];
        if matches!(
            reference.declaration,
            Ok(ReferenceDeclaration::Member(
                MemberDeclaration::Method { .. }
            ))
        ) && let Some(issue) = &reference.use_issue
        {
            self.out.put(" [reference_use_unavailable: ")?;
            self.out.quote(&issue.detail)?;
            self.out.put("]")?;
        }
        Ok(())
    }

    pub(super) fn call_member_end(
        &mut self,
        r: ReferenceId,
        has_receiver: bool,
        qualification: Qualification,
        start: crate::p9::TextOffset,
    ) -> Result<(), RenderError> {
        let reference = &self.code.references()[r.0 as usize];
        let Ok(ReferenceDeclaration::Member(MemberDeclaration::Method { owner, name, .. })) =
            &reference.declaration
        else {
            return self.member_end(r, has_receiver, false, start);
        };
        if name.units.iter().copied().eq("<init>".encode_utf16()) {
            self.ty(*owner)?;
        } else {
            if !has_receiver {
                self.ty(*owner)?;
                self.out.put(".")?;
            }
            if self.kotlin_notation() {
                self.out.kotlin_name(&name.units)?;
            } else {
                self.out.name(&name.units, name.form)?;
            }
        }
        self.call_signature(r, qualification)?;
        self.call_use_issue(r)?;
        self.out.span(
            start,
            Origin::Reference {
                method: self.method,
                reference: r,
            },
        )
    }

    pub(super) fn member_end(
        &mut self,
        r: ReferenceId,
        has_receiver: bool,
        signature: bool,
        start: crate::p9::TextOffset,
    ) -> Result<(), RenderError> {
        let reference = &self.code.references()[r.0 as usize];
        match &reference.declaration {
            Ok(ReferenceDeclaration::Member(m)) => {
                let (owner, name, initializer) = match m {
                    MemberDeclaration::Field { owner, name, .. } => (*owner, name, false),
                    MemberDeclaration::Method { owner, name, .. } => (
                        *owner,
                        name,
                        name.units.iter().copied().eq("<init>".encode_utf16()),
                    ),
                };
                if initializer {
                    // A constructor reference names the type it constructs.
                    // `name<"<init>">` would hide the owner behind a name that
                    // is not an identifier, and repeating the owner before it
                    // would print the type twice.
                    self.ty(owner)?;
                } else {
                    if !has_receiver {
                        self.ty(owner)?;
                        self.out.put(".")?;
                    }
                    if signature && has_receiver {
                        self.out.put("binding<")?;
                        self.ty(owner)?;
                        self.out.put(">::")?;
                    }
                    if self.kotlin_notation() {
                        self.out.kotlin_name(&name.units)?;
                    } else {
                        self.out.name(&name.units, name.form)?;
                    }
                }
                if signature {
                    match m {
                        MemberDeclaration::Field { field_type, .. } => {
                            self.out.put("<")?;
                            self.names.body(
                                self.out,
                                self.lookup,
                                self.code.types(),
                                *field_type,
                            )?;
                            self.out.put(">")?;
                        }
                        MemberDeclaration::Method { prototype, .. } => self.signature(prototype)?,
                    }
                }
            }
            _ => self.reference(r)?,
        }
        if matches!(reference.declaration, Ok(ReferenceDeclaration::Member(_)))
            && let Some(issue) = &reference.use_issue
        {
            self.out.put(" [reference_use_unavailable: ")?;
            self.out.quote(&issue.detail)?;
            self.out.put("]")?;
        }
        self.out.span(
            start,
            Origin::Reference {
                method: self.method,
                reference: r,
            },
        )
    }

    pub fn call(
        &mut self,
        c: &Call,
        initialization: Option<InitializationSyntax>,
        relation: Option<crate::p4::ConstructorRelation>,
    ) -> Result<(), RenderError> {
        self.outline_call(c.target, c.kind)?;
        if let Some(InitializationSyntax::This | InitializationSyntax::Super) = initialization {
            let start = self.out.position();
            self.out.put(
                if matches!(initialization, Some(InitializationSyntax::This)) {
                    "this"
                } else {
                    "super"
                },
            )?;
            self.call_signature(c.target, c.qualification)?;
            self.call_use_issue(c.target)?;
            self.out.span(
                start,
                Origin::Reference {
                    method: self.method,
                    reference: c.target,
                },
            )?;
            if let Some(receiver) = c.receiver {
                self.out.span(
                    start,
                    Origin::Expression {
                        method: self.method,
                        expression: receiver,
                    },
                )?;
            }
        } else {
            if initialization.is_some() {
                self.out.put("initialize ")?;
            } else if matches!(
                c.kind,
                crate::p2::InvokeKind::Custom | crate::p2::InvokeKind::Polymorphic
            ) {
                self.out.fmt(format_args!("invoke<{:?}> ", c.kind))?;
            }
            // A constructor delegation names the original constructor's owner,
            // never the object under construction: `this`/`super` already say
            // the receiver, so repeating it would claim a `this.<init>` target
            // for a superclass call.
            let receiver = match relation {
                Some(
                    crate::p4::ConstructorRelation::This | crate::p4::ConstructorRelation::Super,
                ) => None,
                _ => c.receiver,
            };
            let start = self.out.position();
            if let Some(receiver) = receiver {
                if self.call_owner(c.target, c.qualification).is_some() {
                    self.out.put("(")?;
                }
                self.operand(receiver)?;
                if let Some(owner) = self.call_owner(c.target, c.qualification) {
                    self.via(owner)?;
                }
                self.out.put(".")?;
            }
            self.call_member_end(c.target, receiver.is_some(), c.qualification, start)?;
            if receiver.is_none()
                && let Some(receiver) = c.receiver
            {
                let start = self.out.position();
                self.out.span(
                    start,
                    Origin::Expression {
                        method: self.method,
                        expression: receiver,
                    },
                )?;
            }
        }
        if let Some(proto) = c.invocation_prototype {
            self.out.put(" [invocation prototype: ")?;
            self.reference(proto)?;
            self.out.put("]")?;
        }
        self.out.put("(")?;
        self.args(&c.arguments)?;
        self.out.put(")")?;
        Ok(())
    }
}
