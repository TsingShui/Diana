use super::*;
use crate::ir::analysis::SsaValueGraph;
use crate::ir::{ExceptionAnalysis, InsnNode, RegionGraphBuilder, SemanticBlock};

#[test]
fn a_rejected_skeleton_retains_bodies_and_predicates_for_the_contracted_graph() {
    let id = BlockId::new;
    let mut physical = CFG::new("physical_return");
    physical.entry = id(0);
    let mut block = Block::new(0_u32);
    block.push(InsnNode::return_void());
    physical.add_block(block);
    let values = SsaValueGraph::build(&physical).unwrap();
    let regions = RegionGraphBuilder::new(&physical, &ExceptionAnalysis::default(), &values)
        .build()
        .unwrap();
    let region = regions.tree().root();
    let semantic = SemanticFactory::new(&physical, &regions, region);

    // A long shared tail exceeds the acyclic reachability matrix budget.
    let mut nodes = BTreeMap::new();
    for index in 0..=8192 {
        let transfer = match index {
            0 => FlowTransfer::Branch {
                condition: SemanticPredicate::False,
                when_true: id(1),
                when_false: id(2),
            },
            1 | 2 => FlowTransfer::Jump(id(3)),
            8192 => FlowTransfer::Stop,
            _ => FlowTransfer::Jump(id(index + 1)),
        };
        nodes.insert(
            id(index),
            FlowNode {
                body: SemanticNode::BasicBlock(SemanticBlock {
                    id: id(index),
                    statements: Vec::new(),
                }),
                transfer,
                facts: Default::default(),
            },
        );
    }
    let mut graph = SemanticFlowGraph {
        region,
        entry: id(0),
        next_id: 8193,
        nodes,
        continues: BTreeMap::new(),
        exits: BTreeSet::new(),
    };
    let entry = &graph.nodes[&id(0)];
    assert!(entry.completion(region).normal);
    assert!(graph.structure_acyclic(&semantic).unwrap().is_none());
    let entry = &graph.nodes[&id(0)];
    assert!(entry.facts.completion.get().is_some());
    assert!(matches!(&entry.body, SemanticNode::BasicBlock(block) if block.id == id(0)));

    // The contracted graph uses the retained false predicate, then its shared
    // continuation. Its new exit is an empty sink with no physical DEX block.
    graph.nodes.retain(|block, _| block.raw() <= 3);
    graph.nodes.get_mut(&id(3)).unwrap().transfer = FlowTransfer::Jump(id(9000));
    graph.exits.insert(id(9000));
    let Some(SemanticNode::Sequence(body)) = graph.structure_acyclic(&semantic).unwrap() else {
        panic!("expected the selected arm and continuation");
    };
    assert_eq!(
        body.iter()
            .map(|node| match node {
                SemanticNode::BasicBlock(block) => block.id.raw(),
                _ => panic!("unexpected contracted node {node:?}"),
            })
            .collect::<Vec<_>>(),
        [0, 2, 3]
    );
}

#[test]
fn pruning_follows_embedded_transfers_and_forgets_previous_reachability() {
    let entry = BlockId::new(3);
    let arm = BlockId::new(11);
    let dead = BlockId::new(25);
    let region = RegionId::new(4);
    let jump = SemanticNode::Leave(crate::ir::SemanticLeave {
        site: None,
        condition: None,
        kind: SemanticLeaveKind::Jump(arm),
        edge: None,
        origin: None,
        source: region,
        destination: region,
        target: region,
        cleanup: Vec::new(),
    });
    let node = |body, transfer| FlowNode {
        body,
        transfer,
        facts: Default::default(),
    };
    let mut graph = SemanticFlowGraph {
        region,
        entry,
        next_id: 26,
        nodes: BTreeMap::from([
            (entry, node(jump, FlowTransfer::Stop)),
            (arm, node(SemanticNode::Empty, FlowTransfer::Jump(entry))),
            (dead, node(SemanticNode::Empty, FlowTransfer::Jump(dead))),
        ]),
        continues: BTreeMap::new(),
        exits: BTreeSet::new(),
    };
    graph.prune();
    assert_eq!(
        graph.nodes.keys().copied().collect::<Vec<_>>(),
        [entry, arm]
    );
    graph.nodes.get_mut(&entry).unwrap().take_body();
    graph.prune();
    assert_eq!(graph.nodes.keys().copied().collect::<Vec<_>>(), [entry]);
}

