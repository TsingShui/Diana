use super::*;
use crate::ir::{BlockId, InsnNode, InstructionId, LiteralArg, RegisterArg, SemanticBlock};

fn variable(id: u32, ty: ArgType) -> RegisterArg {
    let mut register = RegisterArg::new(id, ty);
    register.code_var = Some(id);
    register
}

fn definition(id: usize, result: RegisterArg, value: SemanticExpression) -> SemanticStatement {
    SemanticStatement::definition(InstructionId::new(id), result, value)
}

#[test]
fn boolean_assignment_refines_integer_storage_without_expression_rewrites() {
    let value = variable(0, ArgType::INT);
    let mut body = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements: vec![
            definition(
                0,
                value.clone(),
                SemanticExpression::Literal(LiteralArg::int(1)),
            ),
            definition(
                1,
                variable(1, ArgType::BOOLEAN),
                SemanticExpression::Register(value),
            ),
        ],
    });
    let recovery = ExpressionSyntax::apply(
        &mut body,
        &SourceTypeEnvironment::default(),
        BooleanUse::RefineTypes,
    )
    .unwrap();
    assert!(!recovery.changed);
    assert_eq!(recovery.boolean_variables, BTreeSet::from([0]));
}

#[test]
fn later_rewrites_reach_xor_under_an_unrelated_operation() {
    let source = variable(0, ArgType::BOOLEAN);
    let mut xor = InsnNode::new(InsnType::Arith, 0);
    xor.payload.arith_op = Some(ArithOp::Xor);
    xor.result = Some(variable(1, ArgType::INT));
    let xor = SemanticOperation::from_parts(
        xor,
        vec![
            SemanticExpression::Register(source),
            SemanticExpression::Literal(LiteralArg::int(1)),
        ],
        None,
    );
    let array = variable(2, ArgType::array(ArgType::BOOLEAN));
    let mut build = InsnNode::filled_new_array(0, Vec::new());
    build.result = Some(array.clone());
    let build =
        SemanticOperation::from_parts(build, vec![SemanticExpression::Operation(xor)], None);
    let mut body = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements: vec![definition(2, array, SemanticExpression::Operation(build))],
    });
    let recovery = ExpressionSyntax::apply(
        &mut body,
        &SourceTypeEnvironment::default(),
        BooleanUse::Rewrite,
    )
    .unwrap();
    assert!(recovery.changed);
    let SemanticNode::BasicBlock(block) = body else {
        unreachable!()
    };
    let SemanticStatementKind::Definition { value, .. } = &block.statements[0].kind else {
        unreachable!()
    };
    let build = value.as_operation().unwrap();
    assert_eq!(build.insn_type, InsnType::FilledNewArray);
    assert_eq!(
        build.operands()[0].as_operation().unwrap().insn_type,
        InsnType::Not
    );
}
