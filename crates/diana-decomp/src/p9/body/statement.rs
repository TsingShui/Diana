//! Statements, their phases, sites and generated notes.

use super::super::expressions::Expressions;
use super::super::{Origin, RenderError, TextOffset};
use crate::p2::OpId;
use crate::p5::Materialization;
use crate::p7::*;

impl Expressions<'_, '_> {
    fn statement_content(
        &mut self,
        op: OpId,
        s: &Statement,
        release: Option<ControlId>,
    ) -> Result<(), RenderError> {
        if let Some(scope) = release {
            let control = &self.code.controls()[scope.0 as usize];
            self.out.put("release synchronized ")?;
            // A release names its control only when that control prints the
            // label; otherwise the enclosing synchronized header identifies it.
            if control.label_required {
                self.out.put(&control.label)?;
            }
            self.out.put("(")?;
            let StatementKind::Monitor { lock, .. } = s.kind else {
                return Err(RenderError::Invariant("release syntax lost its monitor"));
            };
            self.operand(lock)?;
            self.out.put(")")?;
        } else {
            // Only the statement's own operands may carry an inlined check: the
            // copies printed before it and any snapshot value after it are not
            // the check's first use.
            let previous = self.check_carrier.replace(CheckCarrier::Statement(s.id));
            let statement = self.statement.replace(s.id);
            let rendered = if let Some(plan) = self.code.emission().suspension(op) {
                self.suspension_assignment(plan)
            } else if self.code.emission().allocation_declaration(s.id) {
                self.allocation_declaration(s)
            } else {
                self.assignment(s).and_then(|()| self.semantic(s))
            };
            self.check_carrier = previous;
            self.statement = statement;
            rendered?;
        }
        Ok(())
    }

    fn statement_notes(&mut self, op: OpId, s: &Statement) -> Result<(), RenderError> {
        if let Some(site) = self
            .code
            .emission()
            .coroutine_flow()
            .and_then(|f| f.sites.iter().find(|p| p.call == op))
        {
            self.out.fmt(format_args!(
                " // Diana: inferred suspension, resume state={}; ABI ",
                site.state
            ))?;
            let previous = std::mem::replace(&mut self.out.abi_comment, true);
            let rendered = self.member(site.target, None, true);
            self.out.abi_comment = previous;
            rendered?;
        }
        if let Some(plan) = self
            .code
            .emission()
            .coroutine_loop()
            .filter(|p| p.call == op)
        {
            self.out.fmt(format_args!(
                " // Diana: inferred suspension, resume state={}; ABI ",
                plan.state
            ))?;
            let previous = std::mem::replace(&mut self.out.abi_comment, true);
            let rendered = self.member(plan.target, None, true);
            self.out.abi_comment = previous;
            rendered?;
        }
        if let Some(plan) = self.code.emission().suspension(op) {
            self.out.fmt(format_args!(
                " // Diana: inferred suspension, continuation=this, resume state={}; ABI ",
                plan.original.state()
            ))?;
            let previous = std::mem::replace(&mut self.out.abi_comment, true);
            let binding = self.member(plan.target, None, true);
            self.out.abi_comment = previous;
            binding?;
        }
        if let Some(note) = self.generated_note(s) {
            self.out.put(" ")?;
            self.out.put(note)?;
        }
        for definition in &s.outputs {
            if let Materialization::Unavailable(gap) = definition.materialization() {
                self.out
                    .fmt(format_args!(" unavailable_definition<{gap:?}>;"))?;
            }
        }
        Ok(())
    }