#[test]
fn fragment_boundaries_preserve_scoped_use_counts_and_normal_completion() {
    let region = RegionId::new(1);
    let foreign = RegionId::new(2);
    let block = BlockId::new(6);
    let target = BlockId::new(10);
    let other = BlockId::new(14);
    let jump = |scope, target| {
        SemanticNode::Leave(crate::ir::SemanticLeave {
            site: None,
            condition: None,
            kind: SemanticLeaveKind::Jump(target),
            edge: None,
            origin: None,
            source: scope,
            destination: scope,
            target: scope,
            cleanup: Vec::new(),
        })
    };
    // Completion stops at the first abrupt statement. Embedded-use counts
    // include every lexical occurrence, but only for the enclosing region.
    let body = SemanticNode::Sequence(vec![
        jump(region, target),
        jump(region, target),
        jump(foreign, other),
    ]);
    let targets = BTreeSet::from([other, target]);
    let mut node = FlowNode::fragment(block, region, body, targets.clone(), [other]).unwrap();
    assert!(
        matches!(&node.transfer, FlowTransfer::Fragment { normal: None, open }
        if open == &BTreeSet::from([target]))
    );
    assert_eq!(node.embedded_uses(region), [(target, 2)]);
    assert!(!node.completion(region).normal);
    let body = node.take_body();
    assert!(node.embedded_uses(region).is_empty());
    assert!(node.completion(region).normal);
    assert!(matches!(
        FlowNode::fragment(block, region, body, targets.clone(), []),
        Err(StructureError::MalformedFlowNode(id)) if id == block
    ));
    assert!(matches!(
        FlowNode::fragment(block, region, SemanticNode::Empty, targets, []),
        Err(StructureError::MalformedFlowNode(id)) if id == block
    ));
}

#[test]
fn continuation_ownership_counts_reachable_sources_once_per_target() {
    let entry = BlockId::new(0);
    let arm = BlockId::new(3);
    let join = BlockId::new(8);
    let dead = BlockId::new(14);
    let outside = BlockId::new(20);
    let region = RegionId::new(2);
    let jump = |target| {
        SemanticNode::Leave(crate::ir::SemanticLeave {
            site: None,
            condition: None,
            kind: SemanticLeaveKind::Jump(target),
            edge: None,
            origin: None,
            source: region,
            destination: region,
            target: region,
            cleanup: Vec::new(),
        })
    };
    let node = |body, transfer| FlowNode {
        body,
        transfer,
        facts: Default::default(),
    };
    let half_open = SemanticNode::If {
        condition: crate::ir::SemanticOperand::new(SemanticPredicate::True),
        then_node: Box::new(jump(join)),
        else_node: None,
    };
    let mut graph = SemanticFlowGraph {
        region,
        entry,
        next_id: 21,
        nodes: BTreeMap::from([
            (
                entry,
                node(
                    half_open,
                    FlowTransfer::Fragment {
                        normal: Some(arm),
                        open: BTreeSet::from([join]),
                    },
                ),
            ),
            (
                arm,
                node(
                    SemanticNode::Empty,
                    FlowTransfer::Branch {
                        condition: SemanticPredicate::True,
                        when_true: join,
                        when_false: join,
                    },
                ),
            ),
            (join, node(jump(outside), FlowTransfer::Stop)),
            (dead, node(SemanticNode::Empty, FlowTransfer::Jump(join))),
        ]),
        continues: BTreeMap::new(),
        exits: BTreeSet::from([outside]),
    };
    let mut predecessors = ContinuationPredecessors::default();
    assert_eq!(predecessors.counts(&graph)[&join], 3);
    predecessors.prune(&mut graph);
    assert!(!graph.nodes.contains_key(&dead));
    let counts = predecessors.counts(&graph);
    assert_eq!(counts[&join], 2);
    assert_eq!(counts[&arm], 1);
    assert_eq!(counts[&outside], 1);
    assert!(!graph.is_internal_target(outside));
}

#[test]
fn branch_candidates_keep_independent_members_after_rejected_shared_entries() {
    // The cuts are checked independently from the search using forward
    // reachability and every incoming edge of the induced arm. Sparse block
    // identifiers deliberately have a different order from execution.
    let ids = [51, 307, 11, 810, 29, 17, 1043].map(BlockId::new);
    for shape in 0..64 {
        let mut edges = [
            vec![1, 2],
            vec![3],
            vec![4],
            vec![5],
            vec![5],
            vec![6],
            vec![],
        ];
        for (bit, (from, to)) in [(1, 2), (1, 4), (2, 3), (2, 5), (3, 4), (4, 6)]
            .into_iter()
            .enumerate()
        {
            if shape & (1 << bit) != 0 {
                edges[from].push(to);
            }
        }
        let region = RegionId::new(1);
        let graph = SemanticFlowGraph {
            region,
            entry: ids[0],
            next_id: 1044,
            exits: BTreeSet::new(),
            continues: BTreeMap::new(),
            nodes: edges
                .iter()
                .enumerate()
                .map(|(from, targets)| {
                    let transfer = match targets.as_slice() {
                        [] => FlowTransfer::Stop,
                        [one] => FlowTransfer::Jump(ids[*one]),
                        many => FlowTransfer::Fragment {
                            normal: None,
                            open: many.iter().map(|&n| ids[n]).collect(),
                        },
                    };
                    (
                        ids[from],
                        FlowNode {
                            body: SemanticNode::Empty,
                            transfer,
                            facts: Default::default(),
                        },
                    )
                })
                .collect(),
        };
        let topology = FlowTopology::new(&graph);
        let order = graph.topological_order(&topology).unwrap();
        let ranks = order
            .iter()
            .copied()
            .enumerate()
            .map(|(index, block)| (block, index))
            .collect();
        let mut search = BranchArmSearch::new(&graph, &topology, &order, &ranks);
        for join in [5, 2, 3, 6, 1, 4, 2, 5] {
            let expected_arm = |entry: usize| {
                let mut members = BTreeSet::new();
                let mut reached_join = false;
                let mut reachable = vec![false; ids.len()];
                reachable[entry] = true;
                for (from, targets) in edges.iter().enumerate() {
                    if reachable[from] {
                        if from == join {
                            reached_join = true;
                        } else {
                            members.insert(ids[from]);
                            for &target in targets {
                                reachable[target] = true;
                            }
                        }
                    }
                }
                let single_entry = edges.iter().enumerate().all(|(from, targets)| {
                    targets.iter().all(|&to| {
                        !members.contains(&ids[to])
                            || members.contains(&ids[from])
                            || (to == entry && from == 0)
                    })
                });
                (reached_join && single_entry).then_some(members)
            };
            let expected = expected_arm(1)
                .zip(expected_arm(2))
                .filter(|(left, right)| {
                    left.is_disjoint(right) && !(left.is_empty() && right.is_empty())
                });
            let actual = search
                .pair(ids[0], ids[1], ids[2], ids[join])
                .map(|(left, right)| {
                    assert!(left.continuation.is_none() && right.continuation.is_none());
                    (left.nodes, right.nodes)
                });
            assert_eq!(actual, expected, "shape {shape}, join {join}");
        }
    }
}

