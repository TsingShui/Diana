//! A jump to the statement printed right after it is ordinary sequencing.

use super::super::*;
use super::*;

/// A jump to the statement printed immediately after it is ordinary sequencing.
/// Only containers that print their children in order and add nothing after them
/// may pass the search upward; a loop body does not fall through its own test.
pub(super) fn collapse_fallthrough_gotos(code: &mut MethodCode) -> Result<(), ExpressionError> {
    let parent = super::sequence::parents(code)?;
    let mut buffer = filled(code.controls.len().max(3), ControlId(0))?;
    for transfer in 0..code.emission.transfers.len() {
        // Selector transfers print inside the raw label table, not after the
        // owning block. Its next Sequence child is not the next printed case.
        // Selected case layouts already own their supplied arm spellings.
        if matches!(
            code.transfers[transfer].role,
            Some(crate::p2::NormalEdge::SwitchCase { .. } | crate::p2::NormalEdge::SwitchDefault)
        ) {
            continue;
        }
        let TransferSpelling::Goto(label) = code.emission.transfers[transfer].spelling else {
            continue;
        };
        let from = code.transfers[transfer].from;
        let Some(owner) = code.labels.get(from.0 as usize).map(|label| label.owner) else {
            continue;
        };
        let Some(next) = super::sequence::printed_successor(code, &parent, owner, &mut buffer)
        else {
            continue;
        };
        if super::sequence::printed_entry_is(code, next, label) {
            code.emission.transfers[transfer].spelling = TransferSpelling::Fallthrough;
        }
    }
    Ok(())
}
