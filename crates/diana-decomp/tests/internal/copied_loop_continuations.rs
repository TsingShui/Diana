use super::*;

#[test]
fn a_copied_entry_keeps_its_outer_loop_continuation() {
    let region = RegionId::new(0);
    let [source, primary, secondary, outer, unrelated, local] =
        [0, 1, 2, 3, 4, 5].map(BlockId::new);
    let outer_loop = SemanticLabel::loop_(region, outer);
    let own_label = SemanticLabel::block(region, local);
    let mut heavy = SemanticNode::Empty;
    for id in 20..40 {
        heavy = SemanticNode::Label {
            label: SemanticLabel::block(region, BlockId::new(id)),
            body: Box::new(heavy),
        };
    }
    let jumping = SemanticNode::Leave(crate::ir::SemanticLeave {
        site: None,
        condition: None,
        kind: SemanticLeaveKind::ContinueLabel(outer_loop),
        edge: None,
        origin: None,
        source: region,
        destination: region,
        target: region,
        cleanup: Vec::new(),
    });
    let node = |body, transfer| FlowNode {
        facts: Default::default(),
        body,
        transfer,
    };
    let mut graph = SemanticFlowGraph {
        region,
        entry: source,
        nodes: BTreeMap::from([
            (
                source,
                node(
                    SemanticNode::Empty,
                    FlowTransfer::Branch {
                        condition: SemanticPredicate::True,
                        when_true: primary,
                        when_false: secondary,
                    },
                ),
            ),
            (primary, node(heavy, FlowTransfer::Jump(secondary))),
            (
                secondary,
                node(
                    SemanticNode::Label {
                        label: own_label,
                        body: Box::new(jumping),
                    },
                    FlowTransfer::Jump(primary),
                ),
            ),
        ]),
        next_id: 50,
        continues: BTreeMap::from([
            (
                outer,
                LoopContinuation::scoped(outer_loop, BTreeSet::from([source, primary, secondary])),
            ),
            (
                unrelated,
                LoopContinuation::scoped(
                    SemanticLabel::loop_(region, unrelated),
                    BTreeSet::from([primary]),
                ),
            ),
        ]),
        exits: BTreeSet::new(),
    };
    let component = BTreeSet::from([primary, secondary]);
    let predecessors = graph.predecessors();
    graph
        .split_entries(&component, &component, &predecessors)
        .unwrap();
    let copy = graph
        .targets(source)
        .into_iter()
        .find(|block| block.raw() >= 50)
        .expect("copied ingress");
    assert!(graph.continues[&outer].accepts(copy));
    assert!(!graph.continues[&unrelated].accepts(copy));
    let SemanticNode::Label { label, .. } = &graph.nodes[&copy].body else {
        panic!("the copied body must keep its own lexical label");
    };
    assert_ne!(*label, own_label);
    assert!(FreeLabelReferences::collect(&graph.nodes[&copy].body).contains(&outer_loop));
    graph
        .verify_lexical_domains()
        .expect("the copied label remains inside its original owner");
}
