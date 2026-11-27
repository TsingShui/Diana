use super::*;

fn operation(
    kind: InsnType,
    operands: Vec<SemanticExpression>,
    target: Option<SemanticExpression>,
) -> SemanticOperation {
    SemanticOperation::from_parts(InsnNode::new(kind, 0), operands, target)
}

fn expression(kind: InsnType) -> SemanticExpression {
    SemanticExpression::Operation(operation(kind, Vec::new(), None))
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
        vec![SemanticExpression::Operation(test)],
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
            vec![SemanticExpression::Operation(child)],
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

#[test]
fn nested_shapes_distinguish_selects_and_arity_across_shared_edits() {
    let mut root = operation(
        InsnType::Neg,
        vec![expression(InsnType::Nop)],
        Some(expression(InsnType::Aput)),
    );
    assert_eq!(root.operand_shape(), OperandShape::InvalidEvaluation);
    let original = root.clone();
    *root.compound_target_mut().unwrap() = expression(InsnType::Nop);
    assert_eq!(root.operand_shape(), OperandShape::Plain);
    let plain = root.clone();
    *root.operands_mut().first_mut().unwrap() = SemanticExpression::Select {
        condition: SemanticPredicate::False,
        when_true: Box::new(expression(InsnType::Iput)),
        when_false: Box::new(expression(InsnType::Nop)),
    };
    assert!(root.contains_select());
    assert!(!original.contains_select());
    assert_eq!(plain.operand_shape(), OperandShape::Plain);
    root.insn_type = InsnType::Aput;
    assert!(root.contains_select());
    *root.children_mut().next().unwrap() = expression(InsnType::Nop);
    assert_eq!(root.operand_shape(), OperandShape::InvalidEvaluation);
}

#[test]
fn shared_deep_expression_sets_structure_once_per_operation() {
    let mut tree = operation(InsnType::Iput, vec![], None);
    for _ in 0..4096 {
        tree = operation(
            InsnType::Neg,
            vec![SemanticExpression::Operation(tree)],
            None,
        );
    }
    let root = operation(
        InsnType::Move,
        vec![SemanticExpression::Operation(tree.clone())],
        Some(SemanticExpression::Operation(tree)),
    );
    assert_eq!(root.operand_shape(), OperandShape::InvalidEvaluation);
    let mut current = &root;
    while let [SemanticExpression::Operation(child)] = current.operands() {
        assert_eq!(
            current.operand_shape.get(),
            Some(OperandShape::InvalidEvaluation)
        );
        current = child;
    }
    assert_eq!(
        current.operand_shape.get(),
        Some(OperandShape::InvalidEvaluation)
    );
}

#[test]
fn source_identity_binding_detaches_shared_predicates_and_compound_operands() {
    let register = |n| SemanticExpression::Register(RegisterArg::with_ssa(n, ArgType::INT, 2));
    let mut test = operation(InsnType::If, vec![register(1)], None);
    test.result = Some(RegisterArg::with_ssa(4, ArgType::BOOLEAN, 3));
    let select =
        SemanticExpression::select(SemanticPredicate::Test(test), register(2), register(3));
    let mut root = operation(InsnType::Iget, vec![select], Some(register(5)));
    let original = root.clone();
    let effects = root.effects();
    let shape = root.operand_shape();
    RegisterSite::Operation(&mut root).bind(&|register| Some(register.reg_num + 10));

    for (operation, bound) in [(&root, true), (&original, false)] {
        let SemanticExpression::Select {
            condition: SemanticPredicate::Test(test),
            when_true,
            when_false,
        } = &operation.operands()[0]
        else {
            panic!("expected selected operand")
        };
        let registers = [
            test.operands()[0].as_register().unwrap(),
            when_true.as_register().unwrap(),
            when_false.as_register().unwrap(),
            test.result.as_ref().unwrap(),
            operation.compound_target().unwrap().as_register().unwrap(),
        ];
        for (reg, n) in registers.into_iter().zip(1..=5) {
            assert_eq!(reg.reg_num, n);
            assert_eq!(reg.code_var, bound.then_some(n + 10));
        }
        assert_eq!(operation.effects(), effects);
        assert_eq!(operation.operand_shape(), shape);
    }
}
