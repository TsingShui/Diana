use super::super::{
    SemanticFlowBuilder, SemanticFlowEdgeKind, SemanticFlowPoint, SemanticValueDefinition,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

fn point(i: usize) -> SemanticFlowPoint {
    SemanticFlowPoint::Synthetic((i * 11) as u64)
}

#[test]
fn linear_reaching_distinguishes_write_sites_even_with_repeated_ordinals() {
    let chain = [7, 2, 9, 1, 6, 4].map(point);
    let mut builder = SemanticFlowBuilder::new();
    for edge in chain.windows(2) {
        builder.add_edge(edge[0], edge[1], SemanticFlowEdgeKind::Normal);
    }
    let graph = builder.finish([chain[0]].into());
    let defs = [
        (chain[3], 0, 41),
        (chain[1], 0, 41),
        (chain[5], 1, 99),
        (chain[3], 0, 8),
        (point(99), 0, 123),
        (chain[3], 0, 41),
        (chain[2], 1, 99),
    ]
    .map(|(site, value, ordinal)| SemanticValueDefinition::new(site, value, ordinal));
    let facts = graph.reaching_values(
        &defs,
        3,
        &[chain[0], chain[1], chain[2], chain[3], chain[5], point(99)].into(),
    );
    assert_eq!(facts.at(0, chain[0]), Some(vec![]));
    assert_eq!(facts.at(0, chain[2]), Some(vec![41]));
    assert_eq!(facts.at(0, chain[5]), Some(vec![41, 8, 41]));
    assert_eq!(facts.at(1, chain[2]), Some(vec![99]));
    assert_eq!(facts.at(1, chain[5]), Some(vec![99]));
    assert_eq!(facts.unchanged(1, chain[2], chain[5]), Some(false));
    assert_eq!(facts.unchanged(1, chain[5], chain[2]), Some(false));
    assert_eq!(facts.unchanged(0, chain[1], chain[2]), Some(true));
    assert_eq!(facts.unchanged(2, chain[0], chain[5]), Some(true));
    assert_eq!(facts.at(3, chain[1]), Some(vec![]));
    assert_eq!(facts.unchanged(3, chain[0], chain[1]), None);
    assert_eq!(facts.at(0, chain[4]), None);
    assert_eq!(facts.at(0, point(99)), None);
    assert_eq!(facts.unchanged(1, chain[4], chain[5]), None);
}

#[test]
fn long_linear_flow_retains_one_record_per_write_for_all_target_snapshots() {
    let count = 512;
    let mut builder = SemanticFlowBuilder::new();
    for i in 1..count {
        builder.add_edge(point(i - 1), point(i), SemanticFlowEdgeKind::Normal);
    }
    let graph = builder.finish([point(0)].into());
    let defs = (0..count)
        .rev()
        .map(|i| SemanticValueDefinition::new(point(i), i % 11, 1000 - i))
        .collect::<Vec<_>>();
    let facts = graph.reaching_values(&defs, 11, &(0..count).map(point).collect());
    let super::super::ReachingStorage::Linear(storage) = &facts.0 else {
        panic!("single chain");
    };
    assert_eq!(storage.writes.len(), count);
    assert_eq!(storage.target_ranks.len(), count);
    for i in 0..count {
        for value in 0..11 {
            let expected = (0..=i)
                .rev()
                .find(|last| last % 11 == value)
                .map(|last| 1000 - last);
            assert_eq!(
                facts.at(value, point(i)),
                Some(expected.into_iter().collect())
            );
        }
    }
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
        let kind = match (source, target) {
            (3, 7) | (12, 14) | (20, 23) => SemanticFlowEdgeKind::Exceptional,
            (16, 2) | (23, 23) => SemanticFlowEdgeKind::LoopBack,
            _ => SemanticFlowEdgeKind::Normal,
        };
        builder.add_edge(point(source), point(target), kind);
    }
    let graph = builder.finish([point(0), point(18)].into());
    let definitions = (0..91)
        .map(|i| SemanticValueDefinition::new(point((i * 19) % 27), i % 7, 1000 - i))
        .chain([
            SemanticValueDefinition::new(point(8), 2, 4100),
            SemanticValueDefinition::new(point(8), 2, 4200),
            SemanticValueDefinition::new(point(8), 2, 4100),
            SemanticValueDefinition::new(point(1), 2, 4100),
            SemanticValueDefinition::new(point(55), 2, 8100),
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
            // last write identity (or its absence) for this one value. Its
            // caller ordinal can repeat both at one point and across sites.
            let mut pending = VecDeque::from([(point(0), None), (point(18), None)]);
            let mut visited = BTreeSet::new();
            let mut expected = BTreeMap::<_, BTreeSet<usize>>::new();
            while let Some((site, incoming)) = pending.pop_front() {
                if !visited.insert((site, incoming)) {
                    continue;
                }
                let mut states = definitions
                    .iter()
                    .enumerate()
                    .filter(|(_, def)| def.point == site && def.value == value)
                    .map(|(identity, _)| Some(identity))
                    .collect::<Vec<_>>();
                if states.is_empty() {
                    states.push(incoming);
                }
                expected
                    .entry(site)
                    .or_default()
                    .extend(states.iter().flatten().copied());
                for &(_, successor) in edges.iter().filter(|&&(from, _)| point(from) == site) {
                    pending.extend(states.iter().map(|&state| (point(successor), state)));
                }
            }
            let identities = |site| {
                (targets.contains(&site) && site != point(27))
                    .then(|| expected.get(&site).cloned().unwrap_or_default())
            };
            for site in (0..28).map(point) {
                let here = identities(site);
                assert_eq!(
                    solved.at(value, site),
                    here.as_ref()
                        .map(|ids| ids.iter().map(|&id| definitions[id].ordinal).collect()),
                    "value {value} at {site:?}"
                );
                for other in (0..28).map(point) {
                    assert_eq!(
                        solved.unchanged(value, site, other),
                        here.as_ref()
                            .zip(identities(other).as_ref())
                            .map(|(a, b)| a == b),
                        "value {value} at {site:?} and {other:?}"
                    );
                }
            }
            assert_eq!(
                solved.at(usize::MAX, point(3)),
                targets.contains(&point(3)).then(Vec::new)
            );
            assert_eq!(solved.unchanged(usize::MAX, point(3), point(3)), None);
        }
    }
}

