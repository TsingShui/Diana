//! Statement contents, assignments and selected initialization notation.

use super::literals::machine_type;
use super::{ExpressionSource, Expressions};
use crate::p5;
use crate::p7::*;
use crate::p9::{Origin, RenderError};

impl<S: ExpressionSource> Expressions<'_, '_, S> {
    fn definition_expression(
        &mut self,
        statement: StatementId,
        expression: ExprId,
    ) -> Result<(), RenderError> {
        if !self.kotlin_notation()
            && let Some(check) = self.code.definition_check(statement)
        {
            let start = self.out.position();
            self.out.put("(")?;
            self.ty(check_reference_type(&check, self.code)?)?;
            self.out.put(") ")?;
            self.out.span(
                start,
                Origin::Statement {
                    method: self.method,
                    statement: check.check,
                },
            )?;
            self.out.span(
                start,
                Origin::Operation {
                    method: self.method,
                    operation: check.operation,
                    pc: check.pc,
                },
            )?;
            for &site in self.code.statement_sites(check.check) {
                self.out.span(
                    start,
                    Origin::Site {
                        method: self.method,
                        site,
                    },
                )?;
            }
        }
        self.expr(expression)?;
        Ok(())
    }

    fn field_store(
        &mut self,
        statement: StatementId,
        reference: ReferenceId,
        receiver: Option<ExprId>,
        value: ExprId,
    ) -> Result<(), RenderError> {
        let qualified = self.code.field_qualification(reference);
        if self.code.natural_store(statement) {
            self.member(reference, receiver, qualified)?;
            self.out.put(" = ")?;
            self.operand(value)?;
        } else {
            self.out.put("store ")?;
            self.operand(value)?;
            self.out.put(" -> ")?;
            self.member(reference, receiver, qualified)?;
        }
        Ok(())
    }

    fn array_store(
        &mut self,
        statement: StatementId,
        array: ExprId,
        index: ExprId,
        value: ExprId,
        kind: crate::p2::ValueKind,
    ) -> Result<(), RenderError> {
        if self.code.natural_store(statement) {
            self.operand(array)?;
            self.out.put("[")?;
            self.operand(index)?;
            self.out.put("] = ")?;
            self.operand(value)?;
            // Only a narrowing store needs its original width kept.
            if matches!(
                kind,
                crate::p2::ValueKind::Byte
                    | crate::p2::ValueKind::Char
                    | crate::p2::ValueKind::Short
            ) {
                self.out.fmt(format_args!(" /* {kind:?} store */"))?;
            }
        } else {
            self.out.put("store ")?;
            self.operand(value)?;
            self.out.put(" -> ")?;
            self.operand(array)?;
            self.out.put("[")?;
            self.operand(index)?;
            self.out.put("]")?;
            self.out.fmt(format_args!(" /* {kind:?} store */"))?;
        }
        Ok(())
    }

    fn check_cast(&mut self, value: ExprId, reference: ReferenceId) -> Result<(), RenderError> {
        // A cast statement keeps the check at its original position and
        // writes no register; the value keeps its own local.
        // A standalone check prints as the cast expression it is: the
        // Java-like form keeps the same syntax as an inlined check, and
        // the Kotlin view uses `as`, never the null-returning `as?`.
        if self.kotlin_notation() {
            self.out.put("(")?;
            self.operand(value)?;
            self.out.put(" as ")?;
            self.kotlin_cast_reference(reference)?;
            self.out.put(")")?;
        } else {
            self.out.put("((")?;
            self.reference(reference)?;
            self.out.put(") ")?;
            self.operand(value)?;
            self.out.put(")")?;
        }
        Ok(())
    }

    fn initialization(
        &mut self,
        expression: ExprId,
        syntax: &InitializationSyntax,
        relation: &crate::p4::ConstructorRelation,
    ) -> Result<(), RenderError> {
        let ExprKind::Call(c) = &self.code.expressions()[expression.0 as usize].kind else {
            return Err(RenderError::Invariant("initialization lost its invocation"));
        };
        let a = self.out.position();
        // An explicit constructor run on an allocated object keeps that
        // object and its own position: `<init>` names the call without
        // repeating the type the allocation already stated.
        // A constructor's own delegation names its target by relation:
        // `super(...)` calls the superclass initializer and `this(...)`
        // another initializer of the same class.
        let syntax = match (syntax, relation) {
            (InitializationSyntax::Explicit, crate::p4::ConstructorRelation::Super) => {
                InitializationSyntax::Super
            }
            (InitializationSyntax::Explicit, crate::p4::ConstructorRelation::This) => {
                InitializationSyntax::This
            }
            (syntax, _) => *syntax,
        };
        let explicit = matches!(syntax, InitializationSyntax::Explicit);
        if explicit && let Some(receiver) = c.receiver {
            self.outline_call(c.target, c.kind)?;
            if self.call_owner(c.target, c.qualification).is_some() {
                self.out.put("(")?;
            }
            self.operand(receiver)?;
            if let Some(owner) = self.call_owner(c.target, c.qualification) {
                self.via(owner)?;
            }
            self.out.put(".<init>")?;
            self.call_signature(c.target, c.qualification)?;
            self.call_use_issue(c.target)?;
            self.out.put("(")?;
            self.args(&c.arguments)?;
            self.out.put(")")?;
            let start = a;
            self.out.span(
                start,
                Origin::Reference {
                    method: self.method,
                    reference: c.target,
                },
            )?;
            if let Some(proto) = c.invocation_prototype {
                self.out.put(" [invocation prototype: ")?;
                self.reference(proto)?;
                self.out.put("]")?;
            }
        } else {
            self.call(c, Some(syntax), Some(*relation))?;
        }
        self.out.span(
            a,
            Origin::Expression {
                method: self.method,
                expression,
            },
        )?;
        Ok(())
    }
    /// A construction whose only publication is its own deferred carrier is an
    /// ordinary assignment: Java assigns the local only after `new` completes, so
    /// no separate success marker is needed.
    fn deferred_publication(&self, s: &Statement) -> Option<StorageId> {
        let StatementKind::Construction(group) = s.kind else {
            return None;
        };
        let group = self.code.constructions().get(group.0 as usize)?;
        let [publication] = group.publications.as_slice() else {
            return None;
        };
        let mut carrier = None;
        for definition in &s.outputs {
            if let p5::Materialization::Deferred { local, .. } = definition.materialization() {
                carrier = Some(StorageId(local.0));
            }
        }
        (carrier == Some(publication.destination)).then_some(publication.destination)
    }

