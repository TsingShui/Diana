//! Fallible work stack shared by the body renderers.

use super::super::{RenderError, TextOffset};
use crate::p2::{BlockId, OpId};
use crate::p7::{ControlId, PredicateEmission, TransferId};

pub(super) enum Event {
    OriginOnly(bool),
    Finally,
    BeginAbi(&'static str, ControlId),
    EndAbi(ControlId),
    AbiDispatch,
    Control(ControlId),
    ChainedControl(ControlId),
    End(ControlId, TextOffset),
    Close,
    DecisionArm(ControlId, usize),
    If {
        test: PredicateEmission,
        chained: bool,
        omitted_false: bool,
    },
    EmptyIf(PredicateEmission),
    Condition {
        control: ControlId,
        chained: bool,
    },
    /// A delivered control subtree that prints no text still anchors its
    /// original controls, labels, transfers and copies.
    Anchors(ControlId),
    EmptyArmAnchors(ControlId),
    Else(TransferId),
    ElseTransfer(TransferId),
    ElseIf(TransferId),
    LoopHead(PredicateEmission),
    CompactLoopTail(ControlId, PredicateEmission),
    LoopTailStart,
    LoopTailEnd(PredicateEmission),
    Switch(ControlId, BlockId),
    Case(TransferId),
    Catch(ControlId, usize),
    Acquire(ControlId, OpId),
}
pub(super) fn push<T>(work: &mut Vec<T>, event: T) -> Result<(), RenderError> {
    work.try_reserve(1).map_err(RenderError::MethodScratch)?;
    work.push(event);
    Ok(())
}