#[test]
fn two_long_arms_and_a_back_edge_store_four_inputs_for_all_occurrences() {
    const ARM: usize = 256;
    let join = ARM * 2 + 1;
    let mut builder = SemanticFlowBuilder::new();
    for (first, kind) in [
        (1, SemanticFlowEdgeKind::Normal),
        (1 + ARM, SemanticFlowEdgeKind::Exceptional),
    ] {
        builder.add_edge(point(0), point(first), kind);
        for i in first..first + ARM - 1 {
            builder.add_edge(point(i), point(i + 1), SemanticFlowEdgeKind::Normal);
        }
        builder.add_edge(
            point(first + ARM - 1),
            point(join),
            SemanticFlowEdgeKind::Normal,
        );
    }
    builder.add_edge(point(join), point(0), SemanticFlowEdgeKind::LoopBack);
    let graph = builder.finish([point(0)].into());
    let defs = (1..join)
        .rev()
        .map(|i| SemanticValueDefinition::new(point(i), 0, i % ARM))
        .collect::<Vec<_>>();
    let facts = graph.reaching_values(&defs, 1, &(0..=join).map(point).collect());
    let super::super::ReachingStorage::Blocks(storage) = &facts.0 else {
        panic!("two arms with a common join");
    };
    assert_eq!(storage.inputs.len(), 4);
    assert_eq!(storage.writes.len(), 2 * ARM);
    assert_eq!(storage.target_ranks.len(), 2 * ARM + 2);
    // Both last definitions carry ordinal zero. They may reach the join
    // and the next iteration's entry, and remain distinct at their writes.
    assert_eq!(facts.at(0, point(0)), Some(vec![0, 0]));
    assert_eq!(facts.at(0, point(join)), Some(vec![0, 0]));
    assert_eq!(facts.unchanged(0, point(join), point(0)), Some(true));
    for i in 1..=ARM {
        assert_eq!(facts.at(0, point(i)), Some(vec![i % ARM]));
        assert_eq!(facts.at(0, point(ARM + i)), Some(vec![i % ARM]));
        assert_eq!(facts.unchanged(0, point(i), point(ARM + i)), Some(false));
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
        graph.index.incoming_points(point(3)).collect::<Vec<_>>(),
        [
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

#[test]
fn chain_reachability_matches_independent_paths_inside_and_between_loops() {
    use super::super::{ReachabilityStore, SemanticReachability};

    // The two long arms cannot reach each other. Both meet a cycle whose
    // outgoing tail contains another diamond. The last arm is disconnected.
    let mut edges = (0..48)
        .filter(|n| !matches!(n, 12 | 24 | 32 | 41))
        .map(|n| (n, n + 1))
        .collect::<Vec<_>>();
    edges.extend([
        (0, 13),
        (12, 25),
        (24, 25),
        (31, 27),
        (32, 33),
        (32, 37),
        (36, 39),
        (41, 47),
    ]);
    let points = (0..49).map(point).collect::<Vec<_>>();
    let mut builder = SemanticFlowBuilder::new();
    for &(from, to) in &edges {
        builder.add_edge(points[from], points[to], SemanticFlowEdgeKind::Normal);
    }
    let graph = builder.finish([points[0]].into());
    let mut expected = vec![vec![false; points.len()]; points.len()];
    for (from, reachable) in expected.iter_mut().enumerate() {
        let mut pending = vec![from];
        while let Some(node) = pending.pop() {
            if std::mem::replace(&mut reachable[node], true) {
                continue;
            }
            pending.extend(edges.iter().filter_map(|&(u, v)| (u == node).then_some(v)));
        }
    }
    // Every maximal path reaches the target exactly on the least fixed
    // point obtained by admitting nodes whose nonempty exits all qualify.
    for (target, &target_point) in points.iter().enumerate() {
        let mut mandatory = vec![false; points.len()];
        mandatory[target] = true;
        loop {
            let mut changed = false;
            for node in 0..points.len() {
                let mut exits = edges.iter().filter(|&&(source, _)| source == node);
                let Some(&(_, first)) = exits.next() else {
                    continue;
                };
                if !mandatory[node] && mandatory[first] && exits.all(|&(_, to)| mandatory[to]) {
                    mandatory[node] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        for (source, &source_point) in points.iter().enumerate() {
            assert_eq!(
                graph.must_reach(source_point, target_point),
                mandatory[source],
                "mandatory {source} to {target}"
            );
        }
    }
    assert!(
        !graph.index.has_projected_successors(),
        "mandatory paths use the frozen edge rows"
    );
    for limit in [0, usize::MAX] {
        let index = SemanticReachability::analyze_with_dense_limit(&graph, limit);
        let chains = match &index.store {
            ReachabilityStore::Sparse(cache) => cache.successors.len(),
            ReachabilityStore::Dense { bits, row_words } => bits.len() / row_words,
            ReachabilityStore::Linear => panic!("branched graph"),
        };
        assert!(chains < index.chain_of.len() / 2);
        for (from, reachable) in expected.iter().enumerate() {
            for (to, &can_reach) in reachable.iter().enumerate() {
                assert_eq!(index.reaches(points[from], points[to]), can_reach);
                for middle in [0, 4, 13, 19, 26, 29, 32, 39, 48] {
                    assert_eq!(
                        index.reaches_any_between(points[from], points[to], [&points[middle]]),
                        reachable[middle] && expected[middle][to],
                        "from {from} via {middle} to {to}",
                    );
                }
            }
        }
    }
}
