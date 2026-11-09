//! Original body updates stay visible; loop syntax owns only the original test.

use diana_decomp::p2::{OpId, OpKind};
use diana_decomp::p6::{LoopForm, RegionFacts};
use diana_decomp::p7::MethodCode;
use diana_decomp::p9::{Origin, Rendered};

pub fn updates_once(facts: &RegionFacts, code: &MethodCode, rendered: &Rendered) -> Vec<OpId> {
    assert!(!rendered.text().contains("for ("));
    let mut updates = Vec::new();
    for plan in facts.loops() {
        let target = match plan.form() {
            LoopForm::DoWhile(condition) => condition.node(),
            _ => plan.header(),
        };
        assert_eq!(plan.continue_target(), target);
        let mut found = false;
        for &latch in plan.latches() {
            let source = facts.links()[latch.0 as usize].from();
            let block = facts.nodes()[source.0 as usize].block();
            let Some(operation) = code
                .operations()
                .iter()
                .rev()
                .find(|op| facts.nodes()[op.node.0 as usize].block() == block)
            else {
                continue;
            };
            if !matches!(
                operation.original,
                OpKind::Binary { .. } | OpKind::Unary { .. }
            ) {
                continue;
            }
            found = true;
            if updates.contains(&operation.source) {
                continue;
            }
            let spans: Vec<_> = rendered
                .spans()
                .iter()
                .filter(|span| matches!(span.origin, Origin::Operation { operation: id, .. } if id == operation.source))
                .collect();
            assert_eq!(spans.len(), 1, "one original body-update anchor");
            assert!(spans[0].start.0 < spans[0].end.0, "update is printed");
            updates.push(operation.source);
        }
        assert!(found, "this fixture has an original arithmetic tail");
    }
    updates
}
