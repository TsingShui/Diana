//! Documents the in-memory cost of the core IR node types; guards against
//! silent growth of the hot per-instruction footprint.
use diana_decomp::ir::insn::FillArrayData;
use diana_decomp::ir::{
    ArgType, BoolExpr, InsnArg, InsnNode, InsnPayload, MemberReference, MethodReference,
    RegisterArg, Utf16String, CFG,
};

#[test]
fn report_core_ir_sizes() {
    eprintln!(
        "sizeof InsnNode        = {}",
        std::mem::size_of::<InsnNode>()
    );
    eprintln!(
        "sizeof InsnPayload     = {}",
        std::mem::size_of::<InsnPayload>()
    );
    eprintln!(
        "sizeof MemberReference = {}",
        std::mem::size_of::<MemberReference>()
    );
    eprintln!(
        "sizeof MethodReference = {}",
        std::mem::size_of::<MethodReference>()
    );
    eprintln!(
        "sizeof ArgType         = {}",
        std::mem::size_of::<ArgType>()
    );
    eprintln!(
        "sizeof Utf16String     = {}",
        std::mem::size_of::<Utf16String>()
    );
    eprintln!(
        "sizeof BoolExpr        = {}",
        std::mem::size_of::<BoolExpr>()
    );
    eprintln!(
        "sizeof FillArrayData   = {}",
        std::mem::size_of::<FillArrayData>()
    );
    eprintln!(
        "sizeof InsnArg         = {}",
        std::mem::size_of::<InsnArg>()
    );
    eprintln!(
        "sizeof RegisterArg     = {}",
        std::mem::size_of::<RegisterArg>()
    );
    eprintln!("sizeof CFG             = {}", std::mem::size_of::<CFG>());

    // Regression guard: the rare/heavy payload fields (reference, string_value,
    // class_type, switch_cases, cast_type, fill_array_data, compound_target) are
    // boxed precisely so the hot per-instruction footprint stays small. Every
    // decoded archive holds millions of payloads, so even +8 bytes here is a
    // real memory win handed back. Raise these bounds only alongside a
    // documented memory-profile win.
    assert!(
        std::mem::size_of::<InsnPayload>() <= 184,
        "InsnPayload grew to {} bytes; payload fields are boxed to keep it small",
        std::mem::size_of::<InsnPayload>()
    );
    assert!(
        std::mem::size_of::<InsnNode>() <= 280,
        "InsnNode grew to {} bytes; payload fields are boxed to keep it small",
        std::mem::size_of::<InsnNode>()
    );
}

#[test]
fn cloned_instruction_metadata_keeps_ordered_edges_and_independent_edits() {
    use diana_decomp::ir::{BlockId, EdgeKind, InsnType, InstructionEquivalence};
    let mut original = InsnNode::new(InsnType::Switch, 0);
    original.payload.switch_cases = Some(Box::new(vec![(3, 1), (9, 5)]));
    original.payload.switch_default = Some(4);
    original.payload.phi_edges = vec![(BlockId::new(3), EdgeKind::Exception)];
    let mut changed = original.clone();
    changed
        .payload
        .phi_edges
        .push((BlockId::new(9), EdgeKind::Normal));
    changed.payload.switch_cases.as_mut().unwrap()[1].1 = 16;
    changed.payload.switch_default = Some(11);
    assert_eq!(original.payload.switch_cases.as_deref().unwrap()[1], (9, 5));
    assert_eq!(
        original.payload.phi_edges,
        [(BlockId::new(3), EdgeKind::Exception)]
    );
    assert_eq!(original.payload.switch_default, Some(4));
    // Operation equivalence ignores physical successor offsets and Phi edges.
    assert!(original.operation_equivalent(&changed));
    changed.payload.switch_cases.as_mut().unwrap().swap(0, 1);
    assert!(!original.operation_equivalent(&changed));

    let plain = InsnNode::new(InsnType::Nop, 0);
    let mut explicit = plain.clone();
    explicit.payload.switch_default = None;
    assert!(plain.operation_equivalent(&explicit));
    assert_eq!(
        format!("{:?}", plain.payload),
        format!("{:?}", explicit.payload)
    );
}

#[test]
fn immutable_operand_types_and_reference_clones_preserve_dex_identity() {
    use diana_decomp::ir::{InsnType, MemberReference};
    use std::sync::Arc;
    let array: ArgType = "[[Lexample/LongOriginalClassName;".parse().unwrap();
    let method: MethodReference = "Lexample/Owner;->read([[Lexample/LongOriginalClassName;)I"
        .parse()
        .unwrap();
    let mut instruction = InsnNode::new(InsnType::Invoke, 0);
    instruction.payload.reference = Some(Arc::new(MemberReference::Method(method)));
    let duplicate = instruction.clone();
    assert!(Arc::ptr_eq(
        instruction.payload.reference.as_ref().unwrap(),
        duplicate.payload.reference.as_ref().unwrap()
    ));
    assert_eq!(
        duplicate.payload.reference.unwrap().to_string(),
        "Lexample/Owner;->read([[Lexample/LongOriginalClassName;)I"
    );
    assert_eq!(
        array.clone().to_descriptor(),
        "[[Lexample/LongOriginalClassName;"
    );
    for ty in [
        ArgType::narrow(),
        ArgType::wide(),
        ArgType::unknown(),
        ArgType::unknown_object(),
        ArgType::equality_operand(),
    ] {
        let ArgType::Unknown(candidates) = ty else {
            unreachable!()
        };
        assert!(matches!(candidates, std::borrow::Cow::Borrowed(_)));
        assert_eq!(candidates.as_ref(), candidates.clone().as_ref());
    }
}
