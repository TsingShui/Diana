use super::SparseConstantPropagation;
use crate::ir::analysis::SsaVar;
use crate::ir::{ArgType, Block, InsnArg, InsnNode, InsnType, InstructionId, RegisterArg, CFG};
use std::collections::BTreeMap;
use std::sync::Arc;

fn value(number: u32) -> SsaVar {
    SsaVar::new(number, 1)
}

fn read(number: u32) -> InsnArg {
    InsnArg::reg_ssa(number, 1, ArgType::INT)
}

fn define(number: u32, kind: InsnType, args: Vec<InsnArg>) -> InsnNode {
    let mut instruction = InsnNode::new(kind, args.len());
    instruction.set_result(RegisterArg::with_ssa(number, ArgType::INT, 1));
    instruction.args = args;
    instruction
}

fn constant(number: u32, id: usize, literal: i64) -> InsnNode {
    let mut instruction = define(
        number,
        InsnType::Const,
        vec![InsnArg::lit(literal, ArgType::INT)],
    );
    instruction.id = InstructionId::new(id);
    instruction.offset = id as u32 + 10;
    instruction
}

fn solve(instructions: Vec<InsnNode>) -> BTreeMap<SsaVar, InsnArg> {
    let mut cfg = CFG::new("orderedConstants");
    cfg.add_block(Block::with_instructions(0, 0, instructions));
    SparseConstantPropagation::new(&cfg).solve()
}

fn shared_source(first: &InsnArg, other: &InsnArg) -> bool {
    matches!((first,other), (InsnArg::Wrapped(a),InsnArg::Wrapped(b)) if Arc::ptr_eq(a,b))
}

#[test]
fn concurrent_discoveries_observe_the_original_sweep_and_first_wrapper() {
    // Right becomes available before left. The phi next runs after the
    // definition of left in physical instruction order, so left owns the result.
    let facts = solve(vec![
        define(0, InsnType::Move, vec![read(3)]),
        define(1, InsnType::Phi, vec![read(0), read(2)]),
        define(2, InsnType::Move, vec![read(5)]),
        define(3, InsnType::Move, vec![read(4)]),
        constant(4, 401, 17),
        constant(5, 501, 17),
        define(6, InsnType::Move, vec![read(1)]),
    ]);
    for i in [0, 1, 3, 6] {
        assert!(shared_source(&facts[&value(4)], &facts[&value(i)]));
    }
    assert!(shared_source(&facts[&value(5)], &facts[&value(2)]));
    assert!(!shared_source(&facts[&value(4)], &facts[&value(5)]));
    assert_eq!(
        facts[&value(1)].as_wrapped().unwrap().id,
        InstructionId::new(401)
    );
}

#[test]
fn later_equal_first_phi_operand_preserves_the_already_selected_input_wrapper() {
    let wrapped = InsnArg::wrap(constant(9, 951, 12));
    let facts = solve(vec![
        define(1, InsnType::Move, vec![read(4)]),
        define(2, InsnType::Phi, vec![read(1), wrapped.clone()]),
        constant(4, 451, 12),
    ]);
    assert!(shared_source(&facts[&value(2)], &wrapped));
    assert!(!shared_source(&facts[&value(2)], &facts[&value(1)]));
}

#[test]
fn loop_phi_discovers_an_invariant_before_its_backedge() {
    let facts = solve(vec![
        define(1, InsnType::Phi, vec![read(2), read(3)]),
        define(2, InsnType::Move, vec![read(1)]),
        constant(3, 317, 43),
    ]);
    assert!(shared_source(&facts[&value(1)], &facts[&value(3)]));
    assert!(shared_source(&facts[&value(2)], &facts[&value(3)]));
}

#[test]
fn bottom_for_a_duplicate_definition_leaves_the_known_value_unchanged() {
    let facts = solve(vec![
        constant(0, 71, 9),
        define(0, InsnType::Move, vec![read(1)]),
        define(1, InsnType::Move, vec![read(1)]),
        define(2, InsnType::Phi, vec![read(0), read(1)]),
        define(3, InsnType::Move, vec![read(99)]),
        define(4, InsnType::Phi, vec![read(0), read(3)]),
    ]);
    assert_eq!(
        facts.keys().copied().collect::<Vec<_>>(),
        [value(0), value(2)]
    );
    assert!(shared_source(&facts[&value(0)], &facts[&value(2)]));
}

