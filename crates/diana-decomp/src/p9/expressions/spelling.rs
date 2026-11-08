//! Existing local, type and reference spellings in Java-like and Kotlin views.

use super::{ExpressionSource, Expressions};
use crate::p7::*;
use crate::p9::{Origin, RenderError};

impl<S: ExpressionSource> Expressions<'_, '_, S> {
    pub(in crate::p9) fn kotlin_notation(&self) -> bool {
        self.code.kotlin_notation() && !self.out.abi_comment
    }

    pub(in crate::p9) fn kotlin_type(&mut self, id: TypeUseId) -> Result<(), RenderError> {
        if let Some(named) = self.names.body.get(id.0 as usize).copied().flatten() {
            self.lookup.print_kotlin(self.out, named)
        } else {
            self.ty(id)
        }
    }

    pub(super) fn kotlin_cast_reference(&mut self, id: ReferenceId) -> Result<(), RenderError> {
        let start = self.out.position();
        let reference = &self.code.references()[id.0 as usize];
        if reference.use_issue.is_some() {
            return self.reference(id);
        }
        match &reference.declaration {
            Ok(ReferenceDeclaration::Type { type_use }) => self.kotlin_type(*type_use)?,
            _ => return self.reference(id),
        }
        self.out.span(
            start,
            Origin::Reference {
                method: self.method,
                reference: id,
            },
        )
    }

    pub fn local(&mut self, id: StorageId) -> Result<(), RenderError> {
        let l = self
            .code
            .locals()
            .get(id.0 as usize)
            .ok_or(RenderError::Invariant(
                "expression lost its local declaration",
            ))?;
        if self.kotlin_notation()
            && !matches!(
                l.original.kind(),
                crate::p5::LocalKind::Parameter {
                    role: crate::p3::EntryRole::This,
                    ..
                }
            )
        {
            self.out.kotlin_name_str(&l.name)
        } else {
            self.out.put(&l.name)
        }
    }

    pub fn ty(&mut self, id: TypeUseId) -> Result<(), RenderError> {
        if self.kotlin_notation()
            && let Some(named) = self.names.body.get(id.0 as usize).copied().flatten()
            && self.lookup.dimensions(named) == Some(0)
        {
            return self.lookup.print_kotlin_base(self.out, named);
        }
        self.names
            .body(self.out, self.lookup, self.code.types(), id)
    }

    pub fn reference(&mut self, r: ReferenceId) -> Result<(), RenderError> {
        let reference = self
            .code
            .references()
            .get(r.0 as usize)
            .ok_or(RenderError::Invariant("reference exceeds the owned arena"))?;
        let start = self.out.position();
        match &reference.declaration {
            Err(error) => {
                self.out.fmt(format_args!(
                    "unavailable_reference<{:?}; ",
                    reference.position.reference
                ))?;
                self.out.quote(&error.detail)?;
                self.out.put(">")?;
            }
            Ok(ReferenceDeclaration::String { units }) => self.out.units(units, true)?,
            Ok(ReferenceDeclaration::Type { type_use }) => self.ty(*type_use)?,
            Ok(ReferenceDeclaration::Member(_)) => self.member(r, None, true)?,
            Ok(ReferenceDeclaration::Prototype(p)) => {
                self.out.fmt(format_args!(
                    "method_type<{:?}>",
                    reference.position.reference
                ))?;
                self.signature(p)?;
            }
            Ok(ReferenceDeclaration::MethodHandle {
                kind,
                member,
                declaration,
            }) => {
                self.out.fmt(format_args!(
                    "method_handle<{:?}; kind={kind}; {member:?}; ",
                    reference.position.reference
                ))?;
                match declaration {
                    MemberDeclaration::Field {
                        owner,
                        name,
                        field_type,
                    } => {
                        self.ty(*owner)?;
                        self.out.put(".")?;
                        self.out.name(&name.units, name.form)?;
                        self.out.put(": ")?;
                        self.ty(*field_type)?;
                    }
                    MemberDeclaration::Method {
                        owner,
                        name,
                        prototype,
                    } => {
                        // A constructor handle names the type it constructs,
                        // the same way an ordinary constructor reference does.
                        self.ty(*owner)?;
                        if !name.units.iter().copied().eq("<init>".encode_utf16()) {
                            self.out.put(".")?;
                            self.out.name(&name.units, name.form)?;
                        }
                        self.signature(prototype)?;
                    }
                }
                self.out.put(">")?;
            }
            Ok(ReferenceDeclaration::CallSite { .. }) => self.out.fmt(format_args!(
                "call_site<r{}; {:?}>",
                r.0, reference.position.reference
            ))?,
        }
        if let Some(issue) = &reference.use_issue {
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
}
