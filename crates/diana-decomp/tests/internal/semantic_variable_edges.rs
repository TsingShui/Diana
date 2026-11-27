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

#[test]
fn deferred_operand_facts_preserve_the_first_operations_types_and_values() {
    use crate::ir::{InsnArg, InsnNode};
    use std::sync::Arc;

    let mut register = RegisterArg::new(2, ArgType::LONG);
    register.code_var = Some(3);
    let mut instruction = InsnNode::new(InsnType::Arith, 2);
    instruction.id = InstructionId::new(12);
    instruction.args = vec![InsnArg::Reg(register), InsnArg::Lit(LiteralArg::int(7))];
    let mut repeated = instruction.clone();
    repeated.args[1] = InsnArg::Lit(LiteralArg::int(19));
    let first = SemanticStatement::instruction(instruction).unwrap();
    let second = SemanticStatement::instruction(repeated).unwrap();
    let root = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements: vec![first, second],
    });
    let types = Arc::new(SourceTypeEnvironment::default());
    let eager = VariableSemanticGraph::analyze(&root, &types);
    let graph = VariableSemanticGraph::analyze_shared(&root, &types);
    drop(root);
    drop(types);
    let id = InstructionId::new(12);
    let row = graph.operation(id).unwrap();
    let operands = row.operands();
    assert_eq!(operands[0].ty(), Some(&ArgType::LONG));
    assert_eq!(operands[1].literal(), Some(7));
    assert!(std::ptr::eq(operands, row.operands()));
    assert_eq!(format!("{graph:?}"), format!("{eager:?}"));
}
