use super::*;
use crate::ir::{
    BlockId, InsnNode, LiteralArg, RegionId, RegisterArg, SemanticBlock, SemanticLeave,
    SemanticLeaveKind, SemanticOperand, SemanticStatement,
};

fn returned() -> SemanticNode {
    let region = RegionId::new(0);
    SemanticNode::Leave(SemanticLeave {
        site: None,
        condition: None,
        kind: SemanticLeaveKind::Return(None),
        edge: None,
        origin: None,
        source: region,
        destination: region,
        target: region,
        cleanup: Vec::new(),
    })
}

fn assignment(block: u32, value: i32, variable: &RegisterArg) -> SemanticNode {
    SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(block),
        statements: vec![SemanticStatement::definition(
            InstructionId::new(block as usize),
            variable.clone(),
            SemanticExpression::Literal(LiteralArg::int(value)),
        )],
    })
}

#[test]
fn only_the_completing_arm_can_invalidate_a_known_boolean() {
    let mut flag = RegisterArg::new(0, ArgType::BOOLEAN);
    flag.code_var = Some(0);
    let mut test = InsnNode::new(InsnType::If, 0);
    test.payload.if_op = Some(IfOp::Ne);
    let test = SemanticOperation::from_parts(
        test,
        vec![
            SemanticExpression::Register(flag.clone()),
            SemanticExpression::Literal(LiteralArg::int(0)),
        ],
        None,
    );
    for overwritten in [false, true] {
        let condition = SemanticNode::If {
            condition: SemanticOperand::new(SemanticPredicate::Test(test.clone())),
            then_node: Box::new(returned()),
            else_node: overwritten.then(|| Box::new(assignment(2, 1, &flag))),
        };
        let mut root = SemanticNode::Sequence(vec![condition, assignment(3, 0, &flag)]);
        PathConditionSyntax
            .apply(&mut root, &SourceTypeEnvironment::default())
            .unwrap();
        let SemanticNode::Sequence(children) = &root else {
            panic!("expected the guarded tail");
        };
        let Some(SemanticNode::BasicBlock(last)) = children.last() else {
            panic!("expected the final write block");
        };
        assert_eq!(last.id, BlockId::new(3));
        assert_eq!(last.statements.len(), usize::from(overwritten));
    }
}

#[test]
fn repeated_effectful_test_in_a_nested_select_remains_volatile() {
    let mut raw = InsnNode::new(InsnType::If, 0);
    raw.id = InstructionId::new(19);
    raw.payload.if_op = Some(IfOp::Ne);
    let read = SemanticOperation::from_parts(InsnNode::new(InsnType::Sget, 0), Vec::new(), None);
    let test = SemanticPredicate::Test(SemanticOperation::from_parts(
        raw,
        vec![
            SemanticExpression::Operation(read),
            SemanticExpression::Literal(LiteralArg::int(0)),
        ],
        None,
    ));
    let selected = SemanticExpression::select(
        test.clone(),
        SemanticExpression::Literal(LiteralArg::int(1)),
        SemanticExpression::Literal(LiteralArg::int(2)),
    );
    let wrapped =
        SemanticOperation::from_parts(InsnNode::new(InsnType::Move, 0), vec![selected], None);
    let block = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(5),
        statements: vec![SemanticStatement::definition(
            InstructionId::new(31),
            RegisterArg::new(0, ArgType::INT),
            SemanticExpression::Operation(wrapped),
        )],
    });
    let root = SemanticNode::If {
        condition: SemanticOperand::new(test.clone()),
        then_node: Box::new(block),
        else_node: None,
    };
    let domain = PathConditionDomain::new(
        PredicateCatalog::collect(&root),
        &SourceTypeEnvironment::default(),
    );
    assert!(domain.is_volatile(&test));
}

#[test]
fn negated_complement_uses_the_representative_with_its_original_polarity() {
    let mut raw = InsnNode::new(InsnType::If, 0);
    raw.id = InstructionId::new(40);
    raw.payload.if_op = Some(IfOp::Gt);
    let comparison = SemanticOperation::from_parts(
        raw,
        vec![
            SemanticExpression::Register(RegisterArg::new_ssa(1, 0, ArgType::INT)),
            SemanticExpression::Register(RegisterArg::new_ssa(2, 0, ArgType::INT)),
        ],
        None,
    );
    let mut complement = comparison.clone();
    complement.id = InstructionId::new(41);
    complement.set_comparison_operator(IfOp::Le);
    let complement = SemanticPredicate::Test(complement);
    let root = SemanticNode::Sequence(vec![
        SemanticNode::If {
            condition: SemanticOperand::new(SemanticPredicate::Test(comparison)),
            then_node: Box::new(returned()),
            else_node: None,
        },
        SemanticNode::If {
            condition: SemanticOperand::new(complement.clone()),
            then_node: Box::new(returned()),
            else_node: None,
        },
    ]);
    let domain = PathConditionDomain::new(
        PredicateCatalog::collect(&root),
        &SourceTypeEnvironment::default(),
    );
    let expected = BoolExpr::instruction(InstructionId::new(40));
    assert_eq!(
        domain.predicate_expression(&complement).unwrap(),
        BoolExpr::not(expected.clone())
    );
    assert_eq!(
        domain
            .predicate_expression(&SemanticPredicate::Not(Box::new(complement)))
            .unwrap(),
        expected
    );
}
