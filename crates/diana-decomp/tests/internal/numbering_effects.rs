use super::*;
use crate::ir::{InsnNode, RegisterArg};

fn op(kind: InsnType, arguments: Vec<InsnArg>) -> InsnNode {
    let mut instruction = InsnNode::new(kind, arguments.len());
    instruction.result = Some(RegisterArg::new_ssa(2, 1, ArgType::INT));
    instruction.args = arguments;
    instruction
}

fn number(value: InsnArg, canonical: BTreeMap<SsaVar, InsnArg>) -> Option<ValueExpression> {
    let root = SemanticOperation::from_instruction(op(InsnType::Neg, vec![value])).unwrap();
    ValueNumbering {
        identity: ValueIdentity::Ssa,
        logic: &DomainLogic::new(&BTreeSet::new()),
        scopes: &LexicalScopes::default(),
        canonical_values: &canonical,
        availability: None,
        aliases: BTreeMap::new(),
        classes: HashMap::new(),
    }
    .expression(&root)
}

#[test]
fn nested_and_canonical_trees_reject_throwing_edge_copy_and_unsupported_operations() {
    let literal = || InsnArg::lit(5, ArgType::INT);
    let mut division = op(InsnType::Arith, vec![literal(), literal()]);
    division.payload.arith_op = Some(ArithOp::Div);
    let mut compound = op(InsnType::Neg, vec![literal()]);
    compound.payload.compound_target = Some(Box::new(InsnArg::wrap(division.clone())));
    let mut edge_copy = op(InsnType::Neg, vec![literal()]);
    edge_copy.payload.edge_copy = true;
    let unsupported = op(InsnType::Move, vec![literal()]);
    for nested in [division, compound, edge_copy, unsupported] {
        let wrapped = InsnArg::wrap(op(InsnType::Neg, vec![InsnArg::wrap(nested)]));
        assert!(number(wrapped.clone(), BTreeMap::new()).is_none());
        let referenced = InsnArg::reg_ssa(3, 1, ArgType::INT);
        assert!(number(referenced, [(SsaVar::new(3, 1), wrapped)].into()).is_none());
    }
}

#[test]
fn nested_pure_numbering_retains_every_level_and_the_canonical_literal() {
    let mut expression = InsnArg::reg_ssa(3, 1, ArgType::INT);
    for _ in 0..40 {
        expression = InsnArg::wrap(op(InsnType::Neg, vec![expression]));
    }
    let key = number(
        expression,
        [(SsaVar::new(3, 1), InsnArg::lit(19, ArgType::INT))].into(),
    )
    .unwrap();
    assert_eq!(key.root.instruction, InsnType::Neg);
    assert_eq!(key.children.len(), 40);
    assert!(key
        .children
        .iter()
        .all(|child| child.instruction == InsnType::Neg && child.argument_count == 1));
    let (last, preceding) = key.operands.split_last().unwrap();
    assert_eq!(preceding.len(), 40);
    assert!(preceding
        .iter()
        .all(|slot| *slot == ValueOperand::Operation));
    assert!(matches!(last, ValueOperand::Literal { value: 19, ty } if *ty == ArgType::INT));
}

#[test]
fn sibling_operand_order_and_nested_operation_boundaries_distinguish_keys() {
    let lit = |n| InsnArg::lit(n, ArgType::INT);
    let sum = |left, right| {
        let mut arithmetic = op(InsnType::Arith, vec![left, right]);
        arithmetic.payload.arith_op = Some(ArithOp::Add);
        InsnArg::wrap(arithmetic)
    };
    let a = sum(sum(lit(1), lit(2)), sum(lit(3), lit(4)));
    let b = sum(sum(lit(1), lit(3)), sum(lit(2), lit(4)));
    let c = sum(lit(1), sum(sum(lit(2), lit(3)), lit(4)));
    let direct = number(a.clone(), BTreeMap::new()).unwrap();
    let canonical = number(
        InsnArg::reg_ssa(4, 1, ArgType::INT),
        [(SsaVar::new(4, 1), a)].into(),
    )
    .unwrap();
    assert_eq!(direct, canonical);
    assert_ne!(direct, number(b, BTreeMap::new()).unwrap());
    assert_ne!(direct, number(c, BTreeMap::new()).unwrap());
}
