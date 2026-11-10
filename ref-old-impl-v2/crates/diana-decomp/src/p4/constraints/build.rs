//! Collects constraints from the existing SSA/execution views. Local missing
//! facts remain in the record that owns them; construction failures retain the
//! same unpublished TypePartial and first-error order.

use super::*;

pub(in crate::p4) fn collect(ssa: &SsaMethod<'_>, f: &mut TypeFacts) -> Result<Work, TypeError> {
    initialize::records(ssa, f)?;
    let (params, returned) = initialize::declarations(ssa, f)?;
    let mut work = Work {
        arrays: Vec::new(),
        casts: Vec::new(),
        instances: Vec::new(),
        rules: Vec::new(),
        compatible: Vec::new(),
    };
    let handler_types = origins::handler_types(ssa)?;
    origins::values(ssa, f, &mut work, &params, &handler_types)?;
    operations::collect(ssa, f, &mut work)?;
    terminals::collect(ssa, f, &mut work, returned)?;
    Ok(work)
}
