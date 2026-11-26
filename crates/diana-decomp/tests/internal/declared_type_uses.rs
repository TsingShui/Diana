use super::*;
use crate::ir::PrimitiveType;

#[test]
fn imports_keep_the_first_unresolved_type_in_declaration_order() {
    let first = ArgType::Unknown(std::borrow::Cow::Borrowed(&[PrimitiveType::Long]));
    let later = ArgType::Unknown(std::borrow::Cow::Borrowed(&[PrimitiveType::Byte]));
    assert!(later < first);
    let repeated = ArgType::object("alpha/Shared");
    let mut uses = DeclaredTypeUses::default();
    uses.extend([
        &ArgType::array(repeated.clone()),
        &repeated,
        &ArgType::array(ArgType::array(first.clone())),
        &later,
    ]);
    assert!(matches!(uses.finish(), Err(JavaTypeNameError::Unresolved(found)) if found == first));
}

#[test]
fn duplicate_nested_arrays_share_all_abi_components_and_keep_name_collisions() {
    let left = ArgType::object("alpha/Shared");
    let right = ArgType::object("beta/Shared");
    let array = ArgType::array(left.clone());
    let nested = ArgType::array(array.clone());
    let mut uses = DeclaredTypeUses::default();
    uses.extend([&nested, &array, &left, &nested, &right]);
    let (abi_types, imports) = uses.finish().unwrap();
    let names =
        super::super::type_names::JavaTypeNameResolver::from_imports(None, None, None, imports);
    assert_eq!(
        names.resolve_type(&left).unwrap().to_string(),
        "alpha.Shared"
    );
    assert_eq!(
        names.resolve_type(&right).unwrap().to_string(),
        "beta.Shared"
    );
    assert_eq!(abi_types, BTreeSet::from([left, right, array, nested]));
}