    /// A trailing note for a statement printed in Diana syntax rather than Java.
    /// It names the original instruction and the one fact the form hides.
    pub(super) fn generated_note(&self, s: &Statement) -> Option<&'static str> {
        match &s.kind {
            StatementKind::CheckCast { .. } => Some("// Diana: check-cast, value not reassigned"),
            StatementKind::CaptureException => Some("// Diana: move-exception, caught throwable"),
            StatementKind::Initialize {
                syntax: InitializationSyntax::Explicit,
                ..
            } => None,
            StatementKind::Move { expression, .. } | StatementKind::Evaluate { expression } => {
                match &self.code.expressions().get(expression.0 as usize)?.kind {
                    ExprKind::AllocateInstance { .. } => {
                        Some("// Diana: new-instance, object exists before its constructor")
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub(super) fn statement(
        &mut self,
        op: OpId,
        release: Option<ControlId>,
    ) -> Result<(), RenderError> {
        if self.code.nested_operation(op) {
            return Ok(());
        }
        let original = &self.code.operations()[op.0 as usize];
        let id = original.statement.ok_or(RenderError::Invariant(
            "emission operation has no statement",
        ))?;
        let s = &self.code.statements()[id.0 as usize];
        let phases = &self.code.emission().operations()[op.0 as usize];
        if release.is_none() && self.code.emission().omitted_literal(id) {
            return self.literal_origins(op, original, s);
        }
        if self.code.emission().omitted_nop(id) {
            return self.nop_origins(op, original, s);
        }
        if release.is_none()
            && let Some(omitted) = self.code.emission().omitted_check(id)
        {
            return self.omitted_check_origins(original, s, omitted);
        }
        self.out.indent()?;
        self.copies(&phases.before)?;
        let start = self.out.position();
        if matches!(s.kind, StatementKind::Construction(_)) {
            self.copies(&phases.snapshots)?;
        }
        self.statement_content(op, s, release)?;
        if !matches!(s.kind, StatementKind::Construction(_)) {
            self.phases(&phases.snapshots, "after_operands_before_effect")?;
        }
        self.sites(&s.sites, start)?;
        self.out.put(";")?;
        self.statement_notes(op, s)?;
        self.statement_origins(op, original, s, start)?;
        if !phases.after.is_empty() {
            self.out.put(" ")?;
            self.copies(&phases.after)?;
        }
        self.out.put("\n")
    }

    /// A new-instance statement states the allocated type and its local where the
    /// original allocation runs. The constructor call stays at its own position,
    /// so the object is never claimed to be initialized here.
    pub(super) fn allocation_declaration(&mut self, s: &Statement) -> Result<(), RenderError> {
        let StatementKind::Evaluate { expression } = s.kind else {
            return Err(RenderError::Invariant(
                "allocation declaration lost its evaluate statement",
            ));
        };
        let ExprKind::AllocateInstance { reference } = self
            .code
            .expressions()
            .get(expression.0 as usize)
            .ok_or(RenderError::Invariant(
                "allocation declaration lost its expression",
            ))?
            .kind
        else {
            return Err(RenderError::Invariant(
                "allocation declaration lost its new-instance",
            ));
        };
        let [output] = s.outputs.as_slice() else {
            return Err(RenderError::Invariant(
                "allocation declaration lost its publication",
            ));
        };
        let local = match output.materialization() {
            crate::p5::Materialization::Store(local)
            | crate::p5::Materialization::AllocationCarrier(local) => StorageId(local.0),
            _ => {
                return Err(RenderError::Invariant(
                    "allocation declaration lost its local",
                ));
            }
        };
        let start = self.out.position();
        self.reference(reference)?;
        self.out.put(" ")?;
        self.local(local)?;
        self.out.put(" = new ")?;
        // No argument list: the constructor has not run at this position, so a
        // `()` would claim a default constructor that this allocation never ran.
        self.reference(reference)?;
        self.out.span(
            start,
            Origin::Expression {
                method: self.method,
                expression,
            },
        )
    }

    pub(super) fn sites(
        &mut self,
        sites: &[SiteId],
        statement: TextOffset,
    ) -> Result<(), RenderError> {
        // One statement can own several original sites from one chain, as a
        // contracted construction does. The marker names only that chain, so its
        // text prints once; every site keeps its own zero-length anchor.
        let mut printed: Vec<u32> = Vec::new();
        for &id in sites {
            let s = &self.code.sites()[id.0 as usize];
            let origin = Origin::Site {
                method: self.method,
                site: id,
            };
            if !self.code.emission().site_annotation(id) {
                // The route is already printed as the enclosing catch scope.
                self.out.span(statement, origin)?;
                continue;
            }
            if printed.contains(&s.chain.0) {
                self.out.span(statement, origin)?;
                continue;
            }
            printed.try_reserve(1).map_err(RenderError::MethodScratch)?;
            printed.push(s.chain.0);
            let start = self.out.position();
            // The printed chain line carries clause order, catch types and
            // handler targets; the site only names the chain it selects.
            self.out.fmt(format_args!(" [exn c{}]", s.chain.0))?;
            self.out.span(start, origin)?;
        }
        Ok(())
    }
}
