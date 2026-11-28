mod support;

use diana_decomp::{MethodFailure, SingleMethodAnalysis};

const DIRECT: &[u8] =
    include_bytes!("../../../quality/fixtures/classes/direct-initialization/classes.dex");
const EFFECTS: &[u8] = include_bytes!(
    "../../../quality/fixtures/exceptions/direct-initialization-effects/classes.dex"
);

fn method(bytes: &[u8], owner: &str, name: &str, prototype: &str) -> SingleMethodAnalysis {
    let engine = support::engine(bytes);
    let class = support::class(bytes, &engine, owner);
    let facts = engine.class_facts(class).expect("class facts");
    let method = facts
        .resolve_method(
            &name.encode_utf16().collect::<Vec<_>>(),
            &prototype.encode_utf16().collect::<Vec<_>>(),
        )
        .expect("original method identity");
    engine
        .analyze_method(class, method)
        .expect("method analysis")
}

fn allocation_name(text: &str) -> &str {
    let line = text
        .lines()
        .find(|line| line.contains(" = allocate-instance Child;"))
        .expect("original Child allocation");
    line.split(" = ")
        .next()
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap()
}

fn no_failures(failures: &[MethodFailure]) {
    assert!(
        failures.is_empty(),
        "unexpected recovery failures: {failures:?}"
    );
}

#[test]
fn preserves_different_allocation_and_constructor_owners() {
    let analysis = method(
        DIRECT,
        "Lfixture/DirectInitialization;",
        "direct",
        "(I)Lfixture/Child;",
    );
    no_failures(&analysis.failures);
    let text = analysis.rendered.text();
    let name = allocation_name(text);
    assert!(
        text.contains(&format!(
            "init-direct {name}, Lfixture/Parent;-><init>(I)V, ["
        )),
        "{text}"
    );
    assert!(text.contains(&format!("return {name};")), "{text}");
    assert!(
        !text.contains("new Child("),
        "the Child constructor would overwrite value with 99: {text}"
    );
}

#[test]
fn ordinary_construction_keeps_natural_syntax() {
    let analysis = method(
        DIRECT,
        "Lfixture/DirectInitialization;",
        "ordinary",
        "(I)Lfixture/Child;",
    );
    no_failures(&analysis.failures);
    let text = analysis.rendered.text();
    assert!(text.contains("new Child("), "{text}");
    assert!(!text.contains("init-direct"), "{text}");
}

#[test]
fn string_builder_capacity_stays_before_appends_and_inside_its_handlers() {
    let input =
        include_bytes!("../../../quality/fixtures/atoms/string-builder-capacity/classes.dex");
    for (name, class) in [
        ("builder", "StringBuilder"),
        ("buffer", "StringBuffer"),
        ("protectedBuilder", "StringBuilder"),
    ] {
        let analysis = method(
            input,
            "Lfixture/BuilderCapacity;",
            name,
            "(II)Ljava/lang/String;",
        );
        no_failures(&analysis.failures);
        let text = analysis.rendered.text();
        let chain = format!("new {class}(i).append(BuilderCapacity.produce(1, i2))");
        let before_append = text.split_once(&chain).unwrap_or_else(|| panic!("{text}"));
        assert!(
            before_append
                .1
                .starts_with(".append(BuilderCapacity.produce(2, i2)).toString();"),
            "{text}"
        );
        if name == "protectedBuilder" {
            assert!(before_append.0.contains("try {"), "{text}");
            assert!(
                before_append
                    .1
                    .contains("catch (NegativeArraySizeException "),
                "{text}"
            );
            assert!(
                before_append.1.contains("catch (IllegalStateException "),
                "{text}"
            );
        }
    }
}

#[test]
fn string_builder_keeps_precomputed_values_before_object_conversion() {
    let input =
        include_bytes!("../../../quality/fixtures/atoms/string-builder-conversion/classes.dex");
    for (name, class) in [
        ("precomputed", "StringBuilder"),
        ("protectedBuffer", "StringBuffer"),
    ] {
        let analysis = method(
            input,
            "Lfixture/BuilderConversion;",
            name,
            "(I)Ljava/lang/String;",
        );
        no_failures(&analysis.failures);
        let text = analysis.rendered.text();
        let chain = format!("return new {class}().append(");
        let markers = [
            "BuilderConversion.produce(1,",
            "BuilderConversion.produce(2,",
            chain.as_str(),
            ").append(",
            ").toString();",
        ];
        let mut remaining = text;
        for marker in markers {
            remaining = remaining
                .split_once(marker)
                .unwrap_or_else(|| panic!("{text}"))
                .1;
        }
    }
}

