use super::*;

#[test]
fn unresolved_member_facts_do_not_preempt_or_suppress_import_errors() {
    let unavailable = ArgType::unknown_object();
    let mut types = DeclaredTypeUses::default();
    types.abi_only.push(unavailable.clone());
    types.insert(&ArgType::array(ArgType::object("example/Element")));
    let (abi, _) = types.finish().unwrap();
    assert!(abi.contains(&unavailable));
    assert!(abi.contains(&ArgType::object("example/Element")));

    let mut types = DeclaredTypeUses::default();
    types.abi_only.push(unavailable.clone());
    types.insert(&unavailable);
    types.insert(&ArgType::unknown());
    assert!(matches!(types.finish(), Err(JavaTypeNameError::Unresolved(ty)) if ty == unavailable));
}