#[test]
fn a_continuation_cut_preserves_shared_suffixes_and_external_frontiers() {
    let id = BlockId::new;
    let entry = id(3);
    let source = id(40);
    let side = id(71);
    let middle = id(112);
    let chain = [id(124), id(155)];
    let frontier = id(900);
    let absent = id(1000);
    let region = RegionId::new(2);
    let jump = FlowTransfer::Jump;
    let stop = FlowTransfer::Stop;
    let branch = |when_true, when_false| FlowTransfer::Branch {
        condition: SemanticPredicate::False,
        when_true,
        when_false,
    };
    for keep_entry in [false, true] {
        for shared in [None, Some(chain[0]), Some(chain[1])] {
            for (frontier, back_edge) in [
                (None, false),
                (Some(frontier), false),
                (Some(frontier), true),
                (Some(absent), false),
            ] {
                for old_frontier in [false, true] {
                    let original = if let Some(exit) = frontier.filter(|_| old_frontier) {
                        branch(chain[0], exit)
                    } else {
                        jump(chain[0])
                    };
                    let rewritten = match (keep_entry, frontier) {
                        (true, Some(exit)) => branch(chain[0], exit),
                        (true, None) => jump(chain[0]),
                        (false, Some(exit)) => jump(exit),
                        (false, None) => stop.clone(),
                    };
                    let mut nodes = BTreeMap::from([
                        (entry, branch(source, side)),
                        (source, original),
                        (side, branch(middle, shared.unwrap_or(middle))),
                        (middle, stop.clone()),
                        (chain[0], jump(chain[1])),
                        (chain[1], frontier.map_or_else(|| stop.clone(), jump)),
                    ]);
                    if let Some(exit) = frontier.filter(|&exit| exit != absent) {
                        nodes.insert(
                            exit,
                            if back_edge {
                                jump(chain[1])
                            } else {
                                stop.clone()
                            },
                        );
                    }
                    let mut graph = SemanticFlowGraph {
                        region,
                        entry,
                        nodes: nodes
                            .into_iter()
                            .map(|(block, transfer)| {
                                (
                                    block,
                                    FlowNode {
                                        body: SemanticNode::Empty,
                                        transfer,
                                        facts: Default::default(),
                                    },
                                )
                            })
                            .collect(),
                        exits: BTreeSet::from([absent]),
                        continues: BTreeMap::new(),
                        next_id: 2000,
                    };
                    graph.prune();
                    let mut counts = ContinuationPredecessors::default();
                    counts.counts(&graph);
                    let cut = ContinuationCut::prepare(&graph, source, &chain, frontier).unwrap();
                    graph.nodes.get_mut(&source).unwrap().transfer = rewritten;
                    // The ordinary linear absorption moves its exclusive
                    // nodes into the source before updating reachability.
                    if !keep_entry && shared.is_none() && frontier.is_some() && !back_edge {
                        for block in chain {
                            graph.nodes.remove(&block);
                        }
                    }
                    let mut expected = SemanticFlowGraph {
                        region,
                        entry,
                        nodes: graph.nodes.clone(),
                        exits: graph.exits.clone(),
                        continues: BTreeMap::new(),
                        next_id: graph.next_id,
                    };
                    expected.prune();
                    assert!(cut.prune(&mut graph, &mut counts));
                    assert!(graph.nodes.keys().eq(expected.nodes.keys()));
                    assert_eq!(
                        counts.counts(&graph),
                        ContinuationPredecessors::default().counts(&expected)
                    );
                }
            }
        }
    }
}