#[test]
fn split_construction_preserves_each_exception_snapshot() {
    let analysis = method(
        include_bytes!(
            "../../../quality/fixtures/exceptions/staged-allocation-snapshots/classes.dex"
        ),
        "Lfixture/AllocationStages;",
        "run",
        "()V",
    );
    no_failures(&analysis.failures);
    let text = analysis.rendered.text();
    let (_, body) = text.split_once("try {").expect("protected construction");
    let operations = [
        " = 10;",
        "AllocationStages.prepare()",
        " = 15;",
        " = allocate-instance AllocationItem;",
        " = 20;",
        "AllocationStages.argument(",
        " = 30;",
        "init-direct ",
        "Lfixture/AllocationItem;-><init>(III)V",
        "AllocationStages.publish(",
        "catch (Throwable",
        "AllocationStages.caught(",
    ];
    let mut remaining = body;
    for operation in operations {
        remaining = remaining
            .split_once(operation)
            .unwrap_or_else(|| panic!("missing ordered {operation}: {text}"))
            .1;
    }
}

#[test]
fn observable_preparation_stays_before_the_constructor_expression() {
    let analysis = method(
        include_bytes!("../../../quality/fixtures/atoms/constructor-allocation-order/classes.dex"),
        "LConstructorPrefix;",
        "run",
        "(IZ)I",
    );
    no_failures(&analysis.failures);
    let text = analysis.rendered.text();
    let (_, body) = text
        .split_once("public static int run(")
        .expect("run method");
    let (preparation, construction) = body
        .split_once("new PrefixValue(")
        .expect("natural constructor expression");
    assert!(
        preparation.contains("ConstructorPrefix.prepareSecond("),
        "{text}"
    );
    assert!(
        !construction
            .lines()
            .next()
            .unwrap_or_default()
            .contains("prepareSecond("),
        "{text}"
    );
}

#[test]
fn argument_effects_after_allocation_remain_inside_the_expression() {
    let analysis = method(
        include_bytes!("../../../quality/fixtures/atoms/constructor-argument-effects/classes.dex"),
        "Lfixture/ArgumentEffects;",
        "tree",
        "(II)Lfixture/Point;",
    );
    no_failures(&analysis.failures);
    let text = analysis.rendered.text();
    assert!(
        text.contains("new Point(ArgumentEffects.finish(ArgumentEffects.read(0,"),
        "{text}"
    );
}

#[test]
fn retains_alias_effect_order_and_handler_boundaries() {
    let analysis = method(
        EFFECTS,
        "Lfixture/DirectInitializationEffects;",
        "build",
        "(I)Lfixture/Child;",
    );
    no_failures(&analysis.failures);
    let text = analysis.rendered.text();
    let name = allocation_name(text);
    let operations = [
        "try {".to_owned(),
        "allocate-instance Child".to_owned(),
        "DirectInitializationEffects.event(1)".to_owned(),
        "if (i < 0)".to_owned(),
        "DirectInitializationEffects.event(3)".to_owned(),
        format!("init-direct {name}, Lfixture/Parent;-><init>(I)V"),
        "DirectInitializationEffects.event(4)".to_owned(),
        format!("DirectInitializationEffects.last = {name}"),
        format!("return {name}"),
        "catch (RuntimeException".to_owned(),
        "DirectInitializationEffects.event(5)".to_owned(),
        "throw exception".to_owned(),
    ];
    let mut at = 0;
    for operation in operations {
        at += text[at..]
            .find(&operation)
            .unwrap_or_else(|| panic!("missing ordered {operation}: {text}"))
            + operation.len();
    }
    assert_eq!(text.matches("allocate-instance Child").count(), 1, "{text}");
}

#[test]
fn missing_constructor_remains_a_method_local_failure() {
    let analysis = method(
        DIRECT,
        "Lfixture/DirectInitialization;",
        "broken",
        "()Lfixture/Child;",
    );
    assert_eq!(analysis.failures.len(), 1);
    assert_eq!(analysis.failures[0].name, "broken");
    assert!(analysis.failures[0].reason.contains("has no constructor"));
    let text = analysis.rendered.text();
    assert!(text.contains("// Diana: WARNING: Failed to decompile this method."));
    assert!(text.contains("https://github.com/TsingShui/Diana/issues"));
    assert!(!text.contains("throw "), "invented exception: {text}");
}

