//! Boolean display consumes P4's range-proved declared views.
use super::super::build::filled;
use super::super::*;
use super::access::{LocalAccess, LocalProtection};
use crate::p4::{Family, Interpretation, TypeFacts};
use crate::p5::{LocalKind, Materialization};

fn boolean_type(code: &MethodCode, id: TypeUseId) -> bool {
    code.types
        .get(id.0 as usize)
        .is_some_and(|ty| ty.units == [u16::from(b'Z')])
}

pub(super) fn construct(
    code: &mut MethodCode,
    types: &TypeFacts,
    access: &LocalAccess,
) -> Result<(), ExpressionError> {
    let mut uses = filled(code.expressions.len(), false)?;
    let mut eligible = filled(code.locals.len(), true)?;
    let mut reads = filled(code.locals.len(), 0usize)?;
    let mut identities = filled(code.locals.len(), None)?;
    for expression in &code.expressions {
        let view = match expression.kind {
            ExprKind::Read { view, .. } | ExprKind::Literal { view, .. } => Some(view),
            _ => None,
        };
        uses[expression.id.0 as usize] = view.is_some_and(|view| {
            matches!(view,
            Interpretation::IntegralView { declared } if boolean_type(code, TypeUseId(declared.0)))
        });
        if code.evaluation(expression.id).is_some_and(|composition| {
            !matches!(composition.predicate, EvaluationPredicate::Boolean)
        }) {
            uses[expression.id.0 as usize] = false;
        }
        match expression.kind {
            ExprKind::Read { storage, .. } => {
                reads[storage.0 as usize] += 1;
                eligible[storage.0 as usize] &= uses[expression.id.0 as usize];
                match expression
                    .origin
                    .as_ref()
                    .and_then(|origin| origin.read.ok())
                {
                    Some(crate::p5::StorageRead::Value { value, state, .. }) => {
                        let identity = (value, state);
                        if identities[storage.0 as usize]
                            .is_some_and(|previous| previous != identity)
                        {
                            eligible[storage.0 as usize] = false;
                        }
                        identities[storage.0 as usize] = Some(identity);
                    }
                    _ => eligible[storage.0 as usize] = false,
                }
            }
            ExprKind::Snapshot { storage } => eligible[storage.0 as usize] = false,
            ExprKind::Literal { bits, .. } => {
                uses[expression.id.0 as usize] &=
                    bits.width == crate::SlotWidth::One && bits.value <= 1
            }
            _ => {}
        }
    }
    // Copy expressions retain the destination's machine storage contract.
    for copy in &code.copies {
        for step in &copy.steps {
            if let CopyStatement::Assign { expression, .. } = *step {
                uses[expression.0 as usize] = false;
                if let ExprKind::Read { storage, .. } = code.expressions[expression.0 as usize].kind
                {
                    eligible[storage.0 as usize] = false;
                }
            }
        }
    }
    code.emission.boolean_uses = uses;
    code.emission.boolean_locals = filled(code.locals.len(), false)?;
    for local in &code.locals {
        let index = local.id.0 as usize;
        if matches!(local.original.kind(), LocalKind::Parameter { .. })
            && !local.declared_types.is_empty()
            && !local.type_source_unknown
            && local
                .declared_types
                .iter()
                .all(|&ty| boolean_type(code, ty))
        {
            // The method declaration already spells this parameter boolean.
            code.emission.boolean_locals[index] = true;
        }
    }
    if !access.complete {
        return Ok(());
    };
    let mut definitions = filled(code.locals.len(), None)?;
    let mut multiple = filled(code.locals.len(), false)?;
    for statement in code.statements() {
        for output in &statement.outputs {
            if let Materialization::Store(local) = output.materialization() {
                let index = local.0 as usize;
                multiple[index] |= definitions[index].replace(statement.id).is_some();
            }
        }
    }
    for local in &code.locals {
        let index = local.id.0 as usize;
        if !eligible[index]
            || reads[index] == 0
            || local.gap.is_some()
            || local.type_source_unknown
            || !matches!(access.protection[index], LocalProtection::None)
        {
            continue;
        }
        let LocalKind::Value(value) = local.original.kind() else {
            continue;
        };
        if !types
            .value(value)
            .and_then(|value| value.integral_range())
            .is_some_and(|range| range.min >= 0 && range.max <= 1)
        {
            continue;
        }
        let Some(&ty) = local.declared_types.first() else {
            continue;
        };
        if !local
            .declared_types
            .iter()
            .all(|&ty| boolean_type(code, ty))
        {
            continue;
        }
        let Some(statement) = definitions[index] else {
            continue;
        };
        let statement = &code.require_statement(statement)?;
        if multiple[index]
            || statement.outputs.len() != 1
            || identities[index] != Some((value, statement.outputs[0].state()))
        {
            continue;
        }
        let expression = statement.expression;
        let natural_boolean = match &code.expressions[expression.0 as usize].kind {
            ExprKind::Call(call) => matches!(&code.references[call.target.0 as usize].declaration,
                Ok(ReferenceDeclaration::Member(MemberDeclaration::Method { prototype, .. })) if boolean_type(code, prototype.returns)),
            ExprKind::Field { reference, .. } => {
                matches!(&code.references[reference.0 as usize].declaration,
                Ok(ReferenceDeclaration::Member(MemberDeclaration::Field { field_type, .. })) if boolean_type(code, *field_type))
            }
            ExprKind::InstanceOf { .. } => true,
            _ => false,
        };
        if !natural_boolean {
            continue;
        }
        if !matches!(
            local.original.storage_type(),
            crate::p5::StorageType::Family {
                family: Family::Integral,
                ..
            }
        ) {
            continue;
        }
        code.emission.local_types[index] = LocalSpelling::Named(ty);
        code.emission.boolean_locals[index] = true;
    }
    Ok(())
}
