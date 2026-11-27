use super::{SemanticExpressionReduction, ValueIdentity};
use crate::ir::{
    InsnNode, InsnType, InstructionId, LiteralArg, SemanticExpression, SemanticOperation,
    SemanticPredicate,
};

fn literal(value: i32) -> SemanticExpression {
    SemanticExpression::Literal(LiteralArg::int(value))
}

#[test]
fn selection_reduction_preserves_plain_operands_and_reduces_the_compound_target() {
    let sibling = SemanticOperation::from_parts(InsnNode::new(InsnType::Const, 0), vec![], None);
    let mut instruction = InsnNode::new(InsnType::Arith, 0);
    instruction.id = InstructionId::new(23);
    instruction.offset = 17;
    let root = SemanticOperation::from_parts(
        instruction,
        vec![
            SemanticExpression::Operation(sibling.clone()),
            SemanticExpression::select(SemanticPredicate::True, literal(2), literal(3)),
        ],
        Some(SemanticExpression::select(
            SemanticPredicate::False,
            literal(5),
            literal(7),
        )),
    );
    let retained = root.clone();
    let (result, changed) = SemanticExpressionReduction::new(ValueIdentity::Ssa)
        .reduce(SemanticExpression::Operation(root))
        .unwrap();
    assert!(changed);
    let SemanticExpression::Operation(result) = result else {
        panic!("expected operation")
    };
    let [SemanticExpression::Operation(plain), chosen] = result.operands() else {
        panic!("expected the original two operands")
    };
    assert!(std::ptr::eq(&**plain, &*sibling));
    assert_eq!(chosen.literal_value(), Some(2));
    assert_eq!(
        result
            .compound_target()
            .and_then(SemanticExpression::literal_value),
        Some(7)
    );
    assert_eq!(result.id, InstructionId::new(23));
    assert_eq!(result.offset, 17);
    assert!(matches!(
        retained.operands()[1],
        SemanticExpression::Select { .. }
    ));
    assert!(matches!(
        retained.compound_target(),
        Some(SemanticExpression::Select { .. })
    ));
}
