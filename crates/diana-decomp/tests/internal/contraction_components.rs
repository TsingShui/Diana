use super::{BlockId, ContractionGraph, CyclePolicy, EdgeKind, NormalCopySite, CFG};
use crate::ir::Block;
use std::collections::BTreeSet;

#[test]
fn adding_an_independent_domain_updates_previously_queried_components() {
    let [a, b, c, d, e] = [1, 2, 3, 4, 5].map(BlockId);
    let first = ContractionGraph::new([(a, b), (d, b)]).solve(CyclePolicy::Canonical);
    assert_eq!(first.component(b), BTreeSet::from([a, b, d]));
    assert!(first.component(c).is_empty());

    let second = ContractionGraph::new([(a, c), (e, c), (e, b)]).solve(CyclePolicy::Preserve);
    let joined = first.with_domain(second);
    assert_eq!(joined.terminal(a), None);
    assert_eq!(joined.terminal(b), Some(b));
    assert_eq!(joined.terminal(d), Some(b));
    assert_eq!(joined.component(b), BTreeSet::from([a, b, d, e]));
    assert_eq!(joined.component(c), BTreeSet::from([a, c, e]));
}

#[test]
fn copy_sites_distinguish_duplicate_endpoints_from_duplicate_physical_edges() {
    let [source, entry, sink, outside] = [0, 1, 2, 3].map(BlockId);
    let contractions = ContractionGraph::new([(entry, sink)]).solve(CyclePolicy::Canonical);
    let mut cfg = CFG::new("duplicate_edge_kinds");
    for id in 0..=3 {
        cfg.add_block(Block::new(id));
    }
    cfg.add_edge(source, entry, EdgeKind::True);
    cfg.add_edge(source, entry, EdgeKind::False);
    cfg.add_edge(entry, sink, EdgeKind::Normal);
    assert_eq!(
        contractions.normal_copy_site(&cfg, source, entry, sink),
        Some(NormalCopySite::Block(source))
    );
    cfg.add_edge(source, outside, EdgeKind::SwitchDefault);
    assert_eq!(
        contractions.normal_copy_site(&cfg, source, entry, sink),
        None
    );
}

#[test]
fn a_phi_inside_the_contraction_keeps_its_concrete_ingress() {
    let [source, phi, sink, bypass] = [0, 1, 2, 3].map(BlockId);
    let contractions = ContractionGraph::new([(phi, sink)]).solve(CyclePolicy::Preserve);
    let mut cfg = CFG::new("contracted_phi_ingress");
    for id in 0..=3 {
        cfg.add_block(Block::new(id));
    }
    cfg.add_edge(source, phi, EdgeKind::True);
    cfg.add_edge(source, bypass, EdgeKind::False);
    cfg.add_edge(phi, sink, EdgeKind::Normal);
    assert_eq!(
        contractions.normal_copy_site(&cfg, source, phi, phi),
        Some(NormalCopySite::Edge(crate::ir::RegionEdge {
            source,
            target: phi,
            kind: EdgeKind::True,
        }))
    );
    cfg.add_edge(phi, bypass, EdgeKind::SwitchDefault);
    assert_eq!(contractions.normal_copy_site(&cfg, source, phi, phi), None);
}
