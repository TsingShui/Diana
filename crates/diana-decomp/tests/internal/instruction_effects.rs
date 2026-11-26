use super::*;
use crate::ir::ArithOp;

fn fields(effects: InstructionEffects) -> [bool; 6] {
    [
        effects.reads_memory(),
        effects.writes_memory(),
        effects.calls(),
        effects.may_throw(),
        effects.synchronizes(),
        effects.controls(),
    ]
}

#[test]
fn division_edge_copies_and_nested_operations_keep_their_observable_effects() {
    let mut division = InsnNode::new(InsnType::Arith, 0);
    division.payload.arith_op = Some(ArithOp::Div);
    division.result = Some(RegisterArg::new(0, ArgType::FLOAT));
    assert!(InstructionEffects::of(&division).is_pure());
    division.result = Some(RegisterArg::new(0, ArgType::INT));
    assert_eq!(
        fields(InstructionEffects::of(&division)),
        [false, false, false, true, false, false]
    );
    division.payload.edge_copy = true;
    assert_eq!(
        fields(InstructionEffects::of(&division)),
        [false, true, false, false, false, true]
    );

    let mut outer = InsnNode::new(InsnType::Return, 1);
    outer
        .args
        .push(InsnArg::wrap(InsnNode::new(InsnType::Invoke, 0)));
    outer.payload.compound_target = Some(Box::new(InsnArg::wrap(InsnNode::new(
        InsnType::MonitorExit,
        0,
    ))));
    assert!(InstructionEffects::of(&outer).is_control_only());
    assert_eq!(
        fields(InstructionEffects::of_argument(&outer.args[0])),
        [true, true, true, true, false, false]
    );
    assert_eq!(fields(InstructionEffects::of_tree(&outer)), [true; 6]);
}

#[test]
fn effect_sets_preserve_the_field_order_and_composition_rules() {
    assert_eq!(std::mem::size_of::<InstructionEffects>(), 1);
    assert_eq!(
        format!("{:?}", InstructionEffects::SYNCHRONIZATION),
        "InstructionEffects { reads: false, writes: false, calls: false, may_throw: true, synchronizes: true, controls: false }"
    );
    for bits in 0..64 {
        let left = InstructionEffects::from_bits(bits);
        assert_eq!(left.bits(), bits);
        assert_eq!(InstructionEffects::from_bits(bits | 0xc0), left);
        let [reads, writes, calls, throws, sync, control] = fields(left);
        assert_eq!(left.is_pure(), fields(left) == [false; 6]);
        assert_eq!(left.can_relocate(), !sync && !control);
        assert_eq!(left.can_predicate(), !writes && !calls && !sync && !control);
        assert_eq!(
            fields(left.without_control()),
            [reads, writes, calls, throws, sync, false]
        );
        assert_eq!(
            left.is_control_only(),
            fields(left) == [false, false, false, false, false, true]
        );
        for other_bits in 0..64 {
            let right = InstructionEffects::from_bits(other_bits);
            let other = fields(right);
            assert_eq!(left.cmp(&right), fields(left).cmp(&other));
            let union = std::array::from_fn(|i| fields(left)[i] || other[i]);
            assert_eq!(fields(left.join(right)), union);

            let [other_reads, other_writes, other_calls, other_throws, other_sync, other_control] =
                other;
            let conflict = !left.is_pure()
                && !right.is_pure()
                && (control
                    || other_control
                    || sync
                    || other_sync
                    || calls
                    || other_calls
                    || (writes && (other_reads || other_writes))
                    || (other_writes && (reads || writes))
                    || (throws && other_throws));
            assert_eq!(left.conflicts_with(right), conflict);
        }
    }
}
