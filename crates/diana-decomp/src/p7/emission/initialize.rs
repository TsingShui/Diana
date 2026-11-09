//! Initializes display tables from the closed P7 arena sizes and storage facts.

use super::{EmissionPlan, EvaluationPhases, IfDisplay, LocalSpelling};
use crate::p7::build::{filled, push};
use crate::p7::{ExpressionError, LocalDeclaration, MethodCode, StatementKind, TypeUse};

pub(super) fn plan(code: &MethodCode) -> Result<EmissionPlan, ExpressionError> {
    let mut plan = EmissionPlan::empty();
    plan.show_chains = filled(code.chains.len(), true)?;
    plan.site_annotations = filled(code.sites.len(), true)?;
    plan.field_qualifications = filled(code.references.len(), true)?;
    plan.natural_stores = filled(code.statements.len(), false)?;
    plan.copy_check_uses = filled(code.copies.len(), Vec::new())?;
    plan.definition_check_uses = filled(code.statements.len(), Vec::new())?;
    plan.array_literals = filled(code.statements.len(), None)?;
    plan.omitted_checks = filled(code.statements.len(), None)?;
    plan.omitted_literals = filled(code.statements.len(), false)?;
    plan.omitted_nops = filled(code.statements.len(), false)?;
    for statement in &code.statements {
        if matches!(statement.kind, StatementKind::Nop) {
            plan.omitted_nops[statement.id.0 as usize] = true;
        }
    }
    plan.allocation_declarations = filled(code.statements.len(), false)?;
    plan.allocation_locals = filled(code.locals.len(), false)?;
    plan.hidden_locals = filled(code.locals.len(), false)?;
    for local in &code.locals {
        plan.hidden_locals[local.id.0 as usize] =
            local.original.representative() != local.original.id();
    }
    plan.local_initializers = filled(code.locals.len(), None)?;
    plan.deferred_controls = filled(code.controls.len(), false)?;
    for l in &code.locals {
        let syntax = local_spelling(&code.types, l);
        push(&mut plan.local_types, syntax)?;
    }
    crate::substrate::owned::reserve(&mut plan.operations, code.operations.len())?;
    plan.operations
        .resize_with(code.operations.len(), EvaluationPhases::default);
    crate::substrate::owned::reserve(&mut plan.terminals, code.terminals.len())?;
    plan.terminals
        .resize_with(code.terminals.len(), EvaluationPhases::default);
    plan.ifs = filled(code.terminals.len(), IfDisplay::Both)?;
    plan.omitted_returns = filled(code.terminals.len(), false)?;
    plan.statement_operations = filled(code.statements.len(), None)?;
    plan.origin_only_operations = filled(code.operations.len(), None)?;
    plan.origin_only_terminals = filled(code.terminals.len(), false)?;
    Ok(plan)
}
fn local_spelling(types: &[TypeUse], l: &LocalDeclaration) -> LocalSpelling {
    if matches!(
        l.original.storage_type(),
        crate::p5::StorageType::Family {
            family: crate::p4::Family::Reference,
            ..
        }
    ) && !l.type_source_unknown
        && let Some(&first) = l.declared_types.first()
        && l.declared_types
            .iter()
            .all(|t| types[t.0 as usize].units == types[first.0 as usize].units)
    {
        return LocalSpelling::Named(first);
    }
    LocalSpelling::Machine(l.original.storage_type())
}
