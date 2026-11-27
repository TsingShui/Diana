use super::*;
use crate::ir::{BlockId, LiteralArg, SemanticBlock};
#[test]
fn duplicate_edges_keep_each_variables_first_discovery_order_in_large_groups() {
    let mut order = (1..=17).rev().collect::<Vec<_>>();
    order.extend([13, 9, 7, 2, usize::MAX, 17, usize::MAX]);
    let statements = order
        .iter()
        .flat_map(|&site| {
            [3, 8].map(|variable| {
                let mut result = RegisterArg::new(variable, ArgType::INT);
                result.code_var = Some(variable);
                SemanticStatement::definition(
                    InstructionId::new(site),
                    result,
                    SemanticExpression::Literal(LiteralArg::int(0)),
                )
            })
        })
        .collect();
    let graph = VariableSemanticGraph::analyze(
        &SemanticNode::BasicBlock(SemanticBlock {
            id: BlockId::new(0),
            statements,
        }),
        &SourceTypeEnvironment::default(),
    );
    let expected = (1..=17).rev().chain([usize::MAX]).collect::<Vec<_>>();
    for variable in [3, 8] {
        assert_eq!(graph.variable(variable).unwrap().definitions(), order.len());
        assert_eq!(
            graph
                .edges_for(variable)
                .map(|edge| edge.operation().unwrap().raw())
                .collect::<Vec<_>>(),
            expected
        );
    }
    assert_eq!(graph.edges_for(99).count(), 0);
}
