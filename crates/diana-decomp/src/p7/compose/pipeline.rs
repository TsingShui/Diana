//! Ordered display passes over one established emission plan. Selection writes
//! and trace snapshots remain at their original positions in this sequence.

use super::super::build::filled;
use super::super::{ExpressionError, MethodCode};
use super::{
    allocation, arrays, booleans, branches, casts, concatenations, continuations, continues,
    declarations, expressions, fallthrough, labels, literals, loops, references, returns,
    scope_labels, scopes, stores,
};

/// Applies display choices to the established plan before method publication.
pub(in crate::p7) fn construct(
    code: &mut MethodCode,
    types: &crate::p4::TypeFacts,
) -> Result<(), ExpressionError> {
    code.emission.site_annotations = references::site_annotations(code)?;
    casts::prepare(code, types)?;
    let access = super::access::local_access(code)?;
    // Select complete condition alternatives before generic layout and labels.
    expressions::conditions(code, &access)?;
    concatenations::construct(code, types);
    // Sequencing is fixed before labels are selected, so a collapsed jump no
    // longer keeps a display label alive.
    fallthrough::collapse_fallthrough_gotos(code)?;
    scope_labels::unlabel_inner_scopes(code)?;
    continues::drop_trailing_continues(code)?;
    continuations::drop_completed_continuations(code)?;
    continuations::collapse_planned_continuations(code)?;
    loops::construct(code)?;
    let mut selection = labels::Selection::new(code)?;
    let mut chains = filled(code.chains.len(), false)?;
    // A catch clause prints as its `try` clause, so a chain line is only needed
    // when a printed site names it. A chain nothing points at would be a second,
    // unreferenced statement of the same routes.
    selection.exception_routes(code, &mut chains)?;
    code.emission.show_chains = chains;
    code.emission.field_qualifications = references::field_qualifications(code)?;
    selection.select(code)?;

    let adjacency = super::adjacency::Adjacency::construct(code)?;
    expressions::construct(code, &adjacency, &access)?;
    loops::select_compact(code)?;
    stores::natural_stores(code)?;
    literals::omit_materialized_literals(code)?;
    casts::construct(code, &adjacency, &access)?;
    branches::construct(code)?;
    returns::omit_trailing_void_return(code)?;
    arrays::array_literals(code, &adjacency, &access)?;
    allocation::allocation_declarations(code)?;
    scopes::transparent_labelled_scopes(code)?;
    booleans::construct(code, types, &access)?;
    references::call_owners(code)?;
    references::field_owners(code)?;
    declarations::construct(code)?;
    Ok(())
}
