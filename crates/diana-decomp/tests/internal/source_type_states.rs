use super::*;

#[test]
fn definition_lookup_preserves_absent_and_conflicting_facts_after_repeated_writes() {
    let object = TypeState::Exact(JavaType::source_class("java.lang.Object"));
    let mut states = SourceTypeStates::default();
    states.insert(1, TypeState::Conflict);
    states.insert(2, object.clone());
    let definitions = states.current().clone();
    assert!(states.definition(&2).is_none());
    states.seal_definitions();
    for key in [1, 2, 3, 1, 3] {
        states.insert(key, object.clone());
        assert_eq!(states.definition(&key), definitions.get(&key));
    }
    assert_eq!(
        states.definitions(),
        definitions.iter().map(|(&key, ty)| (key, ty)).collect()
    );
    let retained = states.retained(|_| false);
    assert_eq!(retained.definition(&1), None);
    assert_eq!(
        retained.definition(&2),
        Some(&JavaType::source_class("java.lang.Object"))
    );
    assert_eq!(retained.definition(&3), None);
    assert_eq!(
        retained.get(&3),
        Some(&JavaType::source_class("java.lang.Object"))
    );
}

#[test]
fn type_roles_keep_variable_and_value_with_the_same_numeric_identity_independent() {
    let mut roles = TypeRoles::default();
    let value = SsaVar::new(3, 0);
    roles.mark(3, TypeRole::FIXED | TypeRole::ABI);
    roles.mark(value, TypeRole::DECLARATION | TypeRole::CONTEXTUAL);
    roles.mark(3, TypeRole::JOIN);
    assert!(roles.has(3, TypeRole::ABI | TypeRole::FIXED | TypeRole::JOIN));
    assert!(!roles.has(3, TypeRole::CONTEXTUAL));
    assert!(roles.has(value, TypeRole::CONTEXTUAL | TypeRole::DECLARATION));
    assert!(!roles.has(value, TypeRole::FIXED));
    assert!(!roles.has(SsaVar::new(3, 1), TypeRole::CONTEXTUAL));
}

#[test]
fn unchecked_generic_bounds_only_consume_argument_types_with_a_parameterized_erasure() {
    use crate::ir::generic_types::GenericSignatures;
    let signature = GenericSignatures::method(
        "<T:Ljava/lang/Object;:Ljava/util/List<Ljava/lang/String;>;U:Ljava/lang/Object;>(TT;TU;)V",
    )
    .unwrap();
    let contract = GenericMethodContract {
        signature,
        owner: ClassTypeSignature {
            raw_name: "fixture/NestedCalls".into(),
            type_arguments: Vec::new(),
            inner_segments: Vec::new(),
        },
        owner_parameters: Vec::new(),
    };
    let raw = JavaType::source_class("java.util.List");
    let types = BTreeMap::from([(ArgType::object("java/util/List"), raw.clone())]);
    let bounded = &contract.signature.parameter_types[0];
    let unbounded = &contract.signature.parameter_types[1];
    let unreachable = || -> Option<&JavaType> { panic!("no parameterized formal can match") };
    assert!(
        !GenericInvocationCompatibility::requires_unchecked_conversion(
            unbounded,
            unreachable,
            &contract,
            &types
        )
    );
    assert!(
        !GenericInvocationCompatibility::requires_unchecked_conversion(
            bounded,
            unreachable,
            &contract,
            &BTreeMap::new()
        )
    );
    assert!(
        GenericInvocationCompatibility::requires_unchecked_conversion(
            bounded,
            || Some(&raw),
            &contract,
            &types
        )
    );
    let other = JavaType::source_class("java.util.Set");
    assert!(
        !GenericInvocationCompatibility::requires_unchecked_conversion(
            bounded,
            || Some(&other),
            &contract,
            &types
        )
    );
}
