use super::*;
use crate::ir::{
    analysis::SourceTypeEnvironment, FieldReference, InsnNode, InsnType, InvokeType,
    MethodDescriptor, MethodReference, SemanticOperation,
};
use std::sync::Arc;

#[test]
fn shared_member_descriptors_keep_abi_only_types_and_each_static_use() {
    let environment = SourceTypeEnvironment::default();
    let mut collector = MethodSourceInputCollector::new(&environment, &[], None);
    let owner = ArgType::object("example/Owner");
    let method_owner = ArgType::object("example/Other");
    let element = ArgType::object("example/Element");
    let array = ArgType::array(element.clone());
    let matrix = ArgType::array(array.clone());
    let unresolved = ArgType::unknown_object();
    let field = Arc::new(MemberReference::Field(FieldReference {
        owner: owner.clone(),
        name: "field".into(),
        field_type: unresolved.clone(),
    }));
    let method = Arc::new(MemberReference::Method(MethodReference {
        owner: method_owner.clone(),
        name: "call".into(),
        descriptor: MethodDescriptor {
            parameters: vec![matrix.clone()],
            return_type: ArgType::VOID,
        },
    }));
    for (offset, (kind, reference, invoke)) in [
        (InsnType::Iget, &field, None),
        (InsnType::Invoke, &method, Some(InvokeType::Virtual)),
        (InsnType::Sget, &field, None),
        (InsnType::Invoke, &method, Some(InvokeType::Static)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut insn = InsnNode::new(kind, 0);
        insn.offset = offset as u32;
        insn.payload.reference = Some(Arc::clone(reference));
        insn.payload.invoke_type = invoke;
        collector.visit_operation(&SemanticOperation::from_instruction(insn).unwrap());
    }
    let uses = collector.finish().unwrap().type_uses;
    assert_eq!(
        uses.static_owners.as_ref(),
        &[method_owner.clone(), owner.clone()]
    );
    assert!(uses.references.iter().any(|item| Arc::ptr_eq(item, &field)));
    assert!(uses
        .references
        .iter()
        .any(|item| Arc::ptr_eq(item, &method)));
    assert_eq!(uses.unresolved_members.as_ref(), &[unresolved]);
    let known = uses
        .uses
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        known,
        [owner, method_owner, element, array, matrix, ArgType::VOID]
            .into_iter()
            .collect()
    );
}
