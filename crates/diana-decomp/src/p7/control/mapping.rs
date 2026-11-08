//! Typed identities mapped from existing P6 roles and lowered P7 statements.

use super::*;
use crate::p7::build::{bug, push};
use crate::p7::values::Context;

pub(super) fn label(c: &Context<'_, '_>, node: p6::NodeId) -> Result<LabelId, ExpressionError> {
    let p = c
        .planned
        .facts()
        .labels()
        .get(node.0 as usize)
        .ok_or(bug("control target lost its P6 label"))?;
    if p.node() != node {
        return Err(bug("control label no longer binds its source node"));
    }
    Ok(LabelId(p.id().0))
}

pub(super) fn statement(c: &Context<'_, '_>, op: OpId) -> Result<StatementId, ExpressionError> {
    c.code
        .operations
        .get(op.0 as usize)
        .and_then(|o| o.statement)
        .ok_or(bug("syntax-owned operation lost its statement"))
}

pub(super) fn terminal(c: &Context<'_, '_>, node: p6::NodeId) -> crate::p2::BlockId {
    c.planned.facts().nodes()[node.0 as usize].block()
}

pub(super) fn ids<T: Copy, U>(
    source: &[T],
    map: impl Fn(T) -> U,
) -> Result<Vec<U>, ExpressionError> {
    let mut out = Vec::new();
    for &v in source {
        push(&mut out, map(v))?;
    }
    Ok(out)
}

pub(super) fn control(value: Option<ControlId>) -> Result<ControlId, ExpressionError> {
    value.ok_or(bug("transfer names an unselected control region"))
}

pub(super) fn continuation(
    c: &Context<'_, '_>,
    source: p6::Continuation,
) -> Result<Continuation, ExpressionError> {
    Ok(match source {
        p6::Continuation::None => Continuation::None,
        p6::Continuation::Parent => Continuation::Parent,
        p6::Continuation::Next(n) => Continuation::Next(label(c, n)?),
        p6::Continuation::Goto(l) => Continuation::Goto(LabelId(l.0)),
        p6::Continuation::Break(scope) => Continuation::Break(ControlId(scope.0)),
        p6::Continuation::Continue(id) => Continuation::Continue(control(c.loops[id.0 as usize])?),
    })
}
