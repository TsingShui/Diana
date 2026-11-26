use super::*;

#[test]
fn cached_platform_contract_preserves_loaded_receiver_and_parent_signatures() {
    let platform = GenericTypeHierarchy::from_classes(std::iter::empty::<&ClassNode>()).unwrap();
    let get = "Ljava/util/ArrayList;->get(I)Ljava/lang/Object;"
        .parse()
        .unwrap();
    let size = "Ljava/util/ArrayList;->size()I".parse().unwrap();
    assert!(platform.method_contract(&get).is_some());
    assert!(platform.method_contract(&size).is_none());

    let mut parent = tests::class_with_methods(
        "Ljava/util/AbstractList;",
        Some("Ljava/lang/Object;"),
        &[("size", vec![], ArgType::INT)],
    );
    let parent_signature = "<Parent:Ljava/lang/Object;>()I";
    parent.methods_mut()[0].signature = Some(parent_signature.into());
    let hierarchy = GenericTypeHierarchy::from_classes([&parent]).unwrap();
    assert_eq!(
        hierarchy
            .method_contract_with_platform(&size, &platform)
            .unwrap()
            .signature,
        GenericSignatures::method(parent_signature).unwrap()
    );

    let mut receiver = tests::class_with_methods(
        "Ljava/util/ArrayList;",
        None,
        &[(
            "get",
            vec![ArgType::INT],
            ArgType::object("java/lang/Object"),
        )],
    );
    let receiver_signature = "<Local:Ljava/lang/Object;>(I)TLocal;";
    receiver.methods_mut()[0].signature = Some(receiver_signature.into());
    let hierarchy = GenericTypeHierarchy::from_classes([&receiver]).unwrap();
    assert_eq!(
        hierarchy
            .method_contract_with_platform(&get, &platform)
            .unwrap()
            .signature,
        GenericSignatures::method(receiver_signature).unwrap()
    );
}

#[test]
fn unlisted_constructors_do_not_create_platform_cache_rows() {
    let platform = GenericTypeHierarchy::from_classes(std::iter::empty::<&ClassNode>()).unwrap();
    let novel = "Ljava/util/ArrayList;-><init>(Lsample/InputOnlyType;)V"
        .parse()
        .unwrap();
    // Generic owners can still supply a readable erased constructor contract.
    assert!(platform.method_contract(&novel).is_some());
    let model = platform.hierarchy.platform.as_ref().unwrap();
    assert!(!model.method_contracts.read().unwrap().contains_key(&novel));
}
