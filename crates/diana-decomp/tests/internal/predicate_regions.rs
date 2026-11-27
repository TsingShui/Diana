use super::*;
use crate::ir::{
    ArgType, InsnNode, InsnType, InstructionId, LiteralArg, RegionId, SemanticExpression,
    SemanticLeave, SemanticLeaveKind,
};

fn predicate(id: usize) -> SemanticPredicate {
    let mut instruction = InsnNode::new(InsnType::If, 0);
    instruction.id = InstructionId::new(id);
    SemanticPredicate::Test(crate::ir::SemanticOperation::from_instruction(instruction).unwrap())
}

fn leave(kind: SemanticLeaveKind) -> SemanticNode {
    let region = RegionId::new(0);
    SemanticNode::Leave(SemanticLeave {
        site: None,
        condition: None,
        kind,
        edge: None,
        origin: None,
        source: region,
        destination: region,
        target: region,
        cleanup: Vec::new(),
    })
}

fn block(id: u32) -> SemanticNode {
    SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(id),
        statements: vec![SemanticStatement::instruction(InsnNode::new(InsnType::Nop, 0)).unwrap()],
    })
}

#[test]
fn nested_alternatives_and_initial_guards_keep_their_blocks_in_place() {
    let mut root = block(17);
    let SemanticNode::BasicBlock(inner) = &root else {
        unreachable!()
    };
    let statements = inner.statements.as_ptr();
    for n in 1..50 {
        root = SemanticNode::If {
            condition: crate::ir::SemanticOperand::new(predicate(n as usize)),
            then_node: Box::new(root),
            else_node: Some(Box::new(block(n))),
        };
    }
    root = SemanticNode::sequence([SemanticNode::guard(predicate(89), root), block(99)]);
    assert!(!needs_predicate_regions(&root));
    assert!(!PredicateRegionFormation::apply(&mut root).unwrap());
    let SemanticNode::Sequence(nodes) = &root else {
        panic!("sequence")
    };
    let mut node = &nodes[0];
    while let SemanticNode::If { then_node, .. } = node {
        node = then_node;
    }
    let SemanticNode::BasicBlock(inner) = node else {
        panic!("innermost block")
    };
    assert_eq!(inner.statements.as_ptr(), statements);
    assert_eq!(inner.id, BlockId::new(17));
}

#[test]
fn isolated_branch_still_normalizes_empty_blocks_and_degenerate_sequences() {
    let empty = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(5),
        statements: Vec::new(),
    });
    let mut root = SemanticNode::If {
        condition: crate::ir::SemanticOperand::new(predicate(42)),
        then_node: Box::new(SemanticNode::Sequence(vec![
            empty,
            SemanticNode::Sequence(vec![block(6)]),
        ])),
        else_node: Some(Box::new(block(7))),
    };
    assert!(needs_predicate_regions(&root));
    assert!(!PredicateRegionFormation::apply(&mut root).unwrap());
    let SemanticNode::If {
        then_node,
        else_node: Some(otherwise),
        ..
    } = root
    else {
        panic!("branch")
    };
    assert!(matches!(*then_node, SemanticNode::BasicBlock(block) if block.id == BlockId::new(6)));
    assert!(matches!(*otherwise, SemanticNode::BasicBlock(block) if block.id == BlockId::new(7)));
}

#[test]
fn guarded_tail_inside_nested_branch_keeps_the_region_merge() {
    let mut root = SemanticNode::branch(
        predicate(10),
        SemanticNode::sequence([
            SemanticNode::guard(predicate(20), block(1)),
            SemanticNode::guard(predicate(20), block(2)),
        ]),
        Some(block(3)),
    );
    assert!(needs_predicate_regions(&root));
    assert!(PredicateRegionFormation::apply(&mut root).unwrap());
    let SemanticNode::If { then_node, .. } = root else {
        panic!("outer branch")
    };
    let SemanticNode::If {
        then_node,
        else_node: None,
        ..
    } = *then_node
    else {
        panic!("merged inner guard")
    };
    let SemanticNode::Sequence(stores) = *then_node else {
        panic!("both guarded blocks")
    };
    assert!(
        matches!(stores.as_slice(), [SemanticNode::BasicBlock(a), SemanticNode::BasicBlock(b)]
        if a.id == BlockId::new(1) && b.id == BlockId::new(2))
    );
}

