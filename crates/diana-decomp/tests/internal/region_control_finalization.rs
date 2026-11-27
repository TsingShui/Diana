use crate::ir::{
    analysis::SsaValueGraph, Block, BlockId, EdgeKind, ExceptionAnalysis, InsnNode, InsnType,
    RegionGraph, RegionGraphBuilder, RegionId, RegionInvariantError, RegionKind, CFG,
};
use std::collections::BTreeSet;

fn graph(blocks: &[u32], edges: &[(u32, u32, EdgeKind)]) -> CFG {
    let mut cfg = CFG::new("control_finalization");
    for &block in blocks {
        cfg.add_block(Block::new(block));
    }
    for &(source, target, kind) in edges {
        cfg.add_edge(BlockId::new(source), BlockId::new(target), kind);
    }
    cfg
}

fn build(cfg: &CFG) -> Result<RegionGraph, RegionInvariantError> {
    let values = SsaValueGraph::build(cfg).unwrap();
    RegionGraphBuilder::new(cfg, &ExceptionAnalysis::default(), &values).build()
}

#[test]
fn sparse_nested_control_domains_keep_their_final_identities() {
    let mut cfg = graph(
        &[0, 10, 20, 30, 40, 50, 60, 70, 80],
        &[
            (0, 10, EdgeKind::Normal),
            (10, 80, EdgeKind::True),
            (10, 20, EdgeKind::False),
            (20, 30, EdgeKind::SwitchCase(0)),
            (20, 40, EdgeKind::SwitchCase(1)),
            (20, 70, EdgeKind::SwitchDefault),
            (30, 70, EdgeKind::Normal),
            (40, 50, EdgeKind::True),
            (40, 60, EdgeKind::False),
            (60, 70, EdgeKind::Normal),
            (70, 10, EdgeKind::Normal),
        ],
    );
    cfg.block_mut(BlockId::new(20))
        .unwrap()
        .push(InsnNode::new(InsnType::Switch, 20));
    for terminal in [50, 80] {
        cfg.block_mut(BlockId::new(terminal))
            .unwrap()
            .push(InsnNode::return_void());
    }

    let graph = build(&cfg).unwrap();
    let tree = graph.tree();
    assert_eq!(
        tree.regions()
            .map(|region| region.id.raw())
            .collect::<Vec<_>>(),
        [0, 3, 4]
    );
    let outer = tree.region(RegionId::new(3)).unwrap();
    assert_eq!(outer.parent, Some(tree.root()));
    assert_eq!(outer.children, [RegionId::new(4)]);
    assert!(matches!(&outer.kind, RegionKind::Loop(region)
        if region.follow == Some(BlockId::new(80)) && region.latches == BTreeSet::from([BlockId::new(70)])));
    let inner = tree.region(RegionId::new(4)).unwrap();
    assert_eq!(inner.parent, Some(outer.id));
    assert!(
        matches!(&inner.kind, RegionKind::Switch(region) if region.follow == Some(BlockId::new(70)))
    );
    assert_eq!(graph.owner_of(BlockId::new(50)), Some(inner.id));
    assert_eq!(graph.owner_of(BlockId::new(70)), Some(outer.id));

    // Provisional control allocation is visible through the next public ID.
    let mut extended = tree.clone();
    assert_eq!(
        extended
            .add_child(tree.root(), RegionKind::Try, None)
            .unwrap(),
        RegionId::new(5)
    );
}

#[test]
fn exceptional_only_latch_without_handler_scopes_needs_the_final_placement() {
    let mut cfg = graph(
        &[0, 7, 15, 31],
        &[
            (0, 7, EdgeKind::Normal),
            (7, 31, EdgeKind::Normal),
            (7, 15, EdgeKind::Exception),
            (15, 7, EdgeKind::Normal),
        ],
    );
    cfg.block_mut(BlockId::new(31))
        .unwrap()
        .push(InsnNode::return_void());
    let graph = build(&cfg).unwrap();
    let tree = graph.tree();
    let only_loop = tree.region(RegionId::new(1)).unwrap();
    assert_eq!(only_loop.parent, Some(tree.root()));
    assert_eq!(
        only_loop.blocks,
        [7, 15].into_iter().map(BlockId::new).collect()
    );
    assert!(
        matches!(&only_loop.kind, RegionKind::Loop(region) if region.latches == BTreeSet::from([BlockId::new(15)]))
    );
}

#[test]
fn malformed_switch_and_absent_edge_targets_keep_their_first_error() {
    let mut cfg = graph(&[0, 5], &[(0, 5, EdgeKind::Normal)]);
    let terminal = BlockId::new(5);
    cfg.block_mut(terminal)
        .unwrap()
        .push(InsnNode::return_void());
    let exceptions = ExceptionAnalysis::default();
    let values = SsaValueGraph::build(&cfg).unwrap();
    let mut missing_source = cfg.clone();
    missing_source.add_edge(BlockId::new(45), cfg.entry, EdgeKind::Normal);
    assert!(matches!(
        RegionGraphBuilder::new(&missing_source, &exceptions, &values).build(),
        Err(RegionInvariantError::MissingOwner(block)) if block == BlockId::new(45)
    ));

    for missing in [37, 22] {
        cfg.add_edge(cfg.entry, BlockId::new(missing), EdgeKind::Normal);
    }
    assert!(
        matches!(RegionGraphBuilder::new(&cfg, &exceptions, &values).build(),
        Err(RegionInvariantError::MissingOwner(block)) if block == BlockId::new(37))
    );

    cfg.block_mut(terminal).unwrap().insns.clear();
    cfg.block_mut(terminal)
        .unwrap()
        .push(InsnNode::new(InsnType::Switch, 5));
    assert!(
        matches!(RegionGraphBuilder::new(&cfg, &exceptions, &values).build(),
        Err(RegionInvariantError::MalformedSwitch { block, successors }) if block == terminal && successors.is_empty())
    );
}