#[test]
fn unkeyed_wrappers_widen_and_derived_results_follow_them() {
    let bad = InsnArg::wrap(define(
        38,
        InsnType::Const,
        vec![InsnArg::wrap(constant(39, 391, 41))],
    ));
    let facts = solve(vec![
        define(0, InsnType::Move, vec![bad]),
        define(1, InsnType::Neg, vec![read(0)]),
        define(2, InsnType::Phi, vec![read(1), read(1)]),
        define(3, InsnType::ConstStr, vec![]),
        define(4, InsnType::Move, vec![read(3)]),
        constant(5, 511, -41),
        define(6, InsnType::Phi, vec![read(1), read(5)]),
    ]);
    assert_eq!(facts.keys().copied().collect::<Vec<_>>(), [value(5)]);
}

#[test]
fn string_phis_compare_exact_utf16_and_retain_the_first_dex_index() {
    use crate::ir::Utf16String;
    let make = |number, index, units: &[u16]| {
        let mut i =
            InsnNode::const_string(RegisterArg::with_ssa(number, ArgType::string(), 1), index);
        i.id = InstructionId::new(index as usize);
        i.payload.string_value = Some(Utf16String::from_utf16(units.to_vec()).into());
        i
    };
    let facts = solve(vec![
        make(0, 12, &[0xd800, 0x41]),
        make(1, 17, &[0xd800, 0x41]),
        make(2, 24, &[0xd801, 0x41]),
        define(3, InsnType::Phi, vec![read(1), read(0)]),
        define(4, InsnType::Phi, vec![read(3), read(2)]),
    ]);
    assert!(shared_source(&facts[&value(1)], &facts[&value(3)]));
    assert_eq!(
        facts[&value(3)].as_wrapped().unwrap().payload.string_index,
        Some(17)
    );
    assert!(!facts.contains_key(&value(4)));
}

#[test]
fn folding_checks_all_operands_and_phi_keeps_literal_types_distinct() {
    let facts = solve(vec![
        define(
            0,
            InsnType::Move,
            vec![InsnArg::lit(13, ArgType::INT), read(99)],
        ),
        define(1, InsnType::Neg, vec![read(0), read(99)]),
        define(2, InsnType::Neg, vec![read(0)]),
        define(
            3,
            InsnType::Phi,
            vec![read(2), InsnArg::lit(-13, ArgType::LONG)],
        ),
    ]);
    assert_eq!(
        facts.keys().copied().collect::<Vec<_>>(),
        [value(0), value(2)]
    );
    assert_eq!(facts[&value(2)], InsnArg::lit(-13, ArgType::INT));
}

#[test]
fn unseeded_register_cycles_and_unknown_inputs_produce_no_constants() {
    let facts = solve(vec![
        define(0, InsnType::Phi, vec![read(2), read(3)]),
        define(1, InsnType::Move, vec![read(0)]),
        define(2, InsnType::Neg, vec![read(1)]),
        define(3, InsnType::Move, vec![read(40)]),
        define(4, InsnType::MoveException, vec![]),
    ]);
    assert!(facts.is_empty());
}

#[test]
fn reverse_definition_chain_wakes_each_invariant_phi_and_preserves_the_seed() {
    const DEPTH: u32 = 4096;
    let mut equations = Vec::new();
    for number in 0..DEPTH {
        let mut args = vec![read(number + 1)];
        let kind = if number % 2 == 0 {
            args.push(read(DEPTH + 1));
            InsnType::Phi
        } else {
            InsnType::Move
        };
        equations.push(define(number, kind, args));
    }
    equations.push(constant(DEPTH, 117, 53));
    equations.push(define(DEPTH + 1, InsnType::Move, vec![read(DEPTH + 1)]));
    let facts = solve(equations);
    assert_eq!(facts.len(), DEPTH as usize + 1);
    for n in 0..DEPTH {
        assert!(shared_source(&facts[&value(n)], &facts[&value(DEPTH)]));
    }
}
