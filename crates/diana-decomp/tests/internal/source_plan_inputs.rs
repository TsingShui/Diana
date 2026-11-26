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
        let result =
            ValueFlowGraph::plan_source(&node, &BTreeSet::new(), &Default::default(), mode);
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
        &Default::default(),
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
    let plan = ValueFlowGraph::plan_source(
        &node,
        &BTreeSet::new(),
        &Default::default(),
        RecoveryMode::Structural,
    )
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
        &Default::default(),
        |graph| graph.schedule(RecoveryMode::Full),
    )
    .unwrap();
    assert!(plan.actions.is_empty());
    assert_eq!(
        plan.constants.get(&key).and_then(InsnArg::literal_value),
        Some(83)
    );
}

#[test]
fn dead_constant_is_scheduled_before_any_flow_query() {
    use crate::ir::{InstructionId, LiteralArg, RegisterArg, SemanticExpression};

    let mut local = RegisterArg::new(1, ArgType::INT);
    local.code_var = Some(1);
    let mut statement = SemanticStatement::definition(
        InstructionId::new(0),
        local,
        SemanticExpression::Literal(LiteralArg::new(94, ArgType::INT)),
    );
    statement.site = Some(SemanticSiteId(0));
    let root = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements: vec![statement],
    });
    let graph = ValueFlowGraph::build_source(&root, &BTreeSet::new()).unwrap();
    let scheduled_flow = graph.semantic_flow.as_ref().unwrap();
    assert!(scheduled_flow.graph.get().is_none());
    let plan = graph.schedule(RecoveryMode::Full).unwrap();
    assert!(!plan.actions.is_empty());
    assert!(scheduled_flow.graph.get().is_none());
    assert!(graph.semantic_flow().unwrap().is_complete());
    assert!(scheduled_flow.graph.get().is_some());
}

#[test]
fn a_single_guard_needs_only_operand_and_select_facts_without_definitions() {
    use super::ControlSymbolClosure;
    use crate::ir::{InstructionId, SemanticOperand, SemanticOperation, SemanticPredicate};

    let mut op = InsnNode::new(InsnType::If, 0);
    op.id = InstructionId::new(4);
    let guard = SemanticPredicate::Test(SemanticOperation::from_instruction(op).unwrap());
    let body = SemanticNode::If {
        condition: SemanticOperand::new(SemanticPredicate::Or(vec![
            guard.clone(),
            SemanticPredicate::Not(Box::new(guard)),
        ])),
        then_node: Box::new(SemanticNode::Empty),
        else_node: None,
    };
    assert!(!ControlSymbolClosure::for_source(&body).plan_required);
    for mode in [RecoveryMode::Structural, RecoveryMode::Full] {
        let complete = ValueFlowGraph::build_source(&body, &BTreeSet::new()).unwrap();
        assert!(complete.schedule(mode).unwrap().actions.is_empty());
        let planned =
            ValueFlowGraph::plan_source(&body, &BTreeSet::new(), &Default::default(), mode)
                .unwrap();
        assert!(planned.actions.is_empty());
    }
    let mut malformed = InsnNode::new(InsnType::Aput, 0);
    malformed.id = InstructionId::new(4);
    malformed.offset = 7;
    let malformed = SemanticNode::If {
        condition: SemanticOperand::new(SemanticPredicate::Test(
            SemanticOperation::from_instruction(malformed).unwrap(),
        )),
        then_node: Box::new(SemanticNode::Empty),
        else_node: None,
    };
    let result = ValueFlowGraph::plan_source(
        &malformed,
        &BTreeSet::new(),
        &Default::default(),
        RecoveryMode::Full,
    );
    assert!(matches!(
        result,
        Err(ValueRecoveryError::SourceEvaluation(
            SourceEvaluationError::InvalidArity { offset: 7, .. }
        ))
    ));
}

