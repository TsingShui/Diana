//! Structural and source-placement checks over ordinary Engine delivery.
//! Scenario expectations and LLM semantic review live with the input fixture;
//! these checks do not independently execute the final output plan.

use diana_decomp::p2::{NormalEdge, OpKind};
use diana_decomp::p4::{Family, Interpretation};
use diana_decomp::p5::Materialization;
use diana_decomp::p7::{
    ControlCompletion, ControlId, EmissionLayout, ExprKind, IfShape, MethodCode, TerminalKind,
    TransferId,
};
use diana_decomp::p9::{Origin, RenderOutcome, Rendered};
use diana_decomp::{
    ByteOffset, ClassDefId, ClassKey, DexId, DexImage, Engine, ImageOrigin, InputId,
};

fn inspect_at(
    fixture: &str,
    owner: &str,
    name: &str,
    prototype: &str,
    check: impl FnOnce(&MethodCode, &Rendered),
) {
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../quality/fixtures/branches")
            .join(fixture)
            .join("classes.dex"),
    )
    .expect("prepared fixture is required; missing input is not a skip");
    let index = diana_decomp::input::ProgramIndex::open(&bytes).unwrap();
    let owner: Vec<_> = owner.encode_utf16().collect();
    let class = ClassKey {
        dex: DexId(0),
        class_def: (0..index.class_count())
            .map(ClassDefId)
            .find(|&id| index.class(id).unwrap().descriptor.units().unwrap() == owner)
            .expect("original fixture class"),
    };
    let engine = Engine::open(vec![DexImage {
        bytes: &bytes,
        header_offset: ByteOffset(0),
        origin: ImageOrigin { input: InputId(0) },
    }])
    .unwrap();
    let method = engine
        .class_facts(class)
        .unwrap()
        .resolve_method(
            &name.encode_utf16().collect::<Vec<_>>(),
            &prototype.encode_utf16().collect::<Vec<_>>(),
        )
        .unwrap();
    let analysis = engine.analyze_method(class, method).unwrap();
    let diana_decomp::MethodOutcome::Expressed(expressed) = analysis.result.unwrap() else {
        panic!("{name}: P7 unavailable")
    };
    let RenderOutcome::Ready(rendered) = analysis.rendered else {
        panic!("{name}: P9 unavailable")
    };
    assert!(rendered.issues().is_empty());
    let code = expressed.result().code().unwrap();
    for operation in code.operations() {
        if matches!(operation.original, OpKind::Invoke { .. }) {
            let origin = Origin::Operation {
                method,
                operation: operation.source,
                pc: operation.pc,
            };
            assert_eq!(
                rendered
                    .spans()
                    .iter()
                    .filter(|s| s.origin == origin)
                    .count(),
                1,
                "{name}: one retained call anchor {origin:?}"
            );
        }
    }
    check(code, &rendered);
}

fn inspect(name: &str, prototype: &str, check: impl FnOnce(&MethodCode, &Rendered)) {
    inspect_at(
        "switch-fallthrough",
        "Lfixture/SwitchFallthrough;",
        name,
        prototype,
        check,
    );
}

fn cases(code: &MethodCode) -> Option<(&[ControlId], &[Vec<TransferId>])> {
    code.controls()
        .iter()
        .find_map(|control| match &control.layout {
            EmissionLayout::Switch {
                bodies,
                cases: Some(cases),
                ..
            } => Some((bodies.as_slice(), cases.as_slice())),
            _ => None,
        })
}

fn natural(code: &MethodCode, rendered: &Rendered) {
    let (bodies, groups) = cases(code).expect("selected lexical case groups");
    assert_eq!(bodies.len(), groups.len());
    let terminal = code
        .terminals()
        .iter()
        .find_map(|terminal| match &terminal.kind {
            TerminalKind::Switch { arms, .. } => Some(arms),
            _ => None,
        })
        .unwrap();
    let mut selected: Vec<_> = groups.iter().flatten().map(|id| id.0).collect();
    let mut original: Vec<_> = terminal.iter().map(|id| id.0).collect();
    selected.sort_unstable();
    original.sort_unstable();
    assert_eq!(
        selected, original,
        "each original case/default has one placement"
    );
    assert!(!rendered.text().contains("graph<"), "{}", rendered.text());
    assert!(!rendered.text().contains("goto "), "{}", rendered.text());
}

fn group_for(code: &MethodCode, key: i64) -> usize {
    cases(code).unwrap().1.iter().position(|group| group.iter().any(|id| {
        matches!(code.transfers()[id.0 as usize].role, Some(NormalEdge::SwitchCase { key: observed, .. }) if observed == key)
    })).unwrap()
}

