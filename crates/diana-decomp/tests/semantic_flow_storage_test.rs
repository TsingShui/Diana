use diana_decomp::ir::analysis::{SemanticFlowGraph, SemanticFlowPoint, SemanticValueDefinition};
use diana_decomp::ir::{
    BlockId, InsnNode, InsnType, SemanticBlock, SemanticNode, SemanticSiteId, SemanticStatement,
};

#[test]
fn interleaved_definitions_preserve_ordinals_across_storage_words() {
    let sites = [SemanticSiteId(0), SemanticSiteId(1)];
    let statements = sites
        .map(|site| {
            let mut statement = SemanticStatement::instruction(InsnNode::new(InsnType::Nop, 0))
                .expect("numbered instruction");
            statement.site = Some(site);
            statement
        })
        .into();
    let graph = SemanticFlowGraph::analyze(&SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements,
    }));
    let [first, second] = sites.map(SemanticFlowPoint::after);
    let expected: Vec<_> = (0..70).rev().collect();
    let mut definitions = Vec::new();
    for &ordinal in &expected {
        definitions.push(SemanticValueDefinition::new(first, 2, ordinal));
        definitions.push(SemanticValueDefinition::new(first, 0, ordinal));
    }
    definitions.push(SemanticValueDefinition::new(second, 2, 70));

    let reaching = graph.reaching_values(&definitions, 3, &[first, second].into());
    assert_eq!(reaching.at(0, second), Some(expected.clone()));
    assert_eq!(reaching.at(1, second), Some(vec![]));
    assert_eq!(reaching.at(2, first), Some(expected));
    assert_eq!(reaching.at(2, second), Some(vec![70]));
    assert_eq!(reaching.unchanged(0, first, second), Some(true));
    assert_eq!(reaching.unchanged(1, first, second), Some(true));
    assert_eq!(reaching.unchanged(2, first, second), Some(false));

    let order = graph.reachability();
    assert!(order.reaches(first, second));
    assert!(!order.reaches(second, first));
    assert!(!order.reaches(first, SemanticFlowPoint::Synthetic(99)));
}
