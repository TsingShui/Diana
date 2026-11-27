use super::*;
use crate::ir::{Block, EdgeKind, RegionEdge};

#[test]
fn available_copies_distinguish_merge_backedge_throw_and_exit_sites() {
    let variable = SsaVar::new(2, 1);
    let variables = CodeVariables::from_singletons(&SsaValueGraph::default(), &[variable]);
    let id = variables.variable_of(variable).unwrap();
    let block = BlockId::new;
    let edge = |a, b, kind| RegionEdge {
        source: block(a),
        target: block(b),
        kind,
    };
    let mut cfg = CFG::new("available_copies");
    for i in 0..6 {
        let mut body = Block::new(i as u32);
        if i == 4 {
            body.push(InsnNode::new(InsnType::NewInstance, 0));
        }
        cfg.add_block(body);
    }
    for path in [
        edge(0, 1, EdgeKind::True),
        edge(0, 2, EdgeKind::False),
        edge(1, 3, EdgeKind::Normal),
        edge(2, 3, EdgeKind::Normal),
        edge(3, 4, EdgeKind::Normal),
        edge(4, 3, EdgeKind::True),
        edge(4, 5, EdgeKind::False),
        edge(0, 4, EdgeKind::Exception),
    ] {
        cfg.add_edge(path.source, path.target, path.kind);
    }
    let copies = |value| {
        vec![EdgeCopy {
            destination: RegisterArg::new_ssa(2, 1, ArgType::INT),
            source: InsnArg::lit(value, ArgType::INT),
        }]
    };
    let seven = CopySource::Literal(7, ArgType::INT);
    let exit = NormalCopySite::Edge(edge(4, 5, EdgeKind::False));
    for loop_value in [7, 8] {
        let normal = BTreeMap::from([
            (NormalCopySite::Block(block(1)), copies(7)),
            (NormalCopySite::Block(block(2)), copies(7)),
            (NormalCopySite::Block(block(3)), copies(7)),
            (NormalCopySite::Block(block(4)), copies(loop_value)),
            (exit, copies(7)),
        ]);
        let exceptional = BTreeMap::from([(block(4), CopyFacts::from([(id, seven.clone())]))]);
        let facts = AvailableCopyAnalysis::analyze(
            &cfg,
            &variables,
            &normal,
            &exceptional,
            &BTreeSet::new(),
        )
        .unwrap();
        // The first pass through the join sees seven on both arms. Later
        // visits also encounter the value stored on the backedge.
        assert_eq!(
            facts.contains_before_normal(block(3), id, &seven),
            loop_value == 7
        );
        // The join establishes seven before the allocation can throw.
        assert!(facts.contains(block(4), id, &seven));
        assert_eq!(
            facts.contains_before_site(exit, id, &seven),
            loop_value == 7
        );
    }
}

#[test]
fn an_unused_copy_query_preserves_materialized_register_errors() {
    let mut cfg = CFG::new("unused_copy_query");
    let mut body = Block::new(0u32);
    let result = RegisterArg::new_ssa(2, 1, ArgType::INT);
    let mut definition = InsnNode::new(InsnType::Const, 1);
    definition.result = Some(result);
    definition.args.push(InsnArg::lit(7, ArgType::INT));
    body.push(definition);
    cfg.add_block(body);
    let id = SsaVar::new(2, 1);
    let outcome = AvailableCopyAnalysis::analyze(
        &cfg,
        &CodeVariables::from_singletons(&SsaValueGraph::default(), &[]),
        &BTreeMap::new(),
        &BTreeMap::new(),
        &BTreeSet::from([id]),
    );
    assert!(matches!(outcome, Err(SourceVariableError::MissingCodeVariable(value)) if value == id));
}
