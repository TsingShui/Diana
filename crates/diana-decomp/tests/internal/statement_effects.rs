use super::*;
use crate::ir::analysis::{SourceEvaluationError, SsaVar};
use crate::ir::{
    ArgType, BlockId, InsnNode, InstructionId, LiteralArg, RegisterArg, SemanticBlock,
    SemanticSiteId,
};
use std::collections::BTreeSet;

fn operation(kind: InsnType, operands: Vec<SemanticExpression>) -> SemanticOperation {
    let mut raw = InsnNode::new(kind, operands.len());
    raw.id = InstructionId::new(13);
    SemanticOperation::from_parts(raw, operands, None)
}

fn nested(operation: SemanticOperation) -> SemanticExpression {
    SemanticExpression::Operation(Box::new(operation))
}

fn literal() -> SemanticExpression {
    SemanticExpression::Literal(LiteralArg::new(5, ArgType::INT))
}

fn value() -> SemanticExpression {
    let mut register = RegisterArg::new(3, ArgType::INT);
    register.code_var = Some(3);
    SemanticExpression::Register(register)
}

fn definition(value: SemanticExpression) -> SemanticStatementKind {
    SemanticStatementKind::Definition {
        id: InstructionId::new(1),
        result: RegisterArg::new(1, ArgType::INT),
        value,
    }
}

fn block(statements: Vec<SemanticStatementKind>) -> SemanticNode {
    SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements: statements
            .into_iter()
            .enumerate()
            .map(|(site, kind)| SemanticStatement {
                site: Some(SemanticSiteId(site as u64)),
                origin: None,
                kind,
            })
            .collect(),
    })
}

#[test]
fn store_uses_receiver_before_value_and_keeps_both_field_locations() {
    let mut read = InsnNode::new(InsnType::Sget, 0);
    read.payload.field_index = Some(5);
    let read = SemanticOperation::from_instruction(read).unwrap();
    let wrapped = operation(InsnType::Neg, vec![nested(read.clone())]);
    let mut store = InsnNode::new(InsnType::Iput, 2);
    store.payload.field_index = Some(8);
    let store = SemanticOperation::from_parts(store, vec![value(), nested(wrapped)], None);
    let stored_effect = EffectSummary::direct(&store).join(EffectSummary::direct(&read));
    let root = block(vec![SemanticStatementKind::Instruction(store)]);
    let graph = ValueFlowGraph::build_source(&root, &BTreeSet::new()).unwrap();
    assert_eq!(graph.effects.len(), 1);
    assert_eq!(graph.effects[0].effect, stored_effect);
    assert_eq!(
        graph.uses[&SsaVar::new(3, 0)][0].evaluation_prefix,
        EffectSummary::direct(&read)
    );
}

#[test]
fn selected_condition_control_applies_to_its_arm_use_only() {
    let condition = operation(InsnType::If, Vec::new());
    let selected = SemanticExpression::select(
        SemanticPredicate::Test(condition.clone()),
        value(),
        literal(),
    );
    for statement in [
        definition(selected.clone()),
        SemanticStatementKind::Instruction(operation(InsnType::Neg, vec![selected])),
    ] {
        let root = block(vec![statement]);
        let graph = ValueFlowGraph::build_source(&root, &BTreeSet::new()).unwrap();
        assert!(graph.effects.is_empty());
        assert_eq!(
            graph.uses[&SsaVar::new(3, 0)][0].evaluation_prefix,
            EffectSummary::direct(&condition)
        );
    }
}

#[test]
fn only_the_outer_edge_copy_is_ignored_for_the_statement_effect() {
    let mut copy = InsnNode::new(InsnType::Neg, 0);
    copy.payload.edge_copy = true;
    let copy = SemanticOperation::from_instruction(copy).unwrap();
    let root = block(vec![
        SemanticStatementKind::Instruction(copy.clone()),
        definition(nested(copy.clone())),
        definition(nested(operation(InsnType::Neg, vec![nested(copy.clone())]))),
    ]);
    let graph = ValueFlowGraph::build_source(&root, &BTreeSet::new()).unwrap();
    assert_eq!(graph.effects.len(), 1);
    assert_eq!(
        graph.effects[0].point,
        crate::ir::analysis::SemanticFlowPoint::after(SemanticSiteId(2))
    );
    assert_eq!(graph.effects[0].effect, EffectSummary::direct(&copy));
}

#[test]
fn omitted_constructor_receivers_and_store_operands_still_contribute_effects() {
    let monitor = operation(InsnType::MonitorEnter, vec![literal()]);
    let receiver = nested(monitor.clone());
    for (kind, arguments, target) in [
        (
            InsnType::Constructor,
            vec![receiver.clone(), literal()],
            None,
        ),
        (
            InsnType::CompoundAssign,
            vec![receiver.clone(), literal()],
            Some(literal()),
        ),
        (
            InsnType::Iput,
            vec![literal(), literal()],
            Some(receiver.clone()),
        ),
        (
            InsnType::Aput,
            vec![literal(), literal(), literal()],
            Some(receiver),
        ),
    ] {
        let op =
            SemanticOperation::from_parts(InsnNode::new(kind, arguments.len()), arguments, target);
        let expected = EffectSummary::direct(&op).join(EffectSummary::direct(&monitor));
        let outer = operation(InsnType::Neg, vec![nested(op)]);
        let root = block(vec![definition(nested(outer))]);
        let graph = ValueFlowGraph::build_source(&root, &BTreeSet::new()).unwrap();
        assert_eq!(graph.effects.len(), 1);
        assert_eq!(graph.effects[0].effect, expected);
    }
}

#[test]
fn malformed_receiver_is_reported_before_malformed_stored_value() {
    let mut receiver = InsnNode::new(InsnType::Aput, 0);
    receiver.offset = 37;
    let receiver = SemanticOperation::from_instruction(receiver).unwrap();
    let stored = operation(InsnType::Constructor, Vec::new());
    let root = block(vec![SemanticStatementKind::Instruction(operation(
        InsnType::Iput,
        vec![nested(stored), nested(receiver)],
    ))]);
    assert!(matches!(
        ValueFlowGraph::build_source(&root, &BTreeSet::new()),
        Err(ValueRecoveryError::SourceEvaluation(
            SourceEvaluationError::InvalidArity {
                instruction: InsnType::Aput,
                offset: 37,
                actual: 0,
            }
        ))
    ));
}
