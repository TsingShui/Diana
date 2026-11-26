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
    let mut key = &key;
    for _ in 0..40 {
        assert_eq!(key.instruction, InsnType::Neg);
        let [ValueOperand::Expression(child)] = key.arguments.as_slice() else {
            panic!("nested negation")
        };
        key = child;
    }
    assert_eq!(key.instruction, InsnType::Neg);
    assert!(
        matches!(key.arguments.as_slice(), [ValueOperand::Literal {value: 19, ty}] if *ty == ArgType::INT)
    );
}
