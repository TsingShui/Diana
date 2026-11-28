use super::*;
use crate::ir::{ArgType, EdgeKind, InsnNode, RegisterArg};

#[test]
fn relocating_a_catch_adapter_releases_its_stale_try_ownership() {
    assert_adapter_placement(false);
}

#[test]
fn an_ordinary_ingress_prevents_release_of_a_protected_block() {
    assert_adapter_placement(true);
}

fn assert_adapter_placement(ordinary_ingress: bool) {
    let start = BlockId(0);
    let protected = BlockId(1);
    let adapter = BlockId(2);
    let receiver = BlockId(3);
    let done = BlockId(4);
    let mut cfg = CFG::new("handler_adapter_ownership");
    for block in [start, protected, adapter, receiver, done] {
        cfg.add_block(Block::new(block.raw()));
    }
    let exception = RegisterArg::new_ssa(0, 0, ArgType::throwable());
    cfg.block_mut(adapter)
        .unwrap()
        .push(InsnNode::move_exception(exception.clone()));
    cfg.add_edge(start, protected, EdgeKind::Normal);
    cfg.add_edge(protected, adapter, EdgeKind::Exception);
    cfg.add_edge(adapter, receiver, EdgeKind::Normal);
    cfg.add_edge(protected, done, EdgeKind::Normal);
    if ordinary_ingress {
        cfg.add_edge(start, adapter, EdgeKind::Normal);
    }
    let facts = ControlFlowFacts::analyze(&cfg).expect("flow facts");
    let analysis = ExceptionAnalysis {
        handler_adapters: BTreeMap::from([(adapter, receiver)]),
        ..ExceptionAnalysis::default()
    };
    let mut tree = RegionTree::new(Some(start));
    let root = tree.root();
    let outer = tree
        .add_child(root, RegionKind::Try, Some(protected))
        .unwrap();
    let handler = tree
        .add_child(
            root,
            RegionKind::Cleanup(CatchRegion {
                exception_types: vec![ArgType::throwable()],
                exception_value: Some(exception),
                continuation: None,
            }),
            Some(receiver),
        )
        .unwrap();
    let fragment = tree
        .add_child(outer, RegionKind::Try, Some(adapter))
        .unwrap();
    for (scope, block) in [
        (outer, protected),
        (outer, adapter),
        (fragment, adapter),
        (handler, receiver),
    ] {
        tree.add_block(scope, block).unwrap();
    }
    let representatives = BTreeMap::new();
    let mut builder = ExceptionRegionTreeBuilder::new(
        &analysis,
        &cfg,
        &facts,
        &cfg,
        &facts,
        &representatives,
        tree,
    );
    builder.handlers.insert(outer, vec![handler]);
    builder
        .handler_domains
        .insert(handler, BTreeSet::from([adapter, receiver]));
    builder
        .nest_regions_in_handlers()
        .expect("handler placement");

    assert_eq!(builder.tree.region(fragment).unwrap().parent, Some(handler));
    assert_eq!(
        builder
            .tree
            .region(outer)
            .unwrap()
            .blocks
            .contains(&adapter),
        ordinary_ingress
    );
    assert_eq!(
        cfg.successors_with_kind(protected)
            .iter()
            .find(|(_, kind)| kind.is_exception()),
        Some(&(adapter, EdgeKind::Exception))
    );
    if !ordinary_ingress {
        builder
            .tree
            .canonicalize_nesting()
            .expect("laminar placement");
        builder.tree.cover_method(&cfg).unwrap();
        builder.tree.close_ancestor_ownership().unwrap();
        assert_eq!(builder.tree.region(outer).unwrap().parent, Some(root));
        assert_eq!(builder.tree.region(handler).unwrap().parent, Some(root));
        let owners = builder.tree.verified_owners(&cfg).expect("ownership proof");
        assert_eq!(owners.get(adapter), Some(fragment));
    }
}
