use super::*;

#[test]
fn allocated_names_and_overloads_share_the_return_type_collision_order() {
    let owner = ArgType::object("fixture/Methods");
    let make = |name: &str, descriptor: &str| {
        JavaMethodSymbol::new(
            owner.clone(),
            JavaIdentifier::from_dex(name),
            descriptor.parse().unwrap(),
        )
    };
    let first = make("call", "(I)I");
    let other = make("call", "(I)J");
    let overload = make("call", "(J)I");
    let reserved = make("call$2", "(I)V");
    let names = JavaMemberNames::allocate(
        [],
        [other.clone(), reserved, overload.clone(), first.clone()],
    );
    let reference = |symbol: &JavaMethodSymbol| MethodReference {
        owner: owner.clone(),
        name: "call".to_owned(),
        descriptor: symbol.descriptor.clone(),
    };
    assert_eq!(names.method_symbol(&first).as_str(), "call");
    assert_eq!(names.method(&reference(&other)).as_str(), "call$22");
    assert_eq!(names.method(&reference(&overload)).as_str(), "call");
    assert_eq!(
        names.overloads(&reference(&first)).unwrap(),
        &BTreeSet::from([first.descriptor.clone(), overload.descriptor])
    );
    assert_eq!(
        names.overloads(&reference(&other)).unwrap(),
        &BTreeSet::from([other.descriptor])
    );
}
