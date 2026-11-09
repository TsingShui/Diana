//! Adjacent readable carriers retain their original checks and ordering.

use diana_decomp::p2::{OpId, OpKind};
use diana_decomp::p7::{CheckCarrier, MethodCode};
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn operation_range(rendered: &Rendered, operation: OpId) -> (usize, usize) {
    let spans: Vec<_> = rendered
        .spans()
        .iter()
        .filter(|s| {
            matches!(s.origin,
        Origin::Operation { operation: original, .. } if original == operation)
        })
        .collect();
    assert_eq!(spans.len(), 1, "one original operation anchor");
    (spans[0].start.0, spans[0].end.0)
}
fn check(code: &MethodCode) -> OpId {
    code.operations()
        .iter()
        .find(|op| matches!(op.original, OpKind::Cast { .. }))
        .expect("original check-cast")
        .source
}

#[test]
fn an_initializer_carrier_names_the_check_not_the_preceding_call() {
    inspect(
        "atoms/refined-carriers",
        "Lfixture/RefinedCarriers;",
        "declared",
        "(Ljava/lang/Object;)I",
        |_, code, rendered| {
            let original = check(code);
            let [carried] = code.emission().inline_checks() else {
                panic!("one initializer check");
            };
            assert!(matches!(carried.carrier, CheckCarrier::Definition(_)));
            assert_eq!(carried.operation, original);
            assert_eq!(
                carried.pc,
                code.operations()
                    .iter()
                    .find(|op| op.source == original)
                    .unwrap()
                    .pc
            );
            let (start, end) = operation_range(rendered, original);
            assert!(start < end);
        },
    );
}

#[test]
fn a_later_matching_read_does_not_delay_a_check_past_a_call_or_early_exit() {
    inspect(
        "atoms/refined-carriers",
        "Lfixture/RefinedCarriers;",
        "checkedEarly",
        "(Ljava/lang/Object;I)I",
        |_, code, rendered| {
            let original = check(code);
            assert!(code.emission().inline_checks().is_empty());
            let tick = code
                .operations()
                .iter()
                .find(|op| matches!(op.original, OpKind::Invoke { .. }))
                .unwrap()
                .source;
            let (start, end) = operation_range(rendered, original);
            assert!(start < end);
            assert!(end <= operation_range(rendered, tick).0);
            assert!(rendered.text()[start..end].contains("check-cast"));
        },
    );
}

#[test]
fn exact_payload_fill_remains_an_array_literal_across_adjacent_portions() {
    inspect(
        "atoms/refined-carriers",
        "Lfixture/RefinedCarriers;",
        "bytes",
        "()[B",
        |_, code, rendered| {
            let allocation = code
                .operations()
                .iter()
                .find(|op| matches!(op.original, OpKind::NewArray { .. }))
                .unwrap();
            let literal = code
                .emission()
                .array_literal(allocation.statement.unwrap())
                .expect("exact whole payload");
            let (start, end) = operation_range(rendered, literal.operation);
            assert!(start < end);
            assert!(rendered.text().contains("new byte[]{1, 2, 3}"));
            assert!(!rendered.text().contains("fill_array"));
        },
    );
}

#[test]
fn a_different_reference_check_stays_between_allocation_and_payload_fill() {
    inspect(
        "atoms/refined-carriers",
        "Lfixture/RefinedCarriers;",
        "guarded",
        "(Ljava/lang/Object;)[B",
        |_, code, rendered| {
            let allocation = code
                .operations()
                .iter()
                .find(|op| matches!(op.original, OpKind::NewArray { .. }))
                .unwrap();
            assert!(
                code.emission()
                    .array_literal(allocation.statement.unwrap())
                    .is_none()
            );
            let original = check(code);
            let fill = code
                .operations()
                .iter()
                .find(|op| matches!(op.original, OpKind::FillArrayData { .. }))
                .unwrap()
                .source;
            assert!(
                operation_range(rendered, allocation.source).1
                    <= operation_range(rendered, original).0
            );
            assert!(operation_range(rendered, original).1 <= operation_range(rendered, fill).0);
            assert!(rendered.text().contains("fill_array"));
        },
    );
}
