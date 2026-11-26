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