#[test]
fn branched_constructors_retain_both_joined_values_and_the_handler() {
    let result = method(
        include_bytes!(
            "../../../quality/fixtures/exceptions/branched-constructor-values/classes.dex"
        ),
        "Lfixture/BranchedConstructorValues;",
        "run",
        "(ZZ)I",
    );
    no_failures(&result.failures);
    let source = result.rendered.text();
    let (body, handler) = source
        .split_once("catch (IllegalStateException")
        .expect("typed recovery");
    let (first, second) = body
        .split_once("try {")
        .expect("protected second construction");
    for operands in ["(3)", "(3, 4)"] {
        assert!(
            first.contains(&format!("new BranchedConstructorValues{operands}")),
            "{source}"
        );
    }
    for operands in ["(5)", "(5, 6)"] {
        assert!(
            second.contains(&format!("new BranchedConstructorValues{operands}")),
            "{source}"
        );
    }
    assert!(
        second.contains(".consume(") && handler.contains(".recover("),
        "{source}"
    );
}

#[test]
fn unprotected_constructors_after_a_try_keep_the_normal_and_exceptional_results() {
    let result = method(
        include_bytes!(
            "../../../quality/fixtures/exceptions/abrupt-finally-completion/classes.dex"
        ),
        "Lfixture/CleanupExits;",
        "differentThrow",
        "(I)V",
    );
    no_failures(&result.failures);
    let source = result.rendered.text();
    let (_, protected) = source.split_once("try {").expect("protected read");
    let (read, exits) = protected
        .split_once("catch (Throwable")
        .expect("read handler");
    let (handler, normal) = exits.split_once('}').expect("end of handler");
    assert!(read.contains("CleanupExits.read("), "{source}");
    assert!(!read.contains("CleanupExits.close("), "{source}");
    for (exit, message) in [(handler, "handler"), (normal, "normal")] {
        let (_, after_close) = exit.split_once("CleanupExits.close(").expect("cleanup");
        assert!(
            after_close.contains(&format!(
                "throw new IllegalArgumentException(\"{message}\");"
            )),
            "{source}"
        );
    }
}

#[test]
fn discarded_allocation_publishes_prune_even_when_only_instructions_change() {
    use std::sync::Mutex;

    use diana_decomp::ir::{
        analysis::ClassHierarchyIndex, passes::CfgPipeline, AnalysisEvent, AnalysisObserver,
        ArgType, EdgeKind, ExceptionHandler, InsnNode, PassResult, RegisterArg, Splitter,
    };

    #[derive(Default)]
    struct Events(Mutex<Vec<(&'static str, PassResult, usize, usize)>>);
    impl AnalysisObserver for Events {
        fn observe(&self, event: AnalysisEvent<'_>) {
            if let AnalysisEvent::CfgTransform {
                phase: "ssa",
                name,
                result,
                cfg,
            } = event
            {
                let edges = cfg
                    .blocks
                    .keys()
                    .flat_map(|&block| cfg.successors_with_kind(block))
                    .filter(|(_, kind)| *kind == EdgeKind::Exception)
                    .count();
                self.0
                    .lock()
                    .unwrap()
                    .push((name, result, edges, cfg.handlers.len()));
            }
        }
    }

    for protected in [false, true] {
        let class = ArgType::object("java/lang/StringBuilder");
        let mut allocation = InsnNode::new_instance(RegisterArg::new(0, class.clone()), 0);
        allocation.payload.class_type = Some(Box::new(class));
        let mut instructions = vec![allocation, InsnNode::return_void()];
        let mut handlers = Vec::new();
        if protected {
            handlers.push(ExceptionHandler::new(0, 1, 2, None));
            instructions.push(InsnNode::move_exception(RegisterArg::new(
                0,
                ArgType::object("java/lang/Throwable"),
            )));
            instructions.push(InsnNode::return_void());
        }
        for (pc, instruction) in instructions.iter_mut().enumerate() {
            instruction.set_offset(pc as u32);
        }
        let mut cfg = Splitter::new("discarded")
            .registers(1)
            .instructions(instructions)
            .handlers(handlers)
            .build();
        let events = Events::default();
        CfgPipeline::new(&ClassHierarchyIndex::default())
            .run_observed(&mut cfg, &events)
            .expect("recover the allocation");
        let events = events.0.into_inner().unwrap();
        let first_phi = events
            .iter()
            .position(|event| event.0 == "phi_simplify")
            .unwrap();
        let handlers = usize::from(protected);
        assert_eq!(events[first_phi].2, handlers);
        assert_eq!(
            &events[first_phi + 1..first_phi + 4],
            &[
                ("recover_constructors", PassResult::Changed, 0, handlers),
                ("prune_unreachable", protected.into(), 0, 0),
                ("phi_simplify", PassResult::Unchanged, 0, 0),
            ]
        );
        assert!(!cfg
            .blocks
            .values()
            .any(|block| block.insns.iter().any(|instruction| matches!(
                instruction.insn_type,
                diana_decomp::InsnType::NewInstance | diana_decomp::InsnType::MoveException
            ))));
    }
}
