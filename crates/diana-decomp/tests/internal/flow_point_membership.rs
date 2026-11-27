use super::*;

#[test]
fn before_sites_include_all_edge_endpoints_and_isolated_entries() {
    let id = SemanticSiteId;
    let before = |n| SemanticFlowPoint::before(id(n));
    let after = |n| SemanticFlowPoint::after(id(n));
    let mut builder = SemanticFlowBuilder::new();
    builder.add_edge(before(4), after(15), SemanticFlowEdgeKind::Exceptional);
    builder.add_edge(after(15), before(9), SemanticFlowEdgeKind::FinallyResume(2));
    builder.add_edge(after(15), before(9), SemanticFlowEdgeKind::FinallyResume(2));
    let graph = builder.finish(&[before(u64::MAX), SemanticFlowPoint::Synthetic(7)]);
    for indexed in [false, true] {
        if indexed {
            assert_eq!(graph.successors(after(15)), &[before(9)]);
        }
        assert!(graph.before_sites_known(&[id(4), id(4), id(9), id(17), id(u64::MAX)]));
        assert!(!graph.before_sites_known(&[id(4), id(9)]));
        assert!(!graph.before_sites_known(&[id(4), id(15), id(u64::MAX)]));
        assert!(!graph.before_sites_known(&[id(9), id(u64::MAX)]));
    }
}

#[test]
fn dense_known_prefix_needs_no_index_and_sparse_sites_use_the_exact_graph() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let builds = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&builds);
    let before = |i| SemanticFlowPoint::Before(SemanticSiteId(i));
    let graph = SemanticFlowGraph::with_index(true, 3, move || {
        observed.fetch_add(1, Ordering::Relaxed);
        GraphIndex::new(&[(before(0), before(2), SemanticFlowEdgeKind::Normal)], &[])
    });
    let id = SemanticSiteId;
    assert!(graph.before_sites_known(&[id(0), id(1), id(2), id(7)]));
    assert_eq!(builds.load(Ordering::Relaxed), 0);
    assert!(graph.before_sites_known(&[id(0), id(2)]));
    assert!(!graph.before_sites_known(&[id(0), id(1)]));
    assert_eq!(builds.load(Ordering::Relaxed), 1);
}

#[test]
fn dense_prefix_cannot_hide_a_missing_source_target_or_entry() {
    let before = |n| SemanticFlowPoint::Before(SemanticSiteId(n));
    let after = SemanticFlowPoint::After(SemanticSiteId(10));
    let synthetic = SemanticFlowPoint::Synthetic(11);
    for (source, target, entry) in [
        (before(2), after, synthetic),
        (after, before(2), synthetic),
        (after, synthetic, before(2)),
    ] {
        let mut builder = SemanticFlowBuilder::new();
        builder.add_edge(source, target, SemanticFlowEdgeKind::Exceptional);
        let graph = builder.finish(&[entry]);
        let id = SemanticSiteId;
        assert!(!graph.before_sites_known(&[id(0), id(1)]));
        assert!(graph.before_sites_known(&[id(0), id(1), id(2)]));
    }
}
