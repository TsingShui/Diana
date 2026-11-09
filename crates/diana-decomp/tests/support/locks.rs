//! Original acquisition/release anchors shared by monitor fixture checks.

use diana_decomp::p2::OpKind;
use diana_decomp::p7::MethodCode;
use diana_decomp::p9::{Origin, Rendered};

pub(super) fn lock_anchors(code: &MethodCode, rendered: &Rendered) {
    for op in code.operations() {
        if matches!(op.original, OpKind::Monitor { .. }) {
            let spans: Vec<_> = rendered
                .spans()
                .iter()
                .filter(|s| {
                    matches!(s.origin,
                Origin::Operation { operation, .. } if operation == op.source)
                })
                .collect();
            assert_eq!(spans.len(), 1, "one original acquisition/release anchor");
            assert!(spans[0].start.0 < spans[0].end.0);
        }
    }
}
