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

    // Most instructions need only the inline flags, indices and operand handles.
    // Copies share the optional decoded constants and tables until edited.
    assert!(
        std::mem::size_of::<InsnPayload>() <= 64,
        "InsnPayload grew to {} bytes; payload fields are boxed to keep it small",
        std::mem::size_of::<InsnPayload>()
    );
    assert!(
        std::mem::size_of::<InsnNode>() <= 160,
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
        let start = std::ptr::from_ref(&candidates).addr();
        assert!((start..start + std::mem::size_of_val(&candidates))
            .contains(&candidates.as_ptr().addr()));
        assert_eq!(candidates.as_ref(), candidates.clone().as_ref());
    }
}

#[test]
fn editing_decoded_constants_preserves_other_instruction_copies() {
    use diana_decomp::ir::{InsnType, InstructionEquivalence};
    let mut original = InsnNode::new(InsnType::ConstStr, 0);
    original.payload.string_index = Some(7);
    original.payload.string_value = Some(Box::new("original".into()));
    let mut changed = original.clone();
    changed.payload.string_index = Some(11);
    assert!(original.operation_equivalent(&changed));
    changed.payload.string_value = Some(Box::new("changed".into()));
    assert!(!original.operation_equivalent(&changed));
    assert_eq!(original.payload.string_index, Some(7));
    assert_eq!(
        original
            .payload
            .string_value
            .as_deref()
            .unwrap()
            .to_string_lossy(),
        "original"
    );

    let mut cast = InsnNode::new(InsnType::Cast, 0);
    cast.payload.cast_type = Some(Box::new(ArgType::INT));
    let mut other = cast.clone();
    other.payload.cast_type = Some(Box::new(ArgType::LONG));
    assert_eq!(cast.conversion_type(), Some(&ArgType::INT));
    assert_eq!(other.conversion_type(), Some(&ArgType::LONG));
    assert!(!cast.operation_equivalent(&other));
}

#[test]
fn checked_instruction_layout_retains_each_reference_slot_and_width() {
    use diana_decomp::raw::{
        instruction_layout, DexVersion, IndexedRef, LayoutKind, OperandReference, RawDecodeError,
        ReferenceSlot,
    };
    use diana_decomp::{CodeUnitCount, CodeUnitOffset, MethodId, ProtoId, StringId};

    let pc = CodeUnitOffset(0);
    let polymorphic = [0xfa, 0x20, 0, 0, 0x21, 0, 0xfe, 0xca];
    let insn = instruction_layout(&polymorphic, pc, DexVersion::V038).unwrap();
    assert_eq!(insn.width(), CodeUnitCount(4));
    assert!(
        matches!(insn.kind(), LayoutKind::Instruction(spec) if spec.mnemonic == "invoke-polymorphic")
    );
    assert_eq!(
        insn.references(),
        [
            Some(IndexedRef {
                target: OperandReference::Method(MethodId(0)),
                operand_slot: ReferenceSlot::First
            }),
            Some(IndexedRef {
                target: OperandReference::Proto(ProtoId(0xcafe)),
                operand_slot: ReferenceSlot::Second
            }),
        ]
    );
    assert!(matches!(
        instruction_layout(&polymorphic, pc, DexVersion::V035),
        Err(RawDecodeError::UnsupportedVersion { opcode: 0xfa, .. })
    ));
    let jumbo =
        instruction_layout(&[0x1b, 1, 0x78, 0x56, 0x34, 0x12], pc, DexVersion::V035).unwrap();
    assert_eq!(
        jumbo.references(),
        [
            Some(IndexedRef {
                target: OperandReference::String(StringId(0x1234_5678)),
                operand_slot: ReferenceSlot::First
            }),
            None,
        ]
    );
    let payload = instruction_layout(&[0, 2, 0, 0], pc, DexVersion::V035).unwrap();
    assert_eq!(payload.kind(), LayoutKind::SparseSwitch { entries: 0 });
    assert_eq!(payload.width(), CodeUnitCount(2));
    assert_eq!(payload.references(), [None, None]);
    assert!(std::mem::size_of::<diana_decomp::p1::DecodedInstruction>() <= 96);
}

#[test]
fn candidate_lists_preserve_slice_identity_across_inline_and_shared_lengths() {
    use diana_decomp::ir::{PrimitiveType, TypeCandidates};
    use std::hash::{BuildHasher, RandomState};
    let hash = RandomState::new();
    let kinds = [PrimitiveType::Float, PrimitiveType::Int, PrimitiveType::Int];
    for count in [0, 1, 2, 11, 12, 13, 32] {
        let input = kinds.into_iter().cycle().take(count).collect::<Vec<_>>();
        let candidates: TypeCandidates = input.iter().copied().collect();
        assert_eq!(&*candidates, input);
        assert_eq!(candidates, TypeCandidates::from(input.clone()));
        assert_eq!(format!("{candidates:?}"), format!("{input:?}"));
        assert_eq!(hash.hash_one(&candidates), hash.hash_one(&input[..]));
        let reversed: TypeCandidates = input.iter().rev().copied().collect();
        assert_eq!(candidates.cmp(&reversed), input.as_slice().cmp(&reversed));
    }
    #[cfg(target_pointer_width = "64")]
    assert!(std::mem::size_of::<ArgType>() <= 24);
}
