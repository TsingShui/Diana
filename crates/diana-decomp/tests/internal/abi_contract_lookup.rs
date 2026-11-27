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

#[test]
fn overload_fallback_preserves_shadowed_and_independent_declarations_on_cache_hits() {
    let platform = GenericTypeHierarchy::from_classes(std::iter::empty()).unwrap();
    let local = super::tests::class_with_methods(
        "Ljava/util/ArrayList;",
        None,
        &[("remove", vec![ArgType::LONG], ArgType::VOID)],
    );
    let primary = GenericTypeHierarchy::from_classes([&local]).unwrap();
    let method = "Ljava/util/ArrayList;->remove(I)Ljava/lang/Object;"
        .parse()
        .unwrap();
    let local = "Ljava/util/ArrayList;->remove(J)V".parse().unwrap();
    let erased = "Ljava/util/ArrayList;->remove(Ljava/lang/Object;)Z"
        .parse()
        .unwrap();
    let mut expected: std::collections::BTreeSet<_> = platform
        .method_overloads_walk(&method)
        .into_iter()
        .collect();
    assert!(expected.contains(&erased));
    assert!(expected.insert(local));
    for _ in 0..2 {
        let mut output = Default::default();
        primary.extend_method_overloads(&method, Some(&platform), &mut output);
        assert_eq!(output, expected);
    }
}

#[test]
fn a_physical_ancestor_miss_keeps_a_different_declared_generic_parent_reachable() {
    let mut hierarchy = GenericTypeHierarchy::from_classes(std::iter::empty()).unwrap();
    let root = ArgType::object("fixture/Root");
    let target = ArgType::object("java/lang/String");
    let object = ArgType::object("java/lang/Object");
    let mut owner = (*hierarchy.hierarchy.class_details(&object).unwrap()).clone();
    owner.descriptor = root.to_descriptor();
    owner.generic_signature = Some(GenericSignatures::class("Ljava/lang/String;").unwrap());
    owner.parents = vec![object.clone()];
    let generic = GenericSignatures::field("Ljava/lang/String;").unwrap();
    owner.generic_parents = vec![generic.clone()];
    hierarchy
        .hierarchy
        .loaded
        .classes
        .insert(root.clone(), Arc::new(owner));
    let child = ArgType::object("fixture/Child");
    let root_signature = GenericSignatures::field("Lfixture/Root;").unwrap();
    let mut derived = (*hierarchy.hierarchy.class_details(&object).unwrap()).clone();
    derived.descriptor = child.to_descriptor();
    derived.generic_signature = None;
    derived.parents = vec![root.clone()];
    derived.generic_parents = vec![root_signature.clone()];
    hierarchy
        .hierarchy
        .loaded
        .classes
        .insert(child.clone(), Arc::new(derived));
    for (owner, source) in [
        (&root, root_signature),
        (&child, GenericSignatures::field("Lfixture/Child;").unwrap()),
    ] {
        for _ in 0..2 {
            assert!(!hierarchy.is_subtype(owner, &target));
            assert!(hierarchy.is_subtype(owner, &object));
            assert_eq!(
                hierarchy.project_supertype(&source, &target).as_ref(),
                Some(&generic)
            );
        }
    }
}
