//! Lowers the closed P6 coroutine-body alternative. A local marker pattern on
//! its own is not authority to change a method header or remove its ABI dispatch.
use super::build::{bug, copy, id, push};
use super::values::Context;
use super::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Suspension {
    #[serde(serialize_with = "super::debug")]
    pub original: crate::p6::SuspensionPlan,
    pub call: crate::p2::OpId,
    pub statement: StatementId,
    pub expression: ExprId,
    pub target: ReferenceId,
    pub continuation: ExprId,
    pub continuation_argument: u32,
    pub merged: StorageId,
    pub normal_copy: CopyPlanId,
    pub normal_source: ExprId,
    pub control: ControlId,
    pub header: ControlId,
    pub bodies: [ControlId; 2],
    pub predicate: crate::p2::BlockId,
    pub marker_return: crate::p2::BlockId,
    pub links: [TransferId; 2],
}
/// ABI-only source attachments remain ordinary original controls. P9 prints
/// them as comments in the same traversal, not as a second executed body.
#[derive(Debug, Serialize)]
pub struct CoroutineBody {
    pub root: ControlId,
    pub prefix: ControlId,
    pub dispatch: ControlId,
    pub initial: ControlId,
    pub resume: ControlId,
    pub invalid: ControlId,
    pub entry_check: crate::p2::OpId,
    #[serde(serialize_with = "super::debug")]
    pub operations: Vec<crate::p6::CoroutineOperation>,
}

pub(super) fn construct(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    let Some(body) = c.planned.facts().coroutine_body() else {
        return Ok(());
    };
    let original = *c
        .planned
        .facts()
        .suspensions()
        .first()
        .ok_or(bug("closed coroutine body lost its suspension"))?;
    let call = original.call();
    let statement = c.code.operations[call.0 as usize]
        .statement
        .ok_or(bug("suspension call lost its statement"))?;
    let StatementKind::Evaluate { expression } = c.code.statements[statement.0 as usize].kind
    else {
        return Err(bug("suspension call lost its evaluate expression"));
    };
    let ExprKind::Call(source) = &c.code.expressions[expression.0 as usize].kind else {
        return Err(bug("suspension call lost its invoke expression"));
    };
    let Arguments::Known(arguments) = &source.arguments else {
        return Err(bug("suspension lost its known arguments"));
    };
    let argument = original
        .continuation_argument()
        .checked_sub(u32::from(source.receiver.is_some()))
        .ok_or(bug("suspension continuation precedes receiver"))?;
    let continuation = *arguments
        .get(argument as usize)
        .ok_or(bug("suspension lost its continuation occurrence"))?;
    let normal_copy = CopyPlanId(original.normal_copy().0);
    let [
        CopyStatement::Assign {
            expression: normal_source,
            ..
        },
    ] = c.code.copies[normal_copy.0 as usize].steps.as_slice()
    else {
        return Err(bug("suspension lost its normal result copy"));
    };
    let control = ControlId(original.control().0);
    let suspension = Suspension {
        original,
        call,
        statement,
        expression,
        target: source.target,
        continuation,
        continuation_argument: argument,
        merged: StorageId(original.merged().0),
        normal_copy,
        normal_source: *normal_source,
        control,
        header: ControlId(original.header().0),
        bodies: original.bodies().map(|r| ControlId(r.0)),
        predicate: original.predicate(),
        marker_return: original.marker_return(),
        links: original.links().map(|r| TransferId(r.0)),
    };
    let mut sequence = Vec::new();
    for &region in body.sequence() {
        push(&mut sequence, ControlId(region.0))?;
    }
    let owned = CoroutineBody {
        root: ControlId(body.root().0),
        prefix: ControlId(body.prefix().0),
        dispatch: ControlId(body.dispatch().0),
        initial: ControlId(body.initial().0),
        resume: ControlId(body.resume().0),
        invalid: ControlId(body.invalid().0),
        entry_check: body.entry_check(),
        operations: copy(body.operations())?,
    };
    // Keep the state-publication event as a display action and an adjacency
    // barrier. Hiding its text must not let an effectful receiver/argument
    // producer move from before this field write into the suspension call.
    let node = c.op_nodes[original.state_store().0 as usize]
        .ok_or(bug("coroutine state publication lost its original node"))?;
    let action=c.code.emission.blocks[node.0 as usize].actions.iter_mut().find(|action|
        matches!(action,BodyAction::Statement{operation,..} if *operation==original.state_store()))
        .ok_or(bug("coroutine state publication lost its original action"))?;
    let BodyAction::Statement {
        operation,
        statement,
    } = *action
    else {
        return Err(bug("coroutine state action changed kind"));
    };
    let implicit = BodyAction::AbiOperation {
        operation,
        statement,
    };
    let index = id(c.code.emission.suspensions.len())?;
    push(&mut c.code.emission.suspensions, suspension)?;
    *action = implicit;
    c.code.emission.controls[control.0 as usize].layout =
        EmissionLayout::Suspension { plan: index };
    c.code.emission.controls[owned.root.0 as usize].explicit_children = Some(sequence);
    for &local in body.implicit_locals() {
        c.code.emission.hidden_locals[local.0 as usize] = true;
    }
    c.code.emission.coroutine_body = Some(owned);
    Ok(())
}