#[test]
fn nested_junctions_preserve_order_subsets_and_cross_operand_complements() {
    use super::super::domain::DomainLogic;
    use crate::ir::{BoolExpr, BoolVariable, InstructionId};

    let variables = (0..7)
        .map(|n| BoolVariable::Instruction(InstructionId::new(n)))
        .collect::<BTreeSet<_>>();
    let mut logic = DomainLogic::new(&variables);
    let symbols = (0..7)
        .map(|n| {
            logic
                .compile(&BoolExpr::instruction(InstructionId::new(n)))
                .unwrap()
        })
        .collect::<Vec<_>>();
    for conjunction in [true, false] {
        let combine = |logic: &mut DomainLogic, a, b| {
            if conjunction {
                logic.and(a, b).unwrap()
            } else {
                logic.or(a, b).unwrap()
            }
        };
        let left = [0, 2, 4, 6]
            .into_iter()
            .map(|n| symbols[n])
            .reduce(|a, b| combine(&mut logic, a, b))
            .unwrap();
        let right = [1, 2, 3, 5]
            .into_iter()
            .map(|n| symbols[n])
            .reduce(|a, b| combine(&mut logic, a, b))
            .unwrap();
        let expected = symbols
            .iter()
            .copied()
            .reduce(|a, b| combine(&mut logic, a, b))
            .unwrap();
        assert_eq!(combine(&mut logic, left, right), expected);
        assert_eq!(combine(&mut logic, expected, left), expected);
        assert_eq!(combine(&mut logic, right, expected), expected);
        let inverse = logic.not(symbols[4]).unwrap();
        let opposite = combine(&mut logic, right, inverse);
        let absorbing = if conjunction {
            logic.falsity()
        } else {
            logic.truth()
        };
        assert_eq!(combine(&mut logic, left, opposite), absorbing);
        assert_eq!(combine(&mut logic, opposite, left), absorbing);
    }
}

#[test]
fn sequence_child_fallthrough_controls_nested_and_following_blocks() {
    use crate::ir::{
        InstructionId, RegionId, SemanticLeave, SemanticLeaveKind, SemanticOperand,
        SemanticOperation, SemanticPredicate,
    };

    let mut op = InsnNode::new(InsnType::If, 0);
    op.id = InstructionId::new(9);
    let guard = SemanticPredicate::Test(SemanticOperation::from_instruction(op).unwrap());
    let block = |n| {
        SemanticNode::BasicBlock(SemanticBlock {
            id: BlockId::new(n),
            statements: vec![],
        })
    };
    let exit = || {
        let region = RegionId::new(1);
        SemanticNode::Leave(SemanticLeave {
            site: None,
            condition: None,
            kind: SemanticLeaveKind::Return(None),
            edge: None,
            origin: None,
            source: region,
            destination: region,
            target: region,
            cleanup: Vec::new(),
        })
    };
    let root = SemanticNode::Sequence(vec![
        SemanticNode::If {
            condition: SemanticOperand::new(guard.clone()),
            then_node: Box::new(SemanticNode::Sequence(vec![block(0), exit(), block(1)])),
            else_node: Some(Box::new(block(2))),
        },
        block(3),
        SemanticNode::Sequence(vec![
            SemanticNode::If {
                condition: SemanticOperand::new(SemanticPredicate::Not(Box::new(guard.clone()))),
                then_node: Box::new(SemanticNode::Sequence(vec![block(4), exit(), block(5)])),
                else_node: None,
            },
            block(6),
        ]),
        block(7),
    ]);
    let mut graph = ValueFlowGraph::build_source(&root, &BTreeSet::new()).unwrap();
    let yes = graph.logic.predicate(&guard).unwrap();
    let no = graph.logic.not(yes).unwrap();
    let dead = graph.logic.falsity();
    for (id, expected) in [yes, dead, no, no, no, dead, dead, dead]
        .into_iter()
        .enumerate()
    {
        let observed = graph.block_domains[&BlockId::new(id as u32)];
        assert!(
            graph.logic.equivalent(observed, expected).unwrap(),
            "block {id}"
        );
    }
}

