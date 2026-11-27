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
        EquationOutputs::default(),
        &[],
        &calls,
        &BTreeMap::new(),
        &GenericMethodContracts::Owned(Arc::new(generic)),
        |ty| ty == &ArgType::BOOLEAN,
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
        EquationOutputs::default(),
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
        |_| false,
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
        EquationOutputs::default(),
        &[],
        &calls,
        &BTreeMap::new(),
        &GenericMethodContracts::Owned(Arc::default()),
        |_| false,
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

#[test]
fn bound_completion_propagates_through_later_parameters_and_fills_closed_defaults() {
    use crate::ir::generic_types::GenericSignatures;
    let parameters =
        GenericSignatures::method("<E:Ljava/lang/Object;U:TE;T:TU;R:Ljava/lang/String;>()V")
            .unwrap()
            .type_parameters;
    let object = JavaType::source_class("java.lang.Object");
    let string = JavaType::source_class("java.lang.String");
    let types = BTreeMap::from([
        (ArgType::object("java/lang/Object"), object),
        (ArgType::object("java/lang/String"), string.clone()),
    ]);
    let mut solver = GenericTypeSolver::new(&types).with_inference_variables(&parameters);
    solver.constrain(&JvmTypeSignature::TypeVariable("T".into()), &string);
    solver.complete_with_bounds(&parameters);
    for name in ["E", "U", "T", "R"] {
        assert_eq!(solver.get(name).as_ref(), Some(&string), "{name}");
    }
}

#[test]
fn class_assignability_cache_keeps_ordered_immutable_type_pairs() {
    fn class(name: &str) -> JavaClassType {
        match JavaType::source_class(name) {
            JavaType::Class(class) => class,
            _ => unreachable!(),
        }
    }
    let left = class("fixture.Left");
    let right = class("fixture.Right");
    let mut cache = AssignabilityCache::default();
    cache.insert(&left, &right, true);
    cache.insert(&right, &left, false);
    assert_eq!(cache.get(&left.clone(), &right), Some(true));
    assert_eq!(cache.get(&right, &left), Some(false));
    let mut parameterized = left.clone();
    parameterized
        .segments_mut()
        .last_mut()
        .unwrap()
        .arguments
        .push(JavaTypeArgument::Any);
    assert_eq!(cache.get(&parameterized, &right), None);
    assert_eq!(cache.get(&left, &right), Some(true));
    for index in 0..8 {
        cache.insert(&class(&format!("fixture.C{index}")), &right, false);
    }
    assert_eq!(cache.get(&left, &right), None);
}

#[test]
fn source_constraints_use_the_first_seen_erasure_without_publishing_later_definitions() {
    use crate::ir::{
        BlockId, InsnArg, InsnNode, InstructionId, InvokeType, LiteralArg, SemanticBlock,
    };

    let string = ArgType::object("java/lang/String");
    let register = |id, ty| {
        let mut register = RegisterArg::with_ssa(2, ty, 1);
        register.code_var = Some(id);
        register
    };
    let definition = |id, ty| {
        SemanticStatement::definition(
            InstructionId::new(id as usize),
            register(id, ty),
            SemanticExpression::Literal(LiteralArg::new(1, ArgType::INT)),
        )
    };
    let write = |id| {
        let mut store = InsnNode::new(InsnType::Sput, 1);
        store.payload.reference = Some(Arc::new(MemberReference::Field(
            "Lfixture/Scope;->count:I".parse().unwrap(),
        )));
        store.args.push(InsnArg::Reg(register(id, string.clone())));
        SemanticStatement::instruction(store).unwrap()
    };
    let mut call = InsnNode::new(InsnType::Invoke, 1);
    call.payload.reference = Some(Arc::new(MemberReference::Method(
        "Lfixture/Scope;->consume(I)V".parse().unwrap(),
    )));
    call.payload.invoke_type = Some(InvokeType::Static);
    call.args.push(InsnArg::Reg(register(15, string.clone())));
    let root = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements: vec![
            write(11),
            definition(12, ArgType::INT),
            definition(13, string.clone()),
            write(14),
            SemanticStatement::instruction(call).unwrap(),
        ],
    });
    let types = BTreeMap::from([
        (ArgType::INT, JavaType::int()),
        (string.clone(), JavaType::source_class("String")),
    ]);
    let facts = SourceTypeFlow::solve(
        &root,
        &BTreeMap::new(),
        &BTreeMap::new(),
        &SourceObjectTypes::default(),
        &GenericMethodContracts::Owned(Arc::default()),
        None,
        &types,
        Arc::new(JavaTypeErasureIndex::from_source_types(types.clone())),
        &BTreeMap::new(),
        &BTreeMap::new(),
        None,
        None,
        None,
        None,
        &BTreeMap::new(),
        true,
    )
    .diagnostics();
    assert_eq!(
        facts.requirements,
        [(14, "int".to_owned()), (15, "int".to_owned())]
    );
    assert_eq!(facts.value_requirements, [(2, 1, "int".to_owned())]);
    let definitions: Vec<_> = facts
        .equations
        .iter()
        .map(|eq| (eq.variable, eq.register, eq.version, &eq.erased_type))
        .collect();
    assert_eq!(
        definitions,
        [
            (12, 2, Some(1), &ArgType::INT.to_string()),
            (13, 2, Some(1), &string.to_string())
        ]
    );
    assert_eq!(facts.invocations.len(), 1);
    assert!(!facts.invocations[0].resolved);
}