#[test]
fn isolated_branches_keep_classified_operand_errors_at_the_original_site() {
    use super::super::flow::{RecoveryMode, ValueFlowGraph};
    use crate::ir::analysis::SourceEvaluationError;
    use std::collections::BTreeSet;

    let mut store = InsnNode::new(InsnType::Aput, 0);
    store.offset = 23;
    let mut root = SemanticNode::branch(
        predicate(10),
        SemanticNode::BasicBlock(SemanticBlock {
            id: BlockId::new(5),
            statements: vec![SemanticStatement::instruction(store).unwrap()],
        }),
        Some(block(7)),
    );
    assert!(!needs_predicate_regions(&root));
    assert!(!PredicateRegionFormation::apply(&mut root).unwrap());
    for mode in [RecoveryMode::Structural, RecoveryMode::Full] {
        assert!(matches!(
            ValueFlowGraph::plan_source(&root, &BTreeSet::new(), &Default::default(), mode),
            Err(ValueRecoveryError::SourceEvaluation(
                SourceEvaluationError::InvalidArity {
                    instruction: InsnType::Aput,
                    offset: 23,
                    actual: 0,
                }
            ))
        ));
    }
}

#[test]
fn rejects_equivalent_guard_merge_that_drops_an_abrupt_outcome() {
    let condition = predicate(1);
    let mut root = SemanticNode::sequence([
        SemanticNode::guard(
            condition.clone(),
            leave(SemanticLeaveKind::Throw(SemanticExpression::Literal(
                LiteralArg::new(0, ArgType::INT),
            ))),
        ),
        SemanticNode::guard(condition, leave(SemanticLeaveKind::Return(None))),
    ]);
    let before = SemanticCompletion::analyze(&root);

    let changed = PredicateRegionFormation::apply(&mut root).unwrap();

    assert!(!changed);
    assert_eq!(SemanticCompletion::analyze(&root), before);
    assert!(matches!(root, SemanticNode::Sequence(nodes) if nodes.len() == 2));
}

#[test]
fn skips_trees_without_if_or_self_selecting_select() {
    let mut root = SemanticNode::Empty;
    assert!(!PredicateRegionFormation::apply(&mut root).unwrap());
    assert!(matches!(root, SemanticNode::Empty));
}

#[test]
fn merging_guards_moves_the_original_site_and_both_children() {
    use crate::ir::{SemanticOperand, SemanticSiteId};

    let mut condition = SemanticOperand::new(predicate(8));
    condition.site = Some(SemanticSiteId(18));
    let node = |id| {
        SemanticNode::BasicBlock(SemanticBlock {
            id: BlockId::new(id),
            statements: Vec::new(),
        })
    };
    let mut queries = PredicateQueries::default();
    let mut formation = PredicateRegionFormation {
        changed: false,
        queries: &mut queries,
    };
    let merged = formation
        .form_sequence(vec![
            SemanticNode::If {
                condition,
                then_node: Box::new(SemanticNode::sequence([node(1), node(5)])),
                else_node: None,
            },
            SemanticNode::guard(predicate(9), leave(SemanticLeaveKind::Return(None))),
        ])
        .unwrap();
    let SemanticNode::If {
        condition,
        then_node,
        else_node: None,
    } = merged
    else {
        panic!("equivalent guards should merge");
    };
    assert!(formation.changed);
    assert_eq!(condition.site, Some(SemanticSiteId(18)));
    let SemanticNode::Sequence(children) = *then_node else {
        panic!("guarded children")
    };
    assert!(matches!(children.as_slice(), [
        SemanticNode::BasicBlock(first), SemanticNode::BasicBlock(second), SemanticNode::Leave(exit)
    ] if first.id == BlockId::new(1) && second.id == BlockId::new(5) && matches!(exit.kind, SemanticLeaveKind::Return(None))));
}

#[test]
fn selected_store_arms_keep_the_definition_origin_and_drop_the_ssa_version() {
    use crate::ir::{RegisterArg, SemanticSiteId, StatementOrigin};

    let mut variable = RegisterArg::new_ssa(3, 7, ArgType::INT);
    variable.code_var = Some(12);
    let origin = StatementOrigin {
        block: BlockId::new(4),
        instruction: InstructionId::new(16),
    };
    let literal = |value| SemanticExpression::Literal(LiteralArg::new(value, ArgType::INT));
    let value = SemanticExpression::select(
        predicate(10),
        SemanticExpression::select(
            predicate(11),
            literal(5),
            SemanticExpression::Register(variable.clone()),
        ),
        literal(6),
    );
    let mut statement = SemanticStatement::definition(origin.instruction, variable, value);
    statement.site = Some(SemanticSiteId(21));
    statement.origin = Some(origin.clone());
    let recovered =
        GuardedAssignment::recover(statement, BlockId::new(9)).expect("self-selecting value");
    let SemanticNode::If {
        then_node,
        else_node: Some(otherwise),
        ..
    } = recovered
    else {
        panic!("outer choice")
    };
    let SemanticNode::If {
        then_node: first,
        else_node: None,
        ..
    } = *then_node
    else {
        panic!("inner guard")
    };
    for (block, expected) in [(*first, 5), (*otherwise, 6)] {
        let SemanticNode::BasicBlock(block) = block else {
            panic!("store arm")
        };
        assert_eq!(block.id, BlockId::new(9));
        let [statement] = block.statements.as_slice() else {
            panic!("single store")
        };
        assert_eq!(statement.origin.as_ref(), Some(&origin));
        assert!(statement.site.is_none());
        let result = statement.result().unwrap();
        assert_eq!(result.code_var, Some(12));
        assert_eq!(result.ssa_version, None);
        assert!(
            matches!(statement.value(), Some(SemanticExpression::Literal(value)) if value.value == expected)
        );
    }
}