    pub fn assignment(&mut self, s: &Statement) -> Result<(), RenderError> {
        let deferred = self.deferred_publication(s);
        let mut first = true;
        for definition in &s.outputs {
            let local = match definition.materialization() {
                p5::Materialization::Store(local)
                | p5::Materialization::AllocationCarrier(local)
                | p5::Materialization::ExceptionCapture(local) => Some(local),
                p5::Materialization::Deferred { local, .. }
                    if deferred == Some(StorageId(local.0)) =>
                {
                    Some(local)
                }
                _ => None,
            };
            if let Some(local) = local {
                if !first {
                    self.out.put(", ")?;
                }
                let local = StorageId(local.0);
                if let Some((statement, spelling)) = self.code.local_initializer(local)
                    && statement == s.id
                {
                    if self.kotlin_notation() {
                        self.out.put("var ")?;
                        self.local(local)?;
                        self.out.put(": ")?;
                        self.kotlin_local_type(spelling)?;
                        first = false;
                        continue;
                    }
                    match spelling {
                        LocalSpelling::Named(ty) => self.ty(ty)?,
                        LocalSpelling::Machine(ty) => machine_type(self.out, ty)?,
                    }
                    self.out.put(" ")?;
                }
                self.local(local)?;
                first = false;
            }
        }
        if !first {
            self.out.put(" = ")?;
        }
        Ok(())
    }

    pub fn semantic(&mut self, s: &Statement) -> Result<(), RenderError> {
        match &s.kind {
            StatementKind::Nop => self.out.put("nop")?,
            StatementKind::Move { expression, .. } | StatementKind::Evaluate { expression } => {
                self.definition_expression(s.id, *expression)?
            }
            StatementKind::FieldStore {
                reference,
                receiver,
                value,
            } => self.field_store(s.id, *reference, *receiver, *value)?,
            StatementKind::ArrayStore {
                array,
                index,
                value,
                kind,
            } => self.array_store(s.id, *array, *index, *value, *kind)?,
            StatementKind::CheckCast { value, reference } => self.check_cast(*value, *reference)?,
            StatementKind::FillArray { array, payload } => {
                self.out.put("fill_array ")?;
                self.operand(*array)?;
                let p = &self.code.payloads()[payload.0 as usize];
                self.out.fmt(format_args!(
                    " from payload<width={}; count={}; element={:?}>[",
                    p.width, p.count, p.element
                ))?;
                for (i, bits) in p.bits.iter().enumerate() {
                    if i > 0 {
                        self.out.put(", ")?;
                    }
                    self.out.fmt(format_args!("0x{bits:x}"))?;
                }
                self.out.put("]")?;
            }
            StatementKind::Monitor { enter, lock } => {
                self.out.put(if *enter {
                    "monitor_enter "
                } else {
                    "monitor_exit "
                })?;
                self.operand(*lock)?;
            }
            StatementKind::CaptureException => self.out.put("capture_exception")?,
            StatementKind::Initialize {
                expression,
                syntax,
                relation,
            } => self.initialization(*expression, syntax, relation)?,
            StatementKind::UnresolvedResult { width } => self
                .out
                .fmt(format_args!("unavailable_result<{width:?}>"))?,
            StatementKind::Construction(group) => {
                let g = &self.code.constructions()[group.0 as usize];
                self.expr(g.expression)?;
                if self.deferred_publication(s).is_none() {
                    self.out.put(" [on_success publish (")?;
                    for (i, p) in g.publications.iter().enumerate() {
                        if i > 0 {
                            self.out.put(", ")?;
                        }
                        self.local(p.destination)?;
                    }
                    self.out.put(")]")?;
                }
            }
        }
        Ok(())
    }
}

/// The type a check names, from its own original type reference.
fn check_reference_type<S: ExpressionSource>(
    check: &InlineCheck,
    code: &S,
) -> Result<TypeUseId, RenderError> {
    match code
        .references()
        .get(check.reference.0 as usize)
        .map(|reference| &reference.declaration)
    {
        Some(Ok(ReferenceDeclaration::Type { type_use })) => Ok(*type_use),
        _ => Err(RenderError::Invariant(
            "definition check lost its type reference",
        )),
    }
}