#[test]
fn source_type_lookup_preserves_boolean_aliases_missing_entries_and_first_reverse_spelling() {
    let first = ArgType::object("fixture/A");
    let second = ArgType::object("fixture/B");
    let aliased = BTreeMap::from([
        (ArgType::BOOLEAN, JavaType::source_class("Flag")),
        (first.clone(), JavaType::int()),
        (second.clone(), JavaType::int()),
        (ArgType::INT, JavaType::source_class("Other")),
        (
            ArgType::LONG,
            JavaType::Variable(JavaIdentifier::from_hint("T")),
        ),
    ]);
    let forward = SourceTypeLookup::new(&aliased);
    assert!(forward.is_dynamic_formal(&ArgType::INT));
    assert!(forward.is_dynamic_formal(&ArgType::LONG));
    assert!(!forward.is_dynamic_formal(&first));
    assert!(!forward.is_dynamic_formal(&ArgType::BYTE));
    assert_eq!(forward.get(&second), Some(&JavaType::int()));
    let erasures = JavaTypeErasureIndex::from_source_types(aliased);
    assert_eq!(
        erasures
            .erasure_of(&JavaType::int(), &BTreeMap::new())
            .as_deref(),
        Some(&first)
    );
    let plain = BTreeMap::from([
        (ArgType::BOOLEAN, JavaType::boolean()),
        (first.clone(), JavaType::boolean()),
    ]);
    assert!(SourceTypeLookup::new(&plain).is_dynamic_formal(&first));
}

#[test]
fn shared_and_reconstructed_class_shapes_keep_the_first_erasure_alias() {
    let first = ArgType::object("fixture/A");
    let next = ArgType::object("fixture/B");
    let from_a = JavaType::source_class("names.Shared");
    let from_b = JavaType::source_class("names.Shared");
    let index = JavaTypeErasureIndex::from_source_types(BTreeMap::from([
        (first.clone(), from_a.clone()),
        (next, from_b.clone()),
    ]));
    let variables = BTreeMap::new();
    for source in [from_a, from_b, JavaType::source_class("names.Shared")] {
        assert_eq!(
            index.erasure_of(&source, &variables).as_deref(),
            Some(&first)
        );
    }
    assert!(index
        .erasure_of(&JavaType::source_class("other.Shared"), &variables)
        .is_none());
}

#[test]
fn contracts_with_the_same_name_remain_distinct_by_owner_and_descriptor() {
    use crate::ir::generic_types::GenericSignatures;
    let mut methods = BTreeMap::new();
    for (owner, parameter, generic) in [
        ("One", "I", "()Ljava/lang/String;"),
        ("One", "J", "()Ljava/lang/Long;"),
        ("Two", "I", "()Ljava/lang/Integer;"),
    ] {
        let reference = format!("Lfixture/{owner};->value({parameter})Ljava/lang/Object;")
            .parse::<MethodReference>()
            .unwrap();
        let contract = Arc::new(GenericMethodContract {
            owner: ClassTypeSignature {
                raw_name: format!("fixture/{owner}"),
                type_arguments: Vec::new(),
                inner_segments: Vec::new(),
            },
            owner_parameters: Vec::new(),
            signature: GenericSignatures::method(generic).unwrap(),
        });
        methods.insert(reference, contract);
    }
    let index = GenericMethodIndex::from(methods.clone());
    for (method, expected) in &methods {
        assert!(std::ptr::eq(
            index.contract(method).unwrap(),
            expected.as_ref()
        ));
    }
    for missing in [
        "Lfixture/Two;->value(J)Ljava/lang/Object;",
        "Lfixture/One;->other(I)Ljava/lang/Object;",
    ] {
        assert!(index.contract(&missing.parse().unwrap()).is_none());
    }
}
