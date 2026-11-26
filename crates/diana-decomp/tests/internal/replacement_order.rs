use super::*;
use crate::ir::{analysis::SourceEvaluationError, ArgType, InsnNode};

fn key(n: usize) -> SsaVar {
    SsaVar::new(n as u32 * 3 + 1, 4)
}

fn expression(inputs: impl IntoIterator<Item = usize>) -> SemanticExpression {
    let args = inputs
        .into_iter()
        .map(|n| {
            let k = key(n);
            SemanticExpression::Register(RegisterArg::new_ssa(k.reg_num, k.version, ArgType::INT))
        })
        .collect::<Vec<_>>();
    SemanticExpression::Operation(SemanticOperation::from_parts(
        InsnNode::new(InsnType::FilledNewArray, args.len()),
        args,
        None,
    ))
}

fn check_order(inputs: Vec<Vec<usize>>) {
    let count = inputs.len();
    let raw = inputs
        .iter()
        .enumerate()
        .map(|(n, deps)| (key(n), expression(deps.iter().copied())))
        .collect();
    let graph = ReplacementGraph::new(&raw, ValueIdentity::Ssa).unwrap();
    // Exhaust individual paths independently to identify only nodes that
    // can reach themselves. Keep blocked consumers of those nodes.
    let cyclic = (0..count)
        .map(|n| {
            let mut pending = inputs[n].clone();
            let mut seen = BTreeSet::new();
            while let Some(next) = pending.pop() {
                if next == n {
                    return true;
                }
                if next < count && seen.insert(next) {
                    pending.extend(&inputs[next]);
                }
            }
            false
        })
        .collect::<Vec<_>>();
    let mut handled = cyclic;
    let mut expected = Vec::new();
    while let Some(n) = (0..count)
        .find(|&n| !handled[n] && inputs[n].iter().all(|&dep| dep >= count || handled[dep]))
    {
        handled[n] = true;
        expected.push(key(n));
    }
    assert!(handled.into_iter().all(|done| done));
    assert_eq!(graph.into_order(), expected);
}

#[test]
fn replacement_order_matches_all_small_graphs_and_overlapping_larger_cycles() {
    for bits in 0..(1 << 9) {
        check_order(
            (0..3)
                .map(|n| (0..3).filter(|m| bits & (1 << (3 * n + m)) != 0).collect())
                .collect(),
        );
    }
    check_order(vec![
        vec![6, 6],
        vec![2],
        vec![1, 3],
        vec![4, 10],
        vec![2],
        vec![6, 3],
        vec![6],
        vec![8],
        vec![],
        vec![1, 5, 7, 100],
        vec![11],
        vec![10],
    ]);
}

#[test]
fn long_ascending_replacements_have_a_single_sorted_dependency_product() {
    let size = 512;
    let raw = (0..size)
        .map(|n| {
            let deps = if n == 0 {
                vec![size]
            } else {
                vec![n - 1, n - 1, size]
            };
            (key(n), expression(deps))
        })
        .collect::<BTreeMap<_, _>>();
    let graph = ReplacementGraph::new(&raw, ValueIdentity::Ssa).unwrap();
    assert!(graph.ordered);
    assert_eq!(graph.dependencies.targets.len(), size - 1);
    assert_eq!(graph.into_order(), raw.keys().copied().collect::<Vec<_>>());
}

#[test]
fn malformed_original_and_site_expressions_keep_their_error_order() {
    let bad = |offset| {
        let mut op = InsnNode::new(InsnType::Aput, 0);
        op.offset = offset;
        SemanticExpression::Operation(SemanticOperation::from_parts(op, vec![], None))
    };
    let site = UseSite::Argument(crate::ir::SemanticSiteId(0));
    let local = ValueAction::InlineAt {
        event: 0,
        key: key(0),
        remove_key: false,
        remove_definitions: vec![],
        replacements: vec![(site, super::super::flow::ValueExpression { value: bad(31) })],
    };
    let global = |key, offset| ValueAction::Inline {
        event: 0,
        key,
        expression: bad(offset),
    };
    for (actions, expected) in [
        (
            vec![local.clone(), global(key(8), 93), global(key(4), 17)],
            17,
        ),
        (vec![local], 31),
    ] {
        assert!(
            matches!(ValueSchedule::compile(actions, ValueIdentity::Ssa),
            Err(ValueRecoveryError::SourceEvaluation(SourceEvaluationError::InvalidArity { offset, instruction: InsnType::Aput, .. })) if offset == expected)
        );
    }
}

#[test]
fn global_substitution_shares_untouched_children_and_updates_targets_and_results() {
    let reg = |n| RegisterArg::new_ssa(key(n).reg_num, key(n).version, ArgType::INT);
    let lit = |value| SemanticExpression::Literal(crate::ir::LiteralArg::int(value));
    let mut raw = InsnNode::new(InsnType::FilledNewArray, 1);
    raw.result = Some(reg(4));
    let unaffected = SemanticOperation::from_parts(
        raw.clone(),
        vec![SemanticExpression::Register(reg(1))],
        None,
    );
    let compound = SemanticOperation::from_parts(
        raw,
        vec![SemanticExpression::Register(reg(1))],
        Some(SemanticExpression::Register(reg(2))),
    );
    let mut discarded = InsnNode::new(InsnType::Invoke, 0);
    discarded.result = Some(reg(3));
    let discarded = SemanticOperation::from_parts(discarded, vec![], None);
    let mut tree = SemanticExpression::Operation(SemanticOperation::from_parts(
        InsnNode::new(InsnType::FilledNewArray, 3),
        vec![
            SemanticExpression::Operation(unaffected.clone()),
            SemanticExpression::Operation(compound.clone()),
            SemanticExpression::Operation(discarded.clone()),
        ],
        None,
    ));
    let replacements = BTreeMap::from([(key(2), lit(17))]);
    let discarded_results = BTreeSet::from([key(3)]);
    let mut substitution = ValueSubstitution {
        replacements: &replacements,
        discarded_results: &discarded_results,
        identity: ValueIdentity::Ssa,
        changed: false,
    };
    let mut alone = SemanticExpression::Operation(unaffected.clone());
    SemanticInstructions::transform_expression(&mut alone, &mut substitution).unwrap();
    assert!(!substitution.changed);
    SemanticInstructions::transform_expression(&mut tree, &mut substitution).unwrap();
    assert!(substitution.changed);
    let SemanticExpression::Operation(tree) = tree else {
        panic!("expected outer array")
    };
    let [SemanticExpression::Operation(first), SemanticExpression::Operation(second), SemanticExpression::Operation(third)] =
        tree.operands()
    else {
        panic!("expected three original operations")
    };
    assert!(std::ptr::eq(&**first, &*unaffected));
    assert!(
        matches!(second.compound_target(), Some(SemanticExpression::Literal(value)) if *value == crate::ir::LiteralArg::int(17))
    );
    assert_eq!(second.result, Some(reg(4)));
    assert!(
        matches!(compound.compound_target(), Some(SemanticExpression::Register(value)) if *value == reg(2))
    );
    assert_eq!(third.result, None);
    assert_eq!(discarded.result, Some(reg(3)));
}
