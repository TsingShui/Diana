use super::*;

fn operation(
    kind: InsnType,
    operands: Vec<SemanticExpression>,
    target: Option<SemanticExpression>,
) -> SemanticOperation {
    SemanticOperation::from_parts(InsnNode::new(kind, 0), operands, target)
}

fn expression(kind: InsnType) -> SemanticExpression {
    SemanticExpression::Operation(Box::new(operation(kind, Vec::new(), None)))
}

#[test]
fn shared_and_edited_operands_have_independent_effects() {
    let mut root = operation(
        InsnType::Neg,
        vec![expression(InsnType::Iget)],
        Some(expression(InsnType::MonitorEnter)),
    );
    let result = root.effects();
    assert!(result.reads_memory() && result.may_throw() && result.synchronizes());
    let sibling = root.clone();

    let SemanticExpression::Operation(mut_target) = root.compound_target_mut().unwrap() else {
        unreachable!()
    };
    mut_target.insn_type = InsnType::Nop;
    let edited = root.effects();
    assert!(edited.reads_memory() && edited.may_throw());
    assert!(!edited.synchronizes());
    let SemanticExpression::Operation(child) = &mut root.operands_mut()[0] else {
        unreachable!()
    };
    child.insn_type = InsnType::Nop;
    assert!(root.effects().is_pure());

    *root.children_mut().next().unwrap() = expression(InsnType::Return);
    assert_eq!(root.effects(), InstructionEffects::CONTROL);
    assert_eq!(sibling.effects(), result);
}

#[test]
fn predicate_masks_only_its_root_control_and_edge_copy_overrides_only_its_root() {
    let mut test = operation(InsnType::If, vec![expression(InsnType::Return)], None);
    test.payload.edge_copy = true;
    let select = |test| SemanticExpression::Select {
        condition: SemanticPredicate::Not(Box::new(SemanticPredicate::Test(test))),
        when_true: Box::new(expression(InsnType::Sget)),
        when_false: Box::new(expression(InsnType::Nop)),
    };
    let result = select(test.clone()).effects();
    assert!(
        result.controls() && result.reads_memory() && result.writes_memory() && result.may_throw()
    );
    test.operands_mut()[0] = expression(InsnType::Nop);
    let result = select(test.clone()).effects();
    assert!(!result.controls());
    assert!(result.writes_memory());
    assert_eq!(
        test.effects_ignoring_edge_copy(),
        InstructionEffects::CONTROL
    );
    let outer = operation(
        InsnType::Neg,
        vec![SemanticExpression::Operation(Box::new(test))],
        None,
    );
    assert!(outer.effects_ignoring_edge_copy().writes_memory());
}

#[test]
fn deep_shared_operations_cache_their_operand_summaries() {
    let mut child = operation(InsnType::Iget, vec![], None);
    for _ in 0..4096 {
        child = operation(
            InsnType::Neg,
            vec![SemanticExpression::Operation(Box::new(child))],
            None,
        );
    }
    assert!(child.effects().reads_memory());
    let mut current = &child;
    while let [SemanticExpression::Operation(inner)] = current.operands() {
        assert!(current.operand_effects.get().unwrap().reads_memory());
        current = inner;
    }
    assert_eq!(
        current.operand_effects.get(),
        Some(InstructionEffects::PURE)
    );
}
