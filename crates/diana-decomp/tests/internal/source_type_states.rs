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

#[test]
fn fixed_invocations_revisit_changed_requirements_and_dynamic_invocations_revisit_input_facts() {
    use crate::ir::{InsnNode, InvokeType};

    let make_call = |reference: &str, variable: u32| {
        let mut raw = InsnNode::new(InsnType::Invoke, 0);
        raw.payload.reference = Some(Arc::new(MemberReference::Method(
            reference.parse().unwrap(),
        )));
        raw.payload.invoke_type = Some(InvokeType::Static);
        let mut argument = RegisterArg::with_ssa(variable, ArgType::INT, 0);
        argument.code_var = Some(variable);
        SemanticOperation::from_parts(raw, vec![SemanticExpression::Register(argument)], None)
    };
    let calls = [
        make_call("Lfixture/Calls;->integer(I)V", 4),
        make_call("Lfixture/Calls;->predicate(Z)V", 4),
        make_call("Lfixture/Calls;->integer(I)V", 9),
        make_call("Lfixture/Calls;->receiver(Ljava/lang/Object;)V", 4),
    ];
    let generic = BTreeMap::from([(
        "Lfixture/Calls;->receiver(Ljava/lang/Object;)V"
            .parse()
            .unwrap(),
        GenericMethodContract {
            signature: crate::ir::generic_types::GenericSignatures::method(
                "<T:Ljava/lang/Object;>(TT;)V",
            )
            .unwrap(),
            owner: ClassTypeSignature {
                raw_name: "fixture/Calls".into(),
                type_arguments: Vec::new(),
                inner_segments: Vec::new(),
            },
            owner_parameters: Vec::new(),
        },
    )]);
    let mut graph = TypeEquationGraph::default();
    graph.rebuild(
        &[],
        &calls,
        &BTreeMap::new(),
        &GenericMethodContracts::Owned(Arc::new(generic)),
        &BTreeSet::from([&ArgType::BOOLEAN]),
    );
    let ready = |graph: &mut TypeEquationGraph| {
        graph
            .take_dirty_invocations(&calls)
            .indices()
            .collect::<Vec<_>>()
    };
    assert_eq!(ready(&mut graph), [0, 1, 2, 3]);

    let shared = SourceTypeFact::Variable(4);
    graph.mark_changed(shared);
    assert_eq!(ready(&mut graph), [1, 3]);
    assert!(graph.mark_requirement(shared, RequirementChange::Inserted));
    assert_eq!(ready(&mut graph), [1, 3]);
    assert!(graph.mark_requirement(shared, RequirementChange::Updated));
    assert_eq!(ready(&mut graph), [0, 1, 3]);
    assert!(graph.mark_requirement(
        SourceTypeFact::Value(SsaVar::new(9, 0)),
        RequirementChange::Updated
    ));
    assert_eq!(ready(&mut graph), [2]);
    assert!(!graph.mark_requirement(shared, RequirementChange::Unchanged));
    assert!(ready(&mut graph).is_empty());
}

