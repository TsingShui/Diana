use super::{ArgType, ClassHierarchy, ContractHierarchy, GenericTypeHierarchy};
use crate::ir::generic_types::GenericSignatures;
use std::sync::Arc;

#[test]
fn a_loaded_parent_of_a_platform_owner_preserves_platform_fallback() {
    let platform = GenericTypeHierarchy::from_classes(std::iter::empty()).unwrap();
    let reference = "Ljava/util/SortedMap;->computeIfAbsent(Ljava/lang/Object;Ljava/util/function/Function;)Ljava/lang/Object;"
        .parse().unwrap();
    let expected =
        GenericSignatures::method("(TK;Ljava/util/function/Function<-TK;+TV;>;)TV;").unwrap();
    let mut primary = platform.clone();
    let owner = ArgType::object("java/util/Map");
    let mut shadow = (*platform.hierarchy.class_details(&owner).unwrap()).clone();
    shadow.methods.clear();
    shadow.method_candidates = Default::default();
    primary
        .hierarchy
        .loaded
        .classes
        .insert(owner, Arc::new(shadow));

    let lookup = ContractHierarchy::new(&primary.hierarchy);
    assert!(GenericTypeHierarchy::lookup_method_contract(&lookup, &reference).is_none());
    assert!(lookup.used_loaded.get());
    assert_eq!(
        primary
            .method_contract_with_platform(&reference, &platform)
            .unwrap()
            .signature,
        expected
    );

    let replacement =
        GenericSignatures::method("(TK;Ljava/util/function/Function<-TK;+TV;>;)TK;").unwrap();
    let parent = ArgType::object("java/util/Map");
    let mut local = (*platform.hierarchy.class_details(&parent).unwrap()).clone();
    let method = local
        .methods
        .iter_mut()
        .find(|method| method.reference.short_id.starts_with("computeIfAbsent("))
        .unwrap();
    method.generic_signature = Some(replacement.clone());
    primary
        .hierarchy
        .loaded
        .classes
        .insert(parent, Arc::new(local));
    assert_eq!(
        primary
            .method_contract_with_platform(&reference, &platform)
            .unwrap()
            .signature,
        replacement
    );
}

#[test]
fn an_absent_owner_and_an_absent_inherited_method_consume_only_platform_facts() {
    let platform = GenericTypeHierarchy::from_classes(std::iter::empty()).unwrap();
    for reference in [
        "Lfixture/MissingOwner;->get()Ljava/lang/Object;",
        "Ljava/util/HashMap;->missingContract()Ljava/lang/Object;",
    ] {
        let method = reference.parse().unwrap();
        let lookup = ContractHierarchy::new(&platform.hierarchy);
        assert!(GenericTypeHierarchy::lookup_method_contract(&lookup, &method).is_none());
        assert!(!lookup.used_loaded.get());
        assert!(platform
            .method_contract_with_platform(&method, &platform)
            .is_none());
    }
}

#[test]
fn a_primary_without_platform_facts_uses_the_independent_platform_provider() {
    let platform = GenericTypeHierarchy::from_classes(std::iter::empty()).unwrap();
    let mut primary = platform.clone();
    primary.hierarchy.platform = None;
    let reference = "Ljava/util/Comparator;->compare(Ljava/lang/Object;Ljava/lang/Object;)I"
        .parse()
        .unwrap();
    assert!(primary.method_contract(&reference).is_none());
    assert_eq!(
        primary
            .method_contract_with_platform(&reference, &platform)
            .unwrap()
            .signature,
        GenericSignatures::method("(TT;TT;)I").unwrap(),
    );
}