#[test]
fn distributing_a_proven_tail_preserves_statement_order_and_the_other_exit() {
    use crate::ir::SemanticOperand;

    let first = predicate(17);
    let guard = SemanticOperand::new(SemanticPredicate::And(vec![first.clone(), first.clone()]));
    let block = |id| {
        let statement = SemanticStatement::instruction(InsnNode::new(InsnType::Nop, 0)).unwrap();
        SemanticNode::BasicBlock(SemanticBlock {
            id: BlockId::new(id),
            statements: vec![statement],
        })
    };
    let prefix = SemanticNode::branch(
        first,
        block(2),
        Some(leave(SemanticLeaveKind::Return(None))),
    );
    let tail = block(3);
    let distributed = GuardDistribution::apply(&prefix, &guard, &tail)
        .unwrap()
        .expect("covered tail");
    let SemanticNode::If {
        then_node,
        else_node: Some(otherwise),
        ..
    } = distributed
    else {
        panic!("both paths remain")
    };
    let SemanticNode::Sequence(children) = *then_node else {
        panic!("distributed tail")
    };
    assert!(matches!(children.as_slice(),
        [SemanticNode::BasicBlock(one), SemanticNode::BasicBlock(two)]
        if one.id == BlockId::new(2) && two.id == BlockId::new(3)
    ));
    assert!(
        matches!(*otherwise, SemanticNode::Leave(exit) if matches!(exit.kind, SemanticLeaveKind::Return(None)))
    );
}

#[test]
fn named_predicate_domains_preserve_short_circuit_constants_and_atom_aliases() {
    use crate::ir::BoolExpr;
    let syntax = SemanticPredicate::And(vec![
        SemanticPredicate::Or(vec![SemanticPredicate::False, predicate(8)]),
        SemanticPredicate::Or(vec![predicate(9), predicate(7).negate()]),
        SemanticPredicate::Or(vec![predicate(44), SemanticPredicate::True]),
    ]);
    let atom = |v| BoolExpr::Symbol(BoolVariable::Atom(v));
    assert_eq!(
        syntax
            .domain_with(|id| Some(BoolVariable::Atom((id.raw() % 2) as u32)))
            .unwrap(),
        BoolExpr::and(vec![
            atom(0),
            BoolExpr::or(vec![atom(1), BoolExpr::not(atom(1))])
        ]),
    );
    let equivalent = predicate(2);
    let other = predicate(8);
    for (left, right, expected) in [
        (equivalent.clone(), other.clone(), true),
        (equivalent.clone().negate(), other.clone(), false),
        (equivalent.negate(), other.negate(), true),
    ] {
        assert_eq!(
            PredicateQueries::default()
                .equivalent(&left, &right)
                .unwrap(),
            expected
        );
    }
}

#[test]
fn indexed_atom_classes_preserve_first_match_across_register_and_instruction_aliases() {
    use crate::ir::{InsnArg, RegisterArg, SemanticOperation};

    fn operation(id: usize, register: u32, version: u32, variable: u32) -> SemanticOperation {
        let mut node = InsnNode::new(InsnType::If, 1);
        node.id = InstructionId::new(id);
        let mut input = RegisterArg::new_ssa(register, version, ArgType::INT);
        input.code_var = Some(variable);
        node.args.push(InsnArg::Reg(input));
        SemanticOperation::from_instruction(node).unwrap()
    }

    let mut calls: Vec<_> = (0..16)
        .map(|n| operation(n, n as u32, 1, n as u32))
        .collect();
    calls.push(operation(12, 5, 1, 11));
    calls.push(operation(3, 14, 1, 18));
    calls.push(operation(33, 14, 1, 18));
    let mut source = AtomClasses::new(AtomIdentity::Source);
    let mut distribution = AtomClasses::new(AtomIdentity::Distribution);
    for (index, call) in calls[..16].iter().enumerate() {
        assert_eq!(source.intern(call), index);
        assert_eq!(distribution.intern(call), index);
    }
    // The older structural representative precedes the operation's ID.
    assert_eq!(source.intern(&calls[16]), 11);
    assert_eq!(distribution.intern(&calls[16]), 5);
    // Here the same ID precedes the matching SSA representative. Only
    // distribution accepts that ID, so the numeric namespaces diverge.
    assert_eq!(source.intern(&calls[17]), 16);
    assert_eq!(distribution.intern(&calls[17]), 3);
    assert_eq!(source.intern(&calls[18]), 16);
    assert_eq!(distribution.intern(&calls[18]), 14);
}

