//! Structural checks over the ordinary Engine delivery. These do not execute
//! the final plan or establish behavioral correspondence; fixture scenarios and
//! that missing evidence are recorded in quality/fixtures/loops/nested-loop-exits/README.md.

use diana_decomp::p6::{LoopPlan, RegionFacts, TransferAction};
use diana_decomp::p9::{Origin, RenderOutcome, Rendered};
use diana_decomp::{
    ByteOffset, ClassDefId, ClassKey, DexId, DexImage, Engine, ImageOrigin, InputId,
};

fn inspect(name: &str, check: impl FnOnce(&RegionFacts, &Rendered)) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/loops/nested-loop-exits/classes.dex");
    let bytes = std::fs::read(path).expect(
        "read quality/fixtures/loops/nested-loop-exits/classes.dex (missing input is not a skip)",
    );
    let index = diana_decomp::input::ProgramIndex::open(&bytes).unwrap();
    let class = (0..index.class_count())
        .map(ClassDefId)
        .find(|&id| {
            let view = index.class(id).unwrap();
            view.descriptor.units().unwrap()
                == "Lfixture/LoopExits;".encode_utf16().collect::<Vec<_>>()
        })
        .expect("fixture class identity");
    let class = ClassKey {
        dex: DexId(0),
        class_def: class,
    };
    let engine = Engine::open(vec![DexImage {
        bytes: &bytes,
        header_offset: ByteOffset(0),
        origin: ImageOrigin { input: InputId(0) },
    }])
    .unwrap();
    let prototype = if name == "siblingLoop" {
        "(II)I"
    } else {
        "(III)I"
    };
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
        panic!("fixture method did not reach P7");
    };
    let RenderOutcome::Ready(rendered) = analysis.rendered else {
        panic!("fixture method did not reach P9");
    };
    assert!(rendered.issues().is_empty());
    let code = expressed.result().code().unwrap();
    // Each original mark invocation keeps exactly one source anchor and its PC.
    // This checks effect provenance, not execution of those operations. Literal
    // producers have no independent emitted action and are not covered here.
    for (index, operation) in code.operations().iter().enumerate() {
        if !matches!(operation.original, diana_decomp::p2::OpKind::Invoke { .. }) {
            continue;
        }
        let origin = Origin::Operation {
            method,
            operation: diana_decomp::p2::OpId(u32::try_from(index).unwrap()),
            pc: operation.pc,
        };
        assert_eq!(
            rendered
                .spans()
                .iter()
                .filter(|s| s.origin == origin)
                .count(),
            1,
            "{name}: operation origin {origin:?}"
        );
    }
    let inspection = expressed.inspection();
    check(inspection.facts(), &rendered);
}

fn nested(facts: &RegionFacts) -> (&LoopPlan, &LoopPlan) {
    assert_eq!(facts.loops().len(), 2);
    let outer = facts
        .loops()
        .iter()
        .max_by_key(|p| p.members().len())
        .unwrap();
    let inner = facts.loops().iter().find(|p| p.id() != outer.id()).unwrap();
    assert!(inner.members().iter().all(|n| outer.members().contains(n)));
    (outer, inner)
}

#[test]
fn private_exit_inside_enclosing_core() {
    inspect("nestedBreak", |facts, rendered| {
        let (outer, inner) = nested(facts);
        assert_eq!(inner.private_exits().len(), 1);
        let added: Vec<_> = inner
            .lexical_members()
            .iter()
            .filter(|n| !inner.members().contains(n))
            .collect();
        assert!(!added.is_empty());
        assert!(added.iter().all(|n| outer.members().contains(n)));
        assert!(!inner.lexical_members().contains(&inner.follow().unwrap()));
        assert!(!rendered.text().contains("goto "), "{}", rendered.text());
        assert!(rendered.text().contains("break;"));
        assert_eq!(rendered.text().matches(".mark(").count(), 1);
    });
}

#[test]
fn private_exit_stops_at_enclosing_follow() {
    inspect("outerBreak", |facts, rendered| {
        let (outer, inner) = nested(facts);
        assert_eq!(inner.private_exits().len(), 1);
        let follow = outer.follow().unwrap();
        assert!(!inner.lexical_members().contains(&follow));
        assert!(facts.links().iter().any(|link| {
            inner.lexical_members().contains(&link.from())
                && !inner.members().contains(&link.from())
                && link.to() == follow
                && matches!(
                    facts.link_plans()[link.id().0 as usize].action(),
                    TransferAction::Break(_)
                )
        }));
        assert!(!rendered.text().contains("goto "), "{}", rendered.text());
        assert!(rendered.text().contains("break R"));
        assert_eq!(rendered.text().matches(".mark(").count(), 1);
    });
}

#[test]
fn private_exit_stops_at_enclosing_continue() {
    inspect("outerContinue", |facts, rendered| {
        let (outer, inner) = nested(facts);
        assert_eq!(inner.private_exits().len(), 1);
        assert!(!inner.lexical_members().contains(&outer.continue_target()));
        assert!(facts.links().iter().any(|link| {
            inner.lexical_members().contains(&link.from())
                && link.to() == outer.continue_target()
                && matches!(facts.link_plans()[link.id().0 as usize].action(), TransferAction::Continue(id) if id == outer.id())
        }));
        assert!(rendered.text().contains("continue R"));
        assert_eq!(rendered.text().matches(".mark(").count(), 1);
    });
}

#[test]
fn shared_exit_remains_shared() {
    inspect("sharedExit", |facts, rendered| {
        let (_, inner) = nested(facts);
        assert!(inner.private_exits().is_empty());
        assert_eq!(inner.lexical_members(), inner.members());
        assert_eq!(rendered.text().matches(".mark(").count(), 1);
    });
}

#[test]
fn sibling_loop_is_not_an_enclosing_scope() {
    inspect("siblingLoop", |facts, rendered| {
        assert_eq!(facts.loops().len(), 2);
        for plan in facts.loops() {
            assert!(plan.private_exits().is_empty());
            assert_eq!(plan.lexical_members(), plan.members());
        }
        assert_eq!(rendered.text().matches(".mark(").count(), 1);
    });
}

#[test]
fn local_handler_keeps_exit_protection() {
    inspect("protectedExit", |facts, rendered| {
        let (_, inner) = nested(facts);
        assert!(inner.private_exits().is_empty());
        assert_eq!(inner.lexical_members(), inner.members());
        assert!(rendered.text().contains("RuntimeException"));
        assert_eq!(rendered.text().matches(".mark(").count(), 1);
    });
}