fn initializer_before_switch(code: &MethodCode, rendered: &Rendered) {
    let operation = code
        .operations()
        .iter()
        .find(|op| matches!(op.original, OpKind::Const { bits: 0, .. }))
        .unwrap();
    let statement = &code.statement(operation.statement.unwrap()).unwrap();
    assert!(
        statement
            .outputs
            .iter()
            .any(|d| matches!(d.materialization(), Materialization::Store(_)))
    );
    let span = rendered.spans().iter().find(|s| matches!(s.origin, Origin::Operation { operation: id, pc, .. } if id == operation.source && pc == operation.pc)).unwrap();
    assert!(
        span.start.0 < span.end.0,
        "original constant materializes, rather than an invented default"
    );
    assert!(span.end.0 <= rendered.text().find("switch (").unwrap());
}

#[test]
fn shared_labels_own_one_body() {
    inspect("shared", "(I)I", |code, rendered| {
        natural(code, rendered);
        assert_eq!(group_for(code, 1), group_for(code, 2));
    });
}

#[test]
fn fallthrough_does_not_repeat_direct_entry_initialization() {
    inspect("chain", "(I)I", |code, rendered| {
        natural(code, rendered);
        initializer_before_switch(code, rendered);
        assert_eq!(group_for(code, 10) + 1, group_for(code, 11));
        let group = &cases(code).unwrap().1[group_for(code, 11)];
        assert!(group.iter().all(|id| {
            code.emission().transfers()[id.0 as usize]
                .copies
                .iter()
                .all(|bundle| code.copies()[bundle.0 as usize].steps.is_empty())
        }));
    });
}

#[test]
fn wide_initialization_keeps_its_original_definition() {
    inspect("wide", "(I)J", |code, rendered| {
        natural(code, rendered);
        initializer_before_switch(code, rendered);
        assert_eq!(group_for(code, 10) + 1, group_for(code, 11));
        assert!(rendered.text().contains("= 0L;"));
    });
}

#[test]
fn empty_default_completes_the_switch() {
    inspect("emptyDefault", "(I)I", |code, rendered| {
        natural(code, rendered);
        let (bodies, groups) = cases(code).unwrap();
        assert!(groups.last().unwrap().iter().any(|id| matches!(
            code.transfers()[id.0 as usize].role,
            Some(NormalEdge::SwitchDefault)
        )));
        assert!(
            code.controls()[bodies.last().unwrap().0 as usize]
                .entry
                .is_none()
        );
    });
}

#[test]
fn conditional_break_is_not_an_empty_arm_fallthrough() {
    inspect("conditional", "(II)I", |code, rendered| {
        natural(code, rendered);
        let (yes, no) = code
            .controls()
            .iter()
            .find_map(|control| match control.layout {
                EmissionLayout::If { yes, no, shape, .. } => {
                    assert!(!matches!(shape, IfShape::Empty));
                    Some((yes, no))
                }
                _ => None,
            })
            .expect("original predicate still selects different outcomes");
        assert!([yes, no].iter().any(|id| matches!(
            code.controls()[id.0 as usize].continuation,
            Some(ControlCompletion::Break(_) | ControlCompletion::BreakHere(_))
        )));
        assert!(rendered.text().contains("if (p1"));
    });
}

#[test]
fn nonidentity_parameter_entry_copy_stays_explicit() {
    inspect("parameterSeed", "(II)I", |code, rendered| {
        assert!(cases(code).is_none());
        assert!(code.copies().iter().any(|bundle| !bundle.steps.is_empty()));
        assert!(rendered.text().contains("= p1;"));
    });
}

#[test]
fn multiple_case_predecessors_keep_one_body_and_a_nonadjacent_goto() {
    inspect("sharedTail", "(I)I", |code, rendered| {
        let (bodies, _) = cases(code).expect("case-entry sharing is scheduled once");
        assert_eq!(group_for(code, 2) + 1, group_for(code, 3));
        let entry = code.controls()[bodies[group_for(code, 3)].0 as usize]
            .entry
            .unwrap();
        assert!(
            code.emission()
                .transfers()
                .iter()
                .any(|p| matches!(p.spelling,
            diana_decomp::p7::TransferSpelling::Goto(target) if target == entry))
        );
        assert!(!rendered.text().contains("graph<"));
        assert_eq!(rendered.text().matches(".mark(3)").count(), 1);
    });
}

#[test]
fn mixed_literal_interpretations_are_not_conflated() {
    inspect("mixedBits", "(I)I", |code, _| {
        assert!(cases(code).is_none());
        assert!(
            code.expressions()
                .iter()
                .any(|e| matches!(e.kind, ExprKind::Literal {
            bits, view: Interpretation::Literal { family: Family::Float, .. }
        } if bits.value == 0x3f800000))
        );
        let first = &code
            .statement(code.operations()[0].statement.unwrap())
            .unwrap();
        assert!(
            first
                .outputs
                .iter()
                .all(|d| matches!(d.materialization(), Materialization::Literal(_)))
        );
    });
}

#[test]
fn original_packed_switch_examples_are_structured() {
    for name in ["classify", "clustered"] {
        inspect_at(
            "branch-switch-packed",
            "Lfixture/BranchSwitchPacked;",
            name,
            "(I)I",
            |code, rendered| {
                natural(code, rendered);
                if name == "clustered" {
                    initializer_before_switch(code, rendered);
                }
            },
        );
    }
}
