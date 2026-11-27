use super::*;

#[test]
fn changing_visible_supertypes_invalidates_previous_cast_evidence() {
    let left = ArgType::object("example/Left");
    let right = ArgType::object("example/Right");
    let left_source = JavaType::source_class("Left");
    let right_source = JavaType::source_class("Right");
    let dialect = DexJavaDialect::new(
        true,
        None,
        &[],
        &[],
        &SourceTypeEnvironment::default(),
        BTreeMap::from([
            (left.clone(), left_source.clone()),
            (right, right_source.clone()),
        ]),
        Arc::new(JavaMemberNames::default()),
    )
    .expect("static method");
    assert!(!dialect.source_assignable_to(&left_source, &right_source));
    let dialect = dialect.with_source_object_types(Arc::new(
        BTreeMap::from([(left, right_source.clone())]).into(),
    ));
    assert!(dialect.source_assignable_to(&left_source, &right_source));
    let dialect = dialect.with_source_object_types(Arc::default());
    assert!(!dialect.source_assignable_to(&left_source, &right_source));
}

#[test]
fn overload_applicability_uses_the_operands_of_each_call_site() {
    let object = ArgType::object("java/lang/Object");
    let specific = ArgType::object("example/Specific");
    let reference = |descriptor: &str| MethodReference {
        owner: ArgType::object("example/Calls"),
        name: "call".to_owned(),
        descriptor: descriptor.parse().unwrap(),
    };
    let selected = reference("(Ljava/lang/Object;Ljava/lang/Object;)V");
    let members = JavaMemberNames::default().with_overloads([
        selected.clone(),
        reference("(Lexample/Specific;Ljava/lang/Object;)V"),
        reference("(Ljava/lang/Object;Lexample/Specific;)V"),
        reference("(Lexample/Specific;Lexample/Specific;)V"),
    ]);
    let dialect = DexJavaDialect::new(
        true,
        None,
        &[],
        &[],
        &SourceTypeEnvironment::default(),
        BTreeMap::from([
            (object.clone(), JavaType::source_class("Object")),
            (specific, JavaType::source_class("Specific")),
        ]),
        Arc::new(members),
    )
    .unwrap();

    let any = SemanticExpression::Register(RegisterArg::new(1, object));
    let null = SemanticExpression::Literal(crate::ir::LiteralArg::int(0));
    assert_eq!(
        dialect.overload_argument_casts(&selected, &[any.clone(), null.clone()]),
        BTreeSet::from([1])
    );
    assert_eq!(
        dialect.overload_argument_casts(&selected, &[null.clone(), any]),
        BTreeSet::from([0])
    );
    assert_eq!(
        dialect.overload_argument_casts(&selected, &[null.clone(), null]),
        BTreeSet::from([0, 1])
    );
}
