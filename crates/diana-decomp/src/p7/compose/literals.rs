//! A literal move already prints where its value is read.

use super::super::*;

/// A move whose value prints only at its use sites states nothing on its own:
/// every output materializes as a literal, so the reader sees the constant where
/// it is read and the bare `null;` line carries no fact. Such a statement is
/// admitted only when it has no exception site, no snapshot and no copy phase.
/// Its statement, operation, PC, result commit and expression keep zero-length
/// anchors at the original position; nothing moves and nothing is deleted.
pub(super) fn omit_materialized_literals(code: &mut MethodCode) -> Result<(), ExpressionError> {
    for operation in &code.operations {
        let Some(statement) = operation.body.as_ref() else {
            continue;
        };
        let expression = statement.value(code);
        if !statement.sites.is_empty() {
            continue;
        }
        if !matches!(
            code.expressions.get(expression.0 as usize).map(|e| &e.kind),
            Some(ExprKind::Literal { .. })
        ) {
            continue;
        }
        // Every printed use holds the literal itself, so no output names this
        // statement. A store or an unavailable gap still needs its text.
        if statement.outputs.iter().any(|output| {
            !matches!(
                output.materialization(),
                crate::p5::Materialization::Literal(_)
            )
        }) {
            continue;
        }
        let phases = &code.emission.operations[operation.source.0 as usize];
        if phases.before.is_empty() && phases.snapshots.is_empty() && phases.after.is_empty() {
            code.emission.omitted_literals[statement.id.0 as usize] = true;
        }
    }
    Ok(())
}
