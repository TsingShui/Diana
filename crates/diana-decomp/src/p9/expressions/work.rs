//! Fallible expression events preserve evaluation and source-range order.

use crate::p7::*;
use crate::p9::RenderError;

pub(super) enum ExprEvent {
    Expression(ExprId, bool),
    End(ExprId, crate::p9::TextOffset),
    Anchor(ExprId),
    Text(&'static str),
    Reference(ReferenceId),
    Member(ReferenceId, Option<ExprId>, bool),
    /// `invoke-super` names its target through `super`, so the receiver is the
    /// implicit `this` and is not printed again; its read keeps an anchor.
    SuperMember(ReferenceId, ExprId, Qualification),
    CallMember(ReferenceId, Option<ExprId>, Qualification),
    CallMemberEnd(ReferenceId, bool, Qualification, crate::p9::TextOffset),
    Via(TypeUseId),
    MemberEnd(ReferenceId, bool, bool, crate::p9::TextOffset),
    InvocationPrototype(ReferenceId),
    UnavailableArguments(ExprId),
    ArrayEnd(crate::p2::ValueKind),
    Dimensions(u32),
    Initializer(ReferenceId, ReferenceId),
    CompositionEnd {
        composed: ComposedExpression,
        start: crate::p9::TextOffset,
        previous_carrier: Option<CheckCarrier>,
        previous_statement: Option<StatementId>,
    },
}

pub(super) fn push_event(work: &mut Vec<ExprEvent>, event: ExprEvent) -> Result<(), RenderError> {
    work.try_reserve(1).map_err(RenderError::MethodScratch)?;
    work.push(event);
    Ok(())
}
