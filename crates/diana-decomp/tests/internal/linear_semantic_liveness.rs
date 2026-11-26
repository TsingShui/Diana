use super::*;
use crate::ir::{ArgType, SemanticBlock, SemanticSiteId};
use std::sync::Arc;

#[test]
fn a_read_at_its_own_definition_crosses_only_the_earlier_live_range() {
    let tracked = SsaVar::new(3, 1);
    let crossing = SsaVar::new(5, 1);
    let gap = SsaVar::new(7, 1);
    let read = SsaVar::new(9, 1);
    let statements = [
        (tracked, None),
        (crossing, None),
        (tracked, Some(tracked)),
        (read, Some(tracked)),
        (gap, None),
        (tracked, None),
        (read, Some(tracked)),
    ]
    .into_iter()
    .enumerate()
    .map(|(site, (destination, source))| {
        let mut insn = InsnNode::new(
            if source.is_some() {
                InsnType::Move
            } else {
                InsnType::Const
            },
            1,
        );
        insn.result = Some(RegisterArg::new_ssa(
            destination.reg_num,
            destination.version,
            ArgType::INT,
        ));
        insn.args.push(source.map_or_else(
            || InsnArg::lit(1, ArgType::INT),
            |source| InsnArg::reg_ssa(source.reg_num, source.version, ArgType::INT),
        ));
        let mut statement = SemanticStatement::instruction(insn).unwrap();
        statement.site = Some(SemanticSiteId(site as u64));
        statement
    })
    .collect();
    let root = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements,
    });
    let flow = Arc::new(SemanticFlowGraph::analyze(&root));
    assert!(flow.reverse_linear_points().is_some());
    let mut graph = InterferenceGraph::default();
    SemanticLiveness::analyze(
        &root,
        &BTreeSet::from([tracked, crossing, gap, read]),
        &mut graph,
        || flow,
    )
    .unwrap();
    let edges = graph
        .semantic_edges
        .into_iter()
        .flat_map(|(left, adjacent)| adjacent.into_iter().map(move |right| (left, right)))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        edges,
        BTreeSet::from([(tracked, crossing), (crossing, tracked)])
    );
}

#[test]
fn singleton_ids_include_unused_inputs_in_register_order() {
    use crate::ir::analysis::{CodeVariables, SsaValueGraph};
    use crate::ir::Block;
    let mut cfg = CFG::new("singleton_ids");
    let mut block = Block::new(0u32);
    for register in [17, 2] {
        let mut definition = InsnNode::new(InsnType::Const, 1);
        definition.result = Some(RegisterArg::new_ssa(register, 0, ArgType::INT));
        definition.args.push(InsnArg::lit(1, ArgType::INT));
        block.push(definition);
    }
    cfg.add_block(block);
    let values = SsaValueGraph::build(&cfg).unwrap();
    let inputs = [1, 10, 17, 21].map(|reg| SsaVar::new(reg, 0));
    let classes = CodeVariables::from_singletons(&values, &inputs);
    assert_eq!(classes.next_id(), 5);
    for (expected, register) in [1, 2, 10, 17, 21].into_iter().enumerate() {
        assert_eq!(
            classes.variable_of(SsaVar::new(register, 0)),
            Some(expected as u32)
        );
    }
}

#[test]
fn checking_singletons_still_requires_every_numbered_flow_point() {
    let mut stmt = SemanticStatement::instruction(InsnNode::new(InsnType::Nop, 0)).unwrap();
    stmt.site = Some(SemanticSiteId(217));
    let other = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements: vec![stmt],
    });
    let outcome = SemanticLiveness::analyze(
        &SemanticNode::Empty,
        &BTreeSet::new(),
        &mut InterferenceGraph::for_groups(Some(Groups::default())),
        || Arc::new(SemanticFlowGraph::analyze(&other)),
    );
    assert!(matches!(
        outcome,
        Err(SourceVariableError::MissingSemanticSite("flow point"))
    ));
}

#[test]
fn projected_conflicts_follow_transitive_candidates_after_any_prior_unions() {
    use super::super::{CoalescingCandidates, CoalescingPairs, CoalescingPolicy};
    let names = std::array::from_fn::<_, 12, _>(|i| SsaVar::new(i as u32, 1));
    // Two disjoint components, a self proposal, and four unused names.
    let proposals = [(5, 4), (1, 2), (0, 3), (3, 1), (6, 5), (7, 7)];
    let pairs: CoalescingPairs = proposals
        .iter()
        .map(|&(i, j)| (names[i], names[j], CoalescingPolicy::AllInterference))
        .collect();
    let groups = CoalescingCandidates(pairs).groups();
    assert_eq!(groups.len(), 7);
    for (i, value) in names.iter().enumerate().take(7) {
        assert_eq!(groups[value], names[if i < 4 { 0 } else { 4 }]);
    }

    for mask in 0..64 {
        let compare = |groups| {
            let mut graph = InterferenceGraph::for_groups(groups);
            InterferenceGraph::add_clique(
                graph.groups.as_ref(),
                &mut graph.hard_edges,
                [0, 1, 4, 5, 7, 8, 9, 10, 11].map(|i| names[i]).into_iter(),
            );
            for i in 0..names.len() {
                graph.add_semantic(names[i], names[(i + 2) % names.len()]);
            }
            let mut classes = SsaClasses::new(names);
            let mut graph = ClassInterference::from_singletons(graph, &mut classes);
            let mut checks = Vec::new();
            for (step, &(i, j)) in proposals.iter().enumerate() {
                let left = classes.root(names[i]);
                let right = classes.root(names[j]);
                checks.push((
                    graph.interferes(left, right),
                    graph.hard_interferes(left, right),
                ));
                if left != right && mask & (1 << step) != 0 {
                    classes.union(left, right);
                    graph.merge(left.min(right), left.max(right));
                }
            }
            checks
        };
        assert_eq!(compare(None), compare(Some(groups.clone())));
    }
}
