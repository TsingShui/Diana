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
