use super::{LexicalScopes, RecoveryMode, ScopeId, ValueFlowGraph, ValueRecoveryError};
use crate::ir::{
    analysis::SourceEvaluationError, ArgType, BlockId, InsnArg, InsnNode, InsnType, SemanticBlock,
    SemanticNode, SemanticSiteId, SemanticStatement,
};
use std::collections::BTreeSet;

#[test]
fn source_plan_preserves_operand_errors_without_definitions() {
    let mut first = InsnNode::new(InsnType::Aput, 1);
    first.offset = 17;
    first.args.push(InsnArg::lit(8, ArgType::INT));
    let mut later = InsnNode::new(InsnType::Iput, 0);
    later.offset = 30;
    let node = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements: [first, later]
            .into_iter()
            .enumerate()
            .map(|(site, instruction)| {
                let mut statement =
                    SemanticStatement::instruction(instruction).expect("semantic operation");
                statement.site = Some(SemanticSiteId(site as u64));
                statement
            })
            .collect(),
    });
    for mode in [RecoveryMode::Structural, RecoveryMode::Full] {
        let result = ValueFlowGraph::plan_source(&node, &BTreeSet::new(), &mut None, mode);
        assert!(matches!(
            result,
            Err(ValueRecoveryError::SourceEvaluation(
                SourceEvaluationError::InvalidArity {
                    instruction: InsnType::Aput,
                    offset: 17,
                    actual: 1
                }
            ))
        ));
    }
}

#[test]
fn scope_intervals_include_only_lexical_ancestors() {
    let mut scopes = LexicalScopes::default();
    let root = ScopeId::default();
    let protected = scopes.enter();
    let body = scopes.enter();
    scopes.close(body);
    let nested = scopes.enter();
    let handler = scopes.enter();
    scopes.close(handler);
    scopes.close(nested);
    scopes.close(protected);
    let following = scopes.enter();
    scopes.close(following);
    let expected = [
        (root, vec![root]),
        (protected, vec![root, protected]),
        (body, vec![root, protected, body]),
        (nested, vec![root, protected, nested]),
        (handler, vec![root, protected, nested, handler]),
        (following, vec![root, following]),
    ];
    for (inner, ancestors) in &expected {
        for (candidate, _) in &expected {
            assert_eq!(
                scopes.contains(*candidate, *inner),
                ancestors.contains(candidate)
            );
        }
    }
}

#[test]
fn empty_ssa_plan_preserves_unknown_input_diagnostics() {
    use crate::ir::analysis::{SsaValueGraph, SsaVar};

    let mut instruction = InsnNode::new(InsnType::Return, 1);
    instruction.args.push(InsnArg::reg_ssa(4, 27, ArgType::INT));
    let node = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements: vec![SemanticStatement::instruction(instruction).unwrap()],
    });
    let unknown = ValueFlowGraph::plan_ssa(
        &node,
        &SsaValueGraph::default(),
        &Default::default(),
        &mut None,
        |graph| graph.schedule(RecoveryMode::Full),
    );
    assert!(
        matches!(unknown, Err(ValueRecoveryError::UnknownValue(value)) if value == SsaVar::new(4, 27))
    );
}

#[test]
fn empty_source_schedule_reduces_selects_nested_in_an_operand() {
    use crate::ir::{
        InstructionId, LiteralArg, RegisterArg, SemanticExpression, SemanticOperation,
        SemanticPredicate,
    };

    let selected = SemanticExpression::select(
        SemanticPredicate::True,
        SemanticExpression::Literal(LiteralArg::new(8, ArgType::INT)),
        SemanticExpression::Literal(LiteralArg::new(2, ArgType::INT)),
    );
    let nested = SemanticExpression::Operation(Box::new(SemanticOperation::from_parts(
        InsnNode::new(InsnType::Neg, 1),
        vec![selected],
        None,
    )));
    let mut statement = SemanticStatement::definition(
        InstructionId::new(1),
        RegisterArg::new(0, ArgType::INT),
        nested,
    );
    statement.site = Some(SemanticSiteId(3));
    let mut node = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements: vec![statement],
    });
    let plan =
        ValueFlowGraph::plan_source(&node, &BTreeSet::new(), &mut None, RecoveryMode::Structural)
            .unwrap();
    assert!(plan.actions.is_empty());
    assert!(plan.apply(&mut node, super::ValueIdentity::Source).unwrap());
    assert!(!super::super::schedule::ValueSchedule::contains_select(
        &node
    ));
}

#[test]
fn constants_without_semantic_uses_remain_in_the_ssa_product() {
    use crate::ir::analysis::{SsaValueGraph, SsaVar};
    let key = SsaVar::new(3, 12);
    let constants = [(key, InsnArg::lit(83, ArgType::INT))].into();
    let plan = ValueFlowGraph::plan_ssa(
        &SemanticNode::Empty,
        &SsaValueGraph::default(),
        &constants,
        &mut None,
        |graph| graph.schedule(RecoveryMode::Full),
    )
    .unwrap();
    assert!(plan.actions.is_empty());
    assert_eq!(
        plan.constants.get(&key).and_then(InsnArg::literal_value),
        Some(83)
    );
}