#[test]
fn filtered_phi_queries_use_the_original_order_and_merge_inputs() {
    use crate::ir::analysis::{PhiInput, PhiMerge, SsaValueGraph, SsaVar};
    use crate::ir::{EdgeKind, InstructionId};

    let keys = [SsaVar::new(7, 4), SsaVar::new(2, 1), SsaVar::new(5, 6)];
    let values = SsaValueGraph::with_phis_for_test(
        keys.iter()
            .enumerate()
            .map(|(i, &result)| PhiMerge {
                result,
                block: BlockId::new(3 + i as u32),
                instruction: InstructionId::new(20 + i),
                inputs: vec![PhiInput {
                    predecessor: BlockId::new(2),
                    edge_kind: EdgeKind::Normal,
                    value: keys[(i + 1) % keys.len()],
                }],
            })
            .collect(),
    );
    let canonical = Default::default();
    let mut graph = ValueFlowGraph::build(&SemanticNode::Empty, &values, &canonical).unwrap();
    let snapshot =
        |graph: &ValueFlowGraph<'_>| graph.phis().map(|phi| phi.result).collect::<Vec<_>>();
    assert_eq!(snapshot(&graph), keys);
    graph.exclude_phis(&[keys[1]].into());
    assert_eq!(snapshot(&graph), [keys[0], keys[2]]);
    assert!(graph.phi(keys[1]).is_none());
    assert!(std::ptr::eq(
        graph.phi(keys[2]).unwrap(),
        values.phi(keys[2]).unwrap()
    ));
    graph.exclude_phis(&[keys[0], keys[1]].into());
    assert_eq!(snapshot(&graph), [keys[2]]);
    assert_eq!(graph.phi(keys[2]).unwrap().inputs[0].value, keys[0]);
}

#[test]
fn source_instruction_definition_borrows_its_original_operation() {
    use super::ValueRef;
    use crate::ir::{analysis::SsaVar, RegisterArg};

    let mut result = RegisterArg::new_ssa(2, 3, ArgType::INT);
    result.code_var = Some(8);
    let mut instruction = InsnNode::new(InsnType::Const, 1);
    instruction.result = Some(result);
    instruction.args.push(InsnArg::lit(31, ArgType::INT));
    let block = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(4),
        statements: vec![SemanticStatement::instruction(instruction).unwrap()],
    });
    let SemanticNode::BasicBlock(original) = &block else {
        unreachable!()
    };
    let source = original.statements[0].instruction_ref().unwrap();
    let flow = ValueFlowGraph::build_source(&block, &BTreeSet::new()).unwrap();
    let [fact] = flow.definitions[&SsaVar::new(8, 0)].as_slice() else {
        panic!("single definition")
    };
    let ValueRef::Operation(value) = fact.expression() else {
        panic!("instruction result")
    };
    assert!(std::ptr::eq(value, source));
    assert_eq!(value.operands()[0].literal_value(), Some(31));
    assert_eq!(fact.owned_expression().literal_value(), Some(31));
}

#[test]
fn copy_origins_preserve_cycle_entries_and_retained_definitions() {
    use super::{SsaCopyFlow, SsaVar};
    let keys = [0, 1, 2, 3].map(|reg| SsaVar::new(reg, 1));
    // Every four-vertex functional graph, including a missing outgoing edge,
    // paired with every choice of omitted definitions.
    for encoded in 0..5usize.pow(4) {
        for mask in 0..16 {
            let mut copies = SsaCopyFlow::default();
            let mut remaining = encoded;
            for (index, &key) in keys.iter().enumerate() {
                if let Some(&parent) = keys.get(remaining % 5) {
                    copies.sources.insert(key, parent);
                    if mask & (1 << index) != 0 {
                        copies.omitted.insert(key);
                    }
                }
                remaining /= 5;
            }
            for &key in &keys {
                for filtered in [false, true] {
                    let mut seen = BTreeSet::new();
                    let mut cursor = key;
                    while (!filtered || copies.omitted.contains(&cursor)) && seen.insert(cursor) {
                        let Some(&next) = copies.sources.get(&cursor) else {
                            break;
                        };
                        cursor = next;
                    }
                    assert_eq!(copies.origin(key, filtered), cursor);
                }
            }
        }
    }
}