#[test]
fn equation_inputs_track_array_storage_and_polymorphic_results() {
    use crate::ir::{InsnNode, InvokeType};

    let register = |number| {
        let mut value = RegisterArg::with_ssa(number, ArgType::object("fixture/Value"), 1);
        value.code_var = Some(number);
        value
    };
    let operation = |kind, result, operands| {
        let mut raw = InsnNode::new(kind, 0);
        raw.result = Some(register(result));
        SemanticExpression::Operation(SemanticOperation::from_parts(raw, operands, None))
    };
    let expression = |number| SemanticExpression::Register(register(number));
    let value = operation(
        InsnType::Aget,
        8,
        vec![
            operation(InsnType::Move, 2, vec![expression(1)]),
            operation(InsnType::Arith, 4, vec![expression(3)]),
        ],
    );
    let method: MethodReference = "Lfixture/Box;->element(Ljava/lang/Object;)Ljava/lang/Object;"
        .parse()
        .unwrap();
    let mut call = InsnNode::new(InsnType::Invoke, 0);
    call.payload.invoke_type = Some(InvokeType::Static);
    call.payload.reference = Some(Arc::new(MemberReference::Method(method.clone())));
    call.result = Some(register(9));
    let poly = SemanticExpression::Operation(SemanticOperation::from_parts(
        call,
        vec![operation(InsnType::Arith, 6, vec![expression(5)])],
        None,
    ));
    let contract = GenericMethodContract {
        signature: crate::ir::generic_types::GenericSignatures::method(
            "<T:Ljava/lang/Object;>(TT;)TT;",
        )
        .unwrap(),
        owner: ClassTypeSignature {
            raw_name: "fixture/Box".into(),
            type_arguments: Vec::new(),
            inner_segments: Vec::new(),
        },
        owner_parameters: Vec::new(),
    };
    let mut graph = TypeEquationGraph::default();
    graph.rebuild(
        &[
            TypeEquation {
                result: register(8),
                value,
            },
            TypeEquation {
                result: register(9),
                value: poly,
            },
        ],
        &[],
        &BTreeMap::new(),
        &GenericMethodContracts::Owned(Arc::new(BTreeMap::from([(method, contract)]))),
        &BTreeSet::new(),
    );
    let ready = |graph: &mut TypeEquationGraph| graph.take_dirty().indices().collect::<Vec<_>>();
    assert_eq!(ready(&mut graph), [0, 1]);
    for number in [2, 3, 4, 8] {
        graph.mark_changed(SourceTypeFact::Variable(number));
        assert!(ready(&mut graph).is_empty());
    }
    graph.mark_changed(SourceTypeFact::Value(SsaVar::new(1, 1)));
    assert_eq!(ready(&mut graph), [0]);
    graph.mark_changed(SourceTypeFact::Variable(6));
    assert_eq!(ready(&mut graph), [1]);
    graph.mark_requirement(SourceTypeFact::Variable(9), RequirementChange::Inserted);
    assert_eq!(ready(&mut graph), [1]);
}

#[test]
fn constructor_initialization_skips_only_the_implicit_receiver_for_constraints() {
    use crate::ir::{InsnNode, InvokeType, LiteralArg};

    let mut value = RegisterArg::with_ssa(5, ArgType::object("fixture/Box"), 1);
    value.code_var = Some(5);
    let variable = SemanticExpression::Register(value);
    let literal = SemanticExpression::Literal(LiteralArg::int(0));
    let make_call = |kind, invoke_type, reference: &str, operands| {
        let mut raw = InsnNode::new(kind, 0);
        raw.payload.invoke_type = Some(invoke_type);
        raw.payload.reference = Some(Arc::new(MemberReference::Method(
            reference.parse().unwrap(),
        )));
        SemanticOperation::from_parts(raw, operands, None)
    };
    let no_write = make_call(
        InsnType::Constructor,
        InvokeType::Direct,
        "Lfixture/Box;-><init>(I)V",
        vec![variable.clone(), literal],
    );
    let mut calls = vec![no_write; 70];
    calls.push(make_call(
        InsnType::Invoke,
        InvokeType::Virtual,
        "Lfixture/Box;->read()V",
        vec![variable.clone()],
    ));
    calls.push(make_call(
        InsnType::Constructor,
        InvokeType::Direct,
        "Lfixture/Box;-><init>(Ljava/lang/Object;)V",
        vec![variable.clone(), variable],
    ));
    let mut graph = TypeEquationGraph::default();
    graph.rebuild(
        &[],
        &calls,
        &BTreeMap::new(),
        &GenericMethodContracts::Owned(Arc::default()),
        &BTreeSet::new(),
    );
    let ready = |graph: &mut TypeEquationGraph| {
        graph
            .take_dirty_invocations(&calls)
            .indices()
            .collect::<Vec<_>>()
    };
    assert_eq!(ready(&mut graph), [70, 71]);
    graph.mark_requirement(SourceTypeFact::Variable(5), RequirementChange::Updated);
    assert_eq!(ready(&mut graph), [70, 71]);
}
