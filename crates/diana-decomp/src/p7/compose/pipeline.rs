//! Ordered display passes over one established emission plan. Selection writes
//! and trace snapshots remain at their original positions in this sequence.

use super::super::build::filled;
use super::super::{ExpressionError, MethodCode};
use super::{
    allocation, arrays, booleans, branches, casts, continuations, continues, declarations,
    expressions, fallthrough, labels, literals, loops, references, returns, scope_labels, scopes,
    stores, trace,
};

/// Applies display choices to the established plan before method publication.
pub(in crate::p7) fn construct(
    code: &mut MethodCode,
    types: &crate::p4::TypeFacts,
    key: crate::MethodKey,
) -> Result<(), ExpressionError> {
    // One report per pass that changed something, and only under the
    // `trace-passes` feature. The counters come from the plan itself, so a pass
    // need not describe its own effect.
    macro_rules! traced {
        ($name:literal, $call:expr) => {{
            let before = trace::ENABLED.then(|| trace::snapshot(code));
            let value = $call?;
            if let Some(before) = before {
                trace::report(key, $name, &before, &trace::snapshot(code));
            }
            value
        }};
    }
    // Select complete condition alternatives before generic layout and labels.
    traced!("conditions", expressions::conditions(code, key));
    // Sequencing is fixed before labels are selected, so a collapsed jump no
    // longer keeps a display label alive.
    traced!(
        "collapse_fallthrough_gotos",
        fallthrough::collapse_fallthrough_gotos(code)
    );
    traced!(
        "unlabel_inner_scopes",
        scope_labels::unlabel_inner_scopes(code)
    );
    traced!(
        "drop_trailing_continues",
        continues::drop_trailing_continues(code)
    );
    traced!(
        "drop_completed_continuations",
        continuations::drop_completed_continuations(code)
    );
    traced!(
        "collapse_planned_continuations",
        continuations::collapse_planned_continuations(code)
    );
    traced!("loops", loops::construct(code));
    let mut selection = labels::Selection::new(code)?;
    let mut chains = filled(code.chains.len(), false)?;
    // A catch clause prints as its `try` clause, so a chain line is only needed
    // when a printed site names it. A chain nothing points at would be a second,
    // unreferenced statement of the same routes.
    code.emission.site_annotations =
        traced!("site_annotations", references::site_annotations(code));
    selection.exception_routes(code, &mut chains)?;
    code.emission.show_chains = chains;
    code.emission.field_qualifications = traced!(
        "field_qualifications",
        references::field_qualifications(code)
    );
    selection.select(code)?;

    traced!("expressions", expressions::construct(code, key));
    traced!("loop_compact", loops::select_compact(code));
    traced!("natural_stores", stores::natural_stores(code));
    literals::omit_materialized_literals(code)?;
    traced!("casts", casts::construct(code, types, key));
    traced!("branches", branches::construct(code));
    traced!(
        "omit_trailing_void_return",
        returns::omit_trailing_void_return(code)
    );
    traced!("array_literals", arrays::array_literals(code));
    traced!(
        "allocation_declarations",
        allocation::allocation_declarations(code)
    );
    traced!(
        "transparent_scopes",
        scopes::transparent_labelled_scopes(code)
    );
    booleans::construct(code, types)?;
    references::call_owners(code)?;
    references::field_owners(code)?;
    declarations::construct(code)?;
    Ok(())
}
