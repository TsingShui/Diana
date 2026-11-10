//! Consumes closed P7 arenas lowered from P5/P6. Construction scratch stays
//! private; the completed plan replaces the prior value only after every step
//! succeeds. Errors retain the existing P7 classification and partial facts.

use super::{EmissionPlan, blocks, controls, initialize, phases};
use crate::p7::{ExprKind, ExpressionError, MethodCode};

pub(in crate::p7) fn construct(code: &mut MethodCode) -> Result<(), ExpressionError> {
    let mut plan = initialize::plan(code)?;
    let edge_links = phases::transfers(code, &mut plan)?;
    phases::copies(code, &mut plan, &edge_links)?;
    blocks::construct(code, &mut plan)?;
    controls::construct(code, &mut plan)?;
    controls::complete(code, &mut plan)?;
    controls::surfaces(code, &mut plan)?;
    identity_moves(code, &mut plan);
    code.emission = plan;
    Ok(())
}

fn identity_moves(code: &MethodCode, plan: &mut EmissionPlan) {
    // A reused identity move keeps its operation/statement origins at this
    // point; it performs no remaining storage write or observable evaluation.
    for op in &code.operations {
        let Some(statement) = op.statement else {
            continue;
        };
        let Some(s) = code.statement(statement) else {
            continue;
        };
        let ExprKind::Move { expression, .. } = *s.operator(code) else {
            continue;
        };
        let [definition] = s.outputs.as_slice() else {
            continue;
        };
        let crate::p5::Materialization::Store(local) = definition.materialization() else {
            continue;
        };
        let ExprKind::Read {
            storage,
            view: crate::p4::Interpretation::Direct(family),
        } = code.expressions[expression.0 as usize].kind
        else {
            continue;
        };
        let phases = &plan.operations[op.source.0 as usize];
        if storage.0 == local.0
            && code.locals[local.0 as usize].original.is_reused()
            && matches!(code.locals[local.0 as usize].original.storage_type(),
                crate::p5::StorageType::Family { family: stored, .. } if stored == family)
            && phases.before.is_empty()
            && phases.snapshots.is_empty()
            && phases.after.is_empty()
            && s.sites.is_empty()
        {
            plan.placements[op.source.0 as usize].origin_only =
                Some(super::OriginOnly::Identity(expression));
        }
    }
}
