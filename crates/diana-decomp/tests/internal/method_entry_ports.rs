use super::*;
use crate::ir::analysis::SsaValueGraph;
use crate::ir::{ExceptionAnalysis, InsnNode, RegionGraphBuilder};

fn consecutive_regions(cfg: &CFG) -> RegionGraph {
    let values = SsaValueGraph::build(cfg).unwrap();
    RegionGraphBuilder::new(cfg, &ExceptionAnalysis::default(), &values)
        .build()
        .unwrap()
}

#[test]
fn single_owner_edges_keep_only_the_method_entry_even_on_sparse_blocks() {
    let [entry, mid, exit] = [17, 35, 82].map(BlockId::new);
    let mut cfg = CFG::new("method_ports");
    cfg.entry = entry;
    for (id, target) in [(entry, Some(mid)), (mid, Some(exit)), (exit, None)] {
        let mut block = Block::with_offset(id, id.raw());
        block.push(target.map_or_else(InsnNode::return_void, |next| {
            InsnNode::goto(next.raw() as i32)
        }));
        cfg.add_block(block);
        if let Some(target) = target {
            cfg.add_edge(id, target, EdgeKind::Normal);
        }
    }
    let regions = consecutive_regions(&cfg);
    let anchors = RegionAnchors::analyze(&cfg, &regions, &ControlContractions::identity()).unwrap();
    assert!(RegionEntryPorts::confined_to_method(&cfg, &regions));
    let root = regions.tree().root();
    let ports = RegionEntryPorts::analyze(&cfg, &regions, &anchors).unwrap();
    assert_eq!(ports.of(root).unwrap(), &BTreeSet::from([entry]));

    // Removing a block from the public CFG leaves the separate region graph
    // intact. The missing physical block still owns its original region; the
    // ordinary traversal must follow its stored successors before reporting
    // unrelated unowned blocks.
    cfg.blocks.remove(&mid);
    let unowned = BlockId::new(1);
    cfg.add_block(Block::new(unowned));
    let missing = BlockId::new(120);
    cfg.add_edge(mid, missing, EdgeKind::True);
    assert!(!RegionEntryPorts::confined_to_method(&cfg, &regions));
    assert!(matches!(
        RegionEntryPorts::analyze(&cfg, &regions, &anchors),
        Err(StructureError::RegionOwnerMissing { region, block }) if region == root && block == missing
    ));
}

#[test]
fn exceptional_edges_to_foreign_ids_do_not_create_ordinary_entry_ports() {
    let mut cfg = CFG::new("exceptional_port");
    let mut block = Block::new(0_u32);
    block.push(InsnNode::return_void());
    cfg.add_block(block);
    let regions = consecutive_regions(&cfg);
    let anchors = RegionAnchors::analyze(&cfg, &regions, &ControlContractions::identity()).unwrap();
    let entry = cfg.entry;
    let foreign = BlockId::new(u32::MAX - 1);
    cfg.add_edge(entry, foreign, EdgeKind::Exception);
    assert!(RegionEntryPorts::confined_to_method(&cfg, &regions));
    let ports = RegionEntryPorts::analyze(&cfg, &regions, &anchors).unwrap();
    assert_eq!(
        ports.of(regions.tree().root()).unwrap(),
        &BTreeSet::from([entry])
    );
    cfg.add_edge(entry, foreign, EdgeKind::Normal);
    assert!(matches!(
        RegionEntryPorts::analyze(&cfg, &regions, &anchors),
        Err(StructureError::RegionOwnerMissing { block, .. }) if block == foreign
    ));
}
