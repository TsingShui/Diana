use super::*;
use crate::ir::analysis::ClassHierarchyIndex;
use crate::ir::{Block, EdgeKind, MethodContext, MethodDescriptor, RegisterArg};

#[test]
fn parameter_cast_evidence_uses_only_normally_reachable_blocks() {
    let owner = ArgType::object("fixture/Observer");
    let object = ArgType::object("java/lang/Object");
    let descriptor = MethodDescriptor {
        parameters: vec![object.clone()],
        return_type: ArgType::VOID,
    };
    let reference = MethodReference {
        owner: owner.clone(),
        name: "observe".to_owned(),
        descriptor: descriptor.clone(),
    };
    let mut cfg = CFG::with_method(MethodContext::new(
        owner.clone(),
        "observe",
        descriptor,
        true,
    ));
    cfg.registers = 1;
    cfg.ins = 1;
    cfg.entry = BlockId::new(0);
    cfg.add_block(Block::new(cfg.entry));
    let normal = ArgType::object("fixture/Normal");
    for (id, ty, edge) in [
        (1, normal.clone(), EdgeKind::Normal),
        (2, ArgType::object("fixture/Exception"), EdgeKind::Exception),
    ] {
        let mut block = Block::new(BlockId::new(id));
        let mut cast = InsnNode::check_cast(InsnArg::Reg(RegisterArg::new(0, object.clone())), 0);
        cast.payload.class_type = Some(Box::new(ty));
        block.insns.push(cast);
        cfg.add_block(block);
        cfg.add_edge(cfg.entry, BlockId::new(id), edge);
    }
    let hierarchy = ClassHierarchyIndex::default();
    let mut signatures = SourceSignatureInference {
        hierarchy: &hierarchy,
        parameters: BTreeMap::new(),
        body_parameters: BTreeMap::new(),
        returns: BTreeMap::new(),
    };
    signatures.cfg(&cfg, &BTreeSet::from([&owner]));
    assert_eq!(signatures.body_parameter_types(&reference), [Some(normal)]);
    assert!(signatures.return_type(&reference).is_none());
}
