use super::*;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::ir::{
    analysis::SourceTypeEnvironment, ArgType, BlockId, InsnNode, LiteralArg, RegionId,
    SemanticBlock, SemanticCatch, SemanticExpression, SemanticLoopControl, SemanticOperand,
    SemanticStatement,
};
use crate::language::java::{DexJavaDialect, JavaLowerer, JavaMemberNames};

fn foreach(number: u32, body: SemanticNode) -> SemanticNode {
    SemanticNode::ForEach {
        control: SemanticLoopControl::Region(RegionId::new(number)),
        variable: RegisterArg::new(number, ArgType::INT),
        iterable: SemanticOperand::new(SemanticExpression::Literal(LiteralArg::int(0))),
        body: Box::new(body),
    }
}

#[test]
fn nested_foreach_and_catches_retain_their_lexical_binding_order() {
    let root = SemanticNode::Try {
        region: RegionId::new(1),
        body: Box::new(foreach(8, SemanticNode::Empty)),
        catches: vec![
            SemanticCatch {
                region: RegionId::new(2),
                exception_types: vec![],
                exception_value: Some(RegisterArg::new(3, ArgType::object("java/lang/Exception"))),
                body: foreach(6, SemanticNode::Empty),
            },
            SemanticCatch {
                region: RegionId::new(3),
                exception_types: vec![],
                exception_value: Some(RegisterArg::new(2, ArgType::object("java/lang/Exception"))),
                body: SemanticNode::Empty,
            },
        ],
        finally: None,
    };
    assert_eq!(
        JavaInputBindings::collect(&root)
            .finish()
            .unwrap()
            .into_iter()
            .map(|(kind, reg)| (kind, reg.reg_num))
            .collect::<Vec<_>>(),
        [
            (SemanticBindingKind::ForEach, 8),
            (SemanticBindingKind::Catch, 3),
            (SemanticBindingKind::ForEach, 6),
            (SemanticBindingKind::Catch, 2)
        ]
    );
}

fn dialect() -> DexJavaDialect {
    DexJavaDialect::new(
        true,
        None,
        &[],
        &[],
        &SourceTypeEnvironment::default(),
        BTreeMap::new(),
        Arc::new(JavaMemberNames::default()),
    )
    .unwrap()
}

#[test]
fn prepared_and_standalone_lowering_preserve_first_input_error_and_empty_body() {
    let instructions = [
        (InsnType::MoveException, 82),
        (InsnType::Phi, 9),
        (InsnType::Nop, 4),
    ]
    .into_iter()
    .map(|(opcode, offset)| {
        let mut instruction = InsnNode::new(opcode, 0);
        instruction.offset = offset;
        SemanticStatement::instruction(instruction).unwrap()
    })
    .collect();
    let root = foreach(
        73,
        SemanticNode::BasicBlock(SemanticBlock {
            id: BlockId::new(0),
            statements: instructions,
        }),
    );
    for result in [
        JavaLowerer::new(dialect()).lower(&root),
        dialect().lower_with_bindings(&root, JavaInputBindings::collect(&root)),
    ] {
        assert!(matches!(
            result,
            Err(JavaLoweringError::UnrecoveredExceptionValue(82))
        ));
    }
    let empty = SemanticNode::Empty;
    assert_eq!(
        JavaLowerer::new(dialect()).lower(&empty).unwrap(),
        dialect()
            .lower_with_bindings(&empty, JavaInputBindings::collect(&empty))
            .unwrap()
    );
}
