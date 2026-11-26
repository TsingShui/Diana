use super::*;
use crate::ir::{
    ArgType, BlockId, InsnArg, InsnNode, InsnType, InstructionId, LiteralArg, RegisterArg,
    SemanticBlock, SemanticExpression, SemanticNode, SemanticOperand, SemanticOperation,
    SemanticPredicate, SemanticSiteNumbering, SemanticStatement, SemanticSwitchCase,
};

fn local() -> RegisterArg {
    let mut result = RegisterArg::new(1, ArgType::INT);
    result.code_var = Some(17);
    result
}

fn define(id: usize) -> SemanticStatement {
    SemanticStatement::definition(
        InstructionId::new(id),
        local(),
        SemanticExpression::Literal(LiteralArg::new(id as i64, ArgType::INT)),
    )
}

fn block(id: usize, statements: Vec<SemanticStatement>) -> SemanticNode {
    SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(id as u32),
        statements,
    })
}

fn predicate(id: usize, operands: Vec<SemanticExpression>) -> SemanticPredicate {
    let mut instruction = InsnNode::new(InsnType::If, operands.len());
    instruction.id = InstructionId::new(id);
    SemanticPredicate::Test(SemanticOperation::from_parts(instruction, operands, None))
}

fn read() -> SemanticStatement {
    let mut instruction = InsnNode::new(InsnType::FilledNewArray, 2);
    instruction
        .args
        .extend([InsnArg::Reg(local()), InsnArg::Reg(local())]);
    SemanticStatement::instruction(instruction).unwrap()
}

#[test]
fn repeated_reads_share_a_join_until_the_control_query_or_reaching_definitions_change() {
    let cases = 64;
    let repeats = 96;
    let switch = SemanticNode::Switch {
        region: None,
        selector: SemanticOperand::new(SemanticExpression::Literal(LiteralArg::new(
            0,
            ArgType::INT,
        ))),
        cases: (0..cases)
            .map(|i| SemanticSwitchCase {
                values: vec![i as i32],
                is_default: i + 1 == cases,
                body: block(i, vec![define(i)]),
            })
            .collect(),
    };
    let guard = predicate(1000, vec![SemanticExpression::Register(local()); 2]);
    let mut root = SemanticNode::sequence([
        switch,
        block(101, (0..repeats).map(|_| read()).collect()),
        SemanticNode::If {
            condition: SemanticOperand::new(guard),
            then_node: Box::new(block(102, vec![])),
            else_node: None,
        },
        block(103, vec![define(1001), read(), read()]),
    ]);
    SemanticSiteNumbering::assign(&mut root).unwrap();
    let graph = ValueFlowGraph::build_source(&root, &Default::default()).unwrap();
    let key = SsaVar::new(17, 0);
    let analysis = SemanticReachingAnalysis::new(&graph);
    let rows = ReachingDefinitions::new(&graph, &analysis)
        .analyze(&graph.definitions[&key], &graph.uses[&key])
        .unwrap();
    let expected = (0..cases).collect::<Vec<_>>();
    let before_overwrite = 2 * repeats + 2;
    for (index, row) in rows.uses.iter().enumerate() {
        if index < before_overwrite {
            assert_eq!(rows.candidates(row), expected);
            assert!(row.selected.is_none());
            assert_eq!(row.control, index >= repeats * 2);
        } else {
            assert_eq!(rows.candidates(row), [cases]);
            assert_eq!(row.selected, Some(cases));
        }
    }
    assert_eq!(rows.uses.len(), before_overwrite + 4);
    assert_eq!(rows.candidate_uses[..cases], vec![before_overwrite; cases]);
    assert_eq!(rows.candidate_uses[cases], 4);
    // The ordinary reads, guard reads and post-overwrite reads each own one partition.
    assert_eq!(rows.candidates.len(), 2 * cases + 1);
}

#[test]
fn a_repeated_point_still_applies_each_distinct_control_domain_after_budget_exhaustion() {
    // An ordered BDD for equality over two separated sets of bits grows exponentially.
    let equality = SemanticPredicate::And(
        (0..16)
            .map(|i| {
                let left = predicate(i + 1, vec![]);
                let right = predicate(i + 17, vec![]);
                SemanticPredicate::Or(vec![
                    SemanticPredicate::And(vec![left.clone(), right.clone()]),
                    SemanticPredicate::And(vec![left.negate(), right.negate()]),
                ])
            })
            .collect(),
    );
    let mut root = SemanticNode::sequence([
        SemanticNode::If {
            condition: SemanticOperand::new(equality),
            then_node: Box::new(block(0, vec![define(41)])),
            else_node: Some(Box::new(block(1, vec![define(42)]))),
        },
        block(2, vec![read(), read()]),
    ]);
    SemanticSiteNumbering::assign(&mut root).unwrap();
    let graph = ValueFlowGraph::build_source(&root, &Default::default()).unwrap();
    assert!(graph.proof_budget_exhausted());
    let key = SsaVar::new(17, 0);
    let definitions = &graph.definitions[&key];
    let mut uses = graph.uses[&key].to_vec();
    // A dead use retains no feasible candidate, even when the proof arena is saturated.
    uses[1].domain = graph.logic.falsity();
    let bdd = SemanticReachingAnalysis::new(&graph);
    let output = ReachingDefinitions::new(&graph, &bdd)
        .analyze(definitions, &uses)
        .unwrap();
    assert_eq!(output.candidates(&output.uses[0]), [0, 1]);
    assert!(output.candidates(&output.uses[1]).is_empty());
    assert_eq!(output.candidates(&output.uses[2]), [0, 1]);
    assert_eq!(output.candidates(&output.uses[3]), [0, 1]);
    assert_eq!(output.candidate_uses, [3, 3]);
    let mut missing = uses;
    missing[1].location = super::super::UseLocation::Observation(None);
    assert!(
        matches!(ReachingDefinitions::new(&graph,&bdd).analyze(definitions,&missing), Err(ValueRecoveryError::MissingUsePoint(k)) if k == key)
    );
}
