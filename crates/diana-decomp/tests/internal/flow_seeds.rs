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
