use super::*;

fn block(id: BlockId, nested_instruction: InstructionId) -> SemanticNode {
    let register = || RegisterArg::new(2, ArgType::INT);
    let mut nested = InsnNode::const_value(register(), 17);
    nested.id = nested_instruction;
    let statement =
        SemanticStatement::instruction(InsnNode::mov(register(), InsnArg::wrap(nested))).unwrap();
    SemanticNode::BasicBlock(SemanticBlock {
        id,
        statements: vec![statement],
    })
}

fn body_with_exit(body: SemanticNode) -> SemanticNode {
    let owner = RegionId::new(3);
    let label = SemanticLabel::block(owner, BlockId::new(12));
    let leave = SemanticNode::Leave(SemanticLeave {
        site: None,
        condition: None,
        kind: SemanticLeaveKind::BreakLabel(label),
        edge: None,
        origin: None,
        source: owner,
        destination: owner,
        target: owner,
        cleanup: Vec::new(),
    });
    SemanticNode::Label {
        label,
        body: Box::new(SemanticNode::Sequence(vec![body, leave])),
    }
}

#[test]
fn lexical_exits_preserve_block_exhaustion_and_scan_nested_instruction_ids_on_demand() {
    let mut types = SourceTypeEnvironment::default();
    let mut missing_block_id = block(BlockId::INVALID, InstructionId::INVALID);
    assert!(matches!(
        LexicalExitLowering::apply(&mut missing_block_id, 0, &mut types),
        Err(SourceVariableError::SyntheticIdentityExhausted)
    ));

    let mut no_exit = block(BlockId::new(1), InstructionId::new(usize::MAX - 1));
    LexicalExitLowering::apply(&mut no_exit, 0, &mut types).unwrap();
    let mut exhausted = body_with_exit(no_exit);
    assert!(matches!(
        LexicalExitLowering::apply(&mut exhausted, 0, &mut types),
        Err(SourceVariableError::SyntheticIdentityExhausted)
    ));

    let mut body = body_with_exit(block(BlockId::new(3), InstructionId::new(95)));
    LexicalExitLowering::apply(&mut body, 7, &mut types).unwrap();
    #[derive(Default)]
    struct Ids(Vec<usize>);
    impl SemanticVisitor for Ids {
        fn enter_operation(&mut self, operation: &crate::ir::SemanticOperation) {
            if operation.id.is_valid() {
                self.0.push(operation.id.raw());
            }
        }
    }
    let mut ids = Ids::default();
    ids.visit_node(&body);
    ids.0.sort_unstable();
    assert_eq!(ids.0, [95, 96, 97]);
}
