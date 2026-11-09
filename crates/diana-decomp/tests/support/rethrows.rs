//! Original pending rethrows remain explicit or have one implicit finally anchor.

use diana_decomp::p6::RegionFacts;
use diana_decomp::p9::{Origin, Rendered};

pub(super) fn assert_pending_rethrow(facts: &RegionFacts, rendered: &Rendered) {
    if let Some(cleanup) = facts.tries().iter().find_map(|p| p.finally()) {
        let block = facts.nodes()[cleanup.rethrow().0 as usize].block();
        let spans: Vec<_> = rendered
            .spans()
            .iter()
            .filter(|s| {
                matches!(s.origin,
            Origin::Terminal { block: original, .. } if original == block)
            })
            .collect();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].start, spans[0].end);
        assert!(rendered.text().contains("finally"));
    } else {
        assert!(rendered.text().contains("throw "));
    }
}
