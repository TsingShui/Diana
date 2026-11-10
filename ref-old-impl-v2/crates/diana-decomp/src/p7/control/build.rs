//! Builds owned control facts from mature P6 plans and lowered statements.
//! Each step retains its original partial writes and classified failure; no
//! completed emission layout is implied until the ordinary P7 chain finishes.

use super::{ExpressionError, protection, regions, transfers};
use crate::p7::values::Context;

pub(in crate::p7) fn construct(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    transfers::labels(c)?;
    transfers::transfers(c)?;
    protection::chains(c)?;
    protection::sites(c)?;
    protection::handlers(c)?;
    regions::construct(c)?;
    c.code.root = c
        .planned
        .facts()
        .control_root()
        .map(|r| super::ControlId(r.0));
    Ok(())
}
