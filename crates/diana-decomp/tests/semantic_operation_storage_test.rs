use diana_decomp::ir::{
    ArgType, InsnArg, InsnNode, LiteralArg, RegisterArg, SemanticExpression, SemanticOperation,
};

#[test]
fn editing_a_nested_operation_preserves_other_sites() {
    let result = RegisterArg::with_ssa(1, ArgType::INT, 3);
    let constant = InsnNode::const_value(result.clone(), 17);
    let returned = InsnNode::return_value(InsnArg::wrap(constant));
    let original = SemanticOperation::from_instruction(returned).unwrap();
    let mut edited = original.clone();
    let SemanticExpression::Operation(constant) = &mut edited.operands_mut()[0] else {
        panic!("expected a nested constant")
    };
    constant.discard_result();
    constant.operands_mut()[0] = SemanticExpression::Literal(LiteralArg::int(23));
    edited.offset = 8;

    let untouched = original.operands()[0].as_operation().unwrap();
    let changed = edited.operands()[0].as_operation().unwrap();
    assert_eq!(original.offset, 0);
    assert_eq!(edited.offset, 8);
    assert_eq!(untouched.result.as_ref(), Some(&result));
    assert!(changed.result.is_none());
    assert_eq!(untouched.operands()[0].literal_value(), Some(17));
    assert_eq!(changed.operands()[0].literal_value(), Some(23));

    let shared = original.clone();
    for operation in [original, shared] {
        let (instruction, operands, target) = operation.into_parts();
        assert_eq!(instruction.insn_type, diana_decomp::ir::InsnType::Return);
        assert_eq!(instruction.offset, 0);
        assert!(instruction.args.is_empty());
        assert!(target.is_none());
        assert_eq!(
            operands[0].as_operation().unwrap().result.as_ref(),
            Some(&result)
        );
    }
}

#[test]
fn the_last_owner_releases_a_deep_operation_chain_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let parent = SemanticOperation::from_instruction(InsnNode::return_value(InsnArg::lit(
                1,
                ArgType::INT,
            )))
            .unwrap();
            let mut chain = SemanticOperation::from_instruction(InsnNode::return_void()).unwrap();
            for _ in 0..16_384 {
                let mut node = parent.clone();
                node.operands_mut()[0] = SemanticExpression::Operation(Box::new(chain));
                chain = node;
            }
            let other_owner = chain.clone();
            drop(chain);
            drop(other_owner);
        })
        .unwrap()
        .join()
        .unwrap();
}
