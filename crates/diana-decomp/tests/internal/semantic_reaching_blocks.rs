use super::super::{
    SemanticFlowBuilder, SemanticFlowEdgeKind, SemanticFlowPoint, SemanticValueDefinition,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

fn point(i: usize) -> SemanticFlowPoint {
    SemanticFlowPoint::Synthetic((i * 11) as u64)
}

#[test]
fn block_transfers_match_each_reachable_last_write() {
    // Two entries feed three diamonds, a reentering loop, a self-loop and a
    // shared exit. The last two nodes are a separate unreachable cycle.
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 4),
        (3, 7),
        (4, 5),
        (5, 6),
        (6, 10),
        (7, 8),
        (8, 9),
        (9, 10),
        (10, 11),
        (11, 12),
        (12, 13),
        (12, 14),
        (13, 15),
        (14, 15),
        (15, 16),
        (16, 2),
        (16, 17),
        (17, 18),
        (18, 19),
        (19, 20),
        (20, 21),
        (20, 23),
        (21, 22),
        (22, 23),
        (23, 23),
        (23, 24),
        (25, 26),
        (26, 25),
    ];
    let mut builder = SemanticFlowBuilder::new();
    for (source, target) in edges {
        builder.add_edge(point(source), point(target), SemanticFlowEdgeKind::Normal);
    }
    let graph = builder.finish([point(0), point(18)].into());
    let definitions = (0..91)
        .map(|i| SemanticValueDefinition::new(point((i * 19) % 27), i % 7, 1000 - i))
        .chain([
            SemanticValueDefinition::new(point(8), 2, 4100),
            SemanticValueDefinition::new(point(8), 2, 4200),
            SemanticValueDefinition::new(point(8), 2, 4300),
        ])
        .collect::<Vec<_>>();
    for targets in [
        (0..28).map(point).collect::<BTreeSet<_>>(),
        [
            point(7),
            point(9),
            point(20),
            point(22),
            point(25),
            point(27),
        ]
        .into(),
        [point(3)].into(),
    ] {
        let solved = graph.reaching_values(&definitions, 8, &targets);
        for value in 0..8 {
            // Enumerate concrete paths to a fixed point, carrying exactly one
            // last write (or the absence of a write) for this one value.
            let mut pending = VecDeque::from([(point(0), None), (point(18), None)]);
            let mut visited = BTreeSet::new();
            let mut expected = BTreeMap::<_, BTreeSet<usize>>::new();
            while let Some((site, incoming)) = pending.pop_front() {
                if !visited.insert((site, incoming)) {
                    continue;
                }
                let mut states = definitions
                    .iter()
                    .filter(|def| def.point == site && def.value == value)
                    .map(|def| Some(def.ordinal))
                    .collect::<Vec<_>>();
                if states.is_empty() {
                    states.push(incoming);
                }
                expected
                    .entry(site)
                    .or_default()
                    .extend(states.iter().flatten().copied());
                for &successor in graph.successors(site) {
                    pending.extend(states.iter().map(|&state| (successor, state)));
                }
            }
            for site in (0..28).map(point) {
                let actual = solved
                    .at(value, site)
                    .map(|indices| indices.into_iter().collect::<BTreeSet<_>>());
                let expected = (targets.contains(&site) && site != point(27))
                    .then(|| expected.get(&site).cloned().unwrap_or_default());
                assert_eq!(actual, expected, "value {value} at {site:?}");
            }
        }
    }
}

#[test]
fn cached_reaching_requires_the_same_points_and_definition_ordinals() {
    let mut builder = SemanticFlowBuilder::new();
    for i in 0..3 {
        builder.add_edge(point(i), point(i + 1), SemanticFlowEdgeKind::Normal);
    }
    let graph = builder.finish([point(0)].into());
    let initial = vec![SemanticValueDefinition::new(point(1), 0, 5)];
    let targets = BTreeSet::from([point(3)]);
    let previous = graph.cached_reaching_values(initial.clone(), 1, targets.clone());
    let repeated = graph.cached_reaching_values(initial.clone(), 1, targets.clone());
    assert!(std::sync::Arc::ptr_eq(&previous, &repeated));
    assert_eq!(previous.at(0, point(3)), Some(vec![5]));
    let expanded = graph.cached_reaching_values(initial, 1, [point(0), point(3)].into());
    assert_eq!(expanded.at(0, point(0)), Some(vec![]));
    let amended = graph.cached_reaching_values(
        vec![SemanticValueDefinition::new(point(1), 0, 19)],
        1,
        targets,
    );
    assert_eq!(amended.at(0, point(3)), Some(vec![19]));
    assert_eq!(amended.at(0, point(0)), None);
    assert_eq!(previous.at(0, point(3)), Some(vec![5]));
}

#[test]
fn frozen_edges_sort_deduplicate_and_preserve_each_cleanup_transition() {
    use super::super::SemanticFlowEdgeKind::{Exceptional, FinallyEnter, FinallyResume, Normal};
    let far = SemanticFlowPoint::Synthetic(u64::MAX);
    let isolated = SemanticFlowPoint::Synthetic(u64::MAX - 1);
    let mut builder = SemanticFlowBuilder::new();
    for (source, target, kind) in [
        (far, point(3), Normal),
        (point(0), point(3), FinallyEnter(6)),
        (far, point(2), Exceptional),
        (point(0), point(3), FinallyResume(6)),
        (point(0), point(3), FinallyEnter(6)),
        (far, point(3), Normal),
        (far, point(2), Normal),
        (point(0), point(3), Normal),
    ] {
        builder.add_edge(source, target, kind);
    }
    let graph = builder.finish([point(0), isolated].into());
    assert_eq!(graph.successors(far), &[point(2), point(3)]);
    assert_eq!(graph.successors(point(0)), &[point(3)]);
    assert_eq!(
        graph.predecessors(point(3)).collect::<Vec<_>>(),
        [point(0), point(0), point(0), far]
    );
    assert_eq!(
        graph.edges.predecessors(point(3)),
        &[
            (point(0), Normal),
            (point(0), FinallyEnter(6)),
            (point(0), FinallyResume(6)),
            (far, Normal),
        ]
    );
    assert_eq!(
        graph.points(),
        [point(0), point(2), point(3), far, isolated].into()
    );
    assert!(graph.successors(isolated).is_empty());
    assert!(graph.has_cleanup_continuations());
}
