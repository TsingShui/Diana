use super::*;
use crate::ir::{ArgType, InsnType};

#[test]
fn editing_nested_uses_preserves_definitions_and_other_occurrences() {
    let reg = |n| InsnArg::reg(n, ArgType::INT);
    let mut inner = InsnNode::new(InsnType::CompoundAssign, 2);
    inner.result = Some(RegisterArg::new(5, ArgType::INT));
    inner.args = vec![reg(2), reg(3)];
    inner.payload.compound_target = Some(Box::new(reg(4)));
    let original = InsnArg::wrap(inner);
    let mut root = InsnNode::new(InsnType::CompoundAssign, 2);
    root.result = Some(RegisterArg::new(6, ArgType::INT));
    root.args = vec![reg(1), original.clone()];
    root.payload.compound_target = Some(Box::new(original.clone()));

    let mut order = Vec::new();
    InstructionTree::edit_register_uses(&mut root, |register| {
        order.push(register.reg_num);
        register.ssa_version = Some(order.len() as u32);
    });
    assert_eq!(order, [1, 2, 3, 4, 2, 3, 4]);
    assert_eq!(root.result.unwrap().ssa_version, None);
    for (value, first) in [
        (&root.args[1], Some(2)),
        (root.payload.compound_target.as_deref().unwrap(), Some(5)),
        (&original, None),
    ] {
        let InsnArg::Wrapped(operation) = value else {
            unreachable!()
        };
        assert_eq!(operation.result.as_ref().unwrap().ssa_version, None);
        assert_eq!(operation.args[0].as_register().unwrap().ssa_version, first);
        assert_eq!(
            operation
                .payload
                .compound_target
                .as_deref()
                .unwrap()
                .as_register()
                .unwrap()
                .ssa_version,
            first.map(|v| v + 2)
        );
    }
}