#[test]
fn numeric_predicate_order_matches_decimal_names_at_each_prefix_boundary() {
    for count in [0, 1, 9, 10, 11, 20, 99, 100, 101, 109, 110, 201, 1001] {
        let ranks = PredicateEquivalence::decimal_ranks(count);
        let mut names: Vec<_> = (0..count).collect();
        names.sort_by_cached_key(usize::to_string);
        for (rank, name) in names.into_iter().enumerate() {
            assert_eq!(ranks[name], rank, "domain of {count} atoms");
        }
    }
}

#[test]
fn predicate_proofs_reuse_the_boolean_relation_after_operands_change() {
    fn atom(id: usize, literal: i64) -> SemanticPredicate {
        let mut node = InsnNode::new(InsnType::If, 1);
        node.id = InstructionId::new(id);
        node.args
            .push(crate::ir::InsnArg::lit(literal, ArgType::INT));
        SemanticPredicate::Test(crate::ir::SemanticOperation::from_instruction(node).unwrap())
    }

    let mut queries = PredicateQueries::default();
    let pair = |ids: [usize; 3], literals: [i64; 3]| {
        let a = atom(ids[0], literals[0]);
        let b = atom(ids[1], literals[1]);
        let c = atom(ids[2], literals[2]);
        (
            SemanticPredicate::And(vec![a.clone(), SemanticPredicate::Or(vec![b, c])]),
            a,
        )
    };
    for (ids, literals, answer) in [
        ([10, 11, 12], [1, 1, 2], true),
        ([10, 11, 12], [1, 3, 2], false),
        ([50, 51, 52], [9, 9, 6], true),
    ] {
        let (left, right) = pair(ids, literals);
        assert_eq!(queries.equivalent(&left, &right).unwrap(), answer);
    }
    assert_eq!(queries.answers.len(), 2);
}

#[test]
fn equivalent_large_guards_share_the_same_bounded_proof_refusal() {
    fn equality(bits: usize) -> SemanticPredicate {
        let variables: Vec<_> = (0..2 * bits)
            .map(|n| {
                let mut instruction = InsnNode::new(InsnType::If, 1);
                instruction.id = InstructionId::new(n);
                instruction
                    .args
                    .push(crate::ir::InsnArg::lit(n as i64, ArgType::INT));
                SemanticPredicate::Test(
                    crate::ir::SemanticOperation::from_instruction(instruction).unwrap(),
                )
            })
            .collect();
        // Establish an order that puts all left bits before all right bits.
        // The constant disjunction is neutral to the actual equality test.
        let mut indexes: Vec<_> = (0..variables.len()).collect();
        indexes.sort_by_cached_key(usize::to_string);
        let mut seeds = vec![SemanticPredicate::False; variables.len()];
        for (rank, discovery) in indexes.into_iter().enumerate() {
            seeds[variables.len() - 1 - discovery] = variables[rank].clone();
        }
        seeds.push(SemanticPredicate::True);
        let mut pairs = (0..bits)
            .map(|n| {
                let x = variables[n].clone();
                let y = variables[bits + n].clone();
                SemanticPredicate::Or(vec![
                    SemanticPredicate::And(vec![x.clone(), y.clone()]),
                    SemanticPredicate::And(vec![x.negate(), y.negate()]),
                ])
            })
            .collect::<Vec<_>>();
        pairs.push(SemanticPredicate::Or(seeds));
        SemanticPredicate::And(pairs)
    }
    let small = equality(3);
    let large = equality(19);
    let fresh = PredicateEquivalence::new(&large, &large);
    let error = fresh.equivalent(&large, &large).unwrap().unwrap_err();
    assert!(error.is_resource_limit());
    let mut queries = PredicateQueries::default();
    for _ in 0..2 {
        assert!(queries.equivalent(&small, &small).unwrap());
        assert!(!queries.equivalent(&large, &large).unwrap());
    }
    assert_eq!(queries.answers.len(), 2);
}
