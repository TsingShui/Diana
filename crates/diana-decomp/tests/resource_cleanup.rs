//! Ownership checks through ordinary Engine delivery, not behavioral execution.
//! Independent scenarios are in quality/fixtures/exceptions/resource-cleanup/README.md.

use diana_decomp::p6::{CatchTarget, RegionFacts, RegionKind, SiteProtection};
use diana_decomp::p9::{Origin, RenderOutcome, Rendered};
use diana_decomp::{
    ByteOffset, ClassDefId, ClassKey, DexId, DexImage, Engine, ImageOrigin, InputId,
};

fn inspect(name: &str, check: impl FnOnce(&RegionFacts, &Rendered)) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/exceptions/resource-cleanup/classes.dex");
    let bytes = std::fs::read(path).expect("resource-cleanup fixture input is required");
    let engine = Engine::open(vec![DexImage {
        bytes: &bytes,
        header_offset: ByteOffset(0),
        origin: ImageOrigin { input: InputId(0) },
    }])
    .unwrap();
    let index = diana_decomp::input::ProgramIndex::open(&bytes).unwrap();
    let descriptor: Vec<_> = "Lfixture/ResourceCleanup;".encode_utf16().collect();
    let class = ClassKey {
        dex: DexId(0),
        class_def: (0..index.class_count())
            .map(ClassDefId)
            .find(|&id| index.class(id).unwrap().descriptor.units().unwrap() == descriptor)
            .expect("original fixture owner"),
    };
    let method = engine
        .class_facts(class)
        .unwrap()
        .resolve_method(
            &name.encode_utf16().collect::<Vec<_>>(),
            &"(I)I".encode_utf16().collect::<Vec<_>>(),
        )
        .unwrap();
    let analysis = engine.analyze_method(class, method).unwrap();
    let diana_decomp::MethodOutcome::Expressed(expressed) = analysis.result.unwrap() else {
        panic!("{name}: P7 unavailable");
    };
    let RenderOutcome::Ready(rendered) = analysis.rendered else {
        panic!("{name}: P9 unavailable");
    };
    assert!(rendered.issues().is_empty());
    let code = expressed.result().code().unwrap();
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
            "{name}: retained call origin {origin:?}"
        );
    }
    check(expressed.inspection().facts(), &rendered);
}

fn caught_sites_are_natural(facts: &RegionFacts) {
    for site in facts.throws() {
        if facts.handler_chains()[site.chain().0 as usize]
            .clauses()
            .iter()
            .any(|clause| matches!(clause.target, CatchTarget::Handler(_)))
        {
            assert_eq!(site.protection(), SiteProtection::Natural, "{site:?}");
        }
    }
}

#[test]
fn loop_owns_private_cleanup_handlers() {
    inspect("loop", |facts, rendered| {
        let [plan] = facts.loops() else {
            panic!("one original loop");
        };
        let protected = facts
            .tries()
            .iter()
            .find(|p| p.body().iter().all(|n| plan.members().contains(n)))
            .expect("established body protection");
        assert!(
            protected
                .members()
                .iter()
                .all(|n| plan.lexical_members().contains(n))
        );
        assert!(!protected.members().contains(&plan.header()));
        assert!(!protected.members().contains(&plan.follow().unwrap()));
        let region = facts
            .regions()
            .iter()
            .find(|r| matches!(r.kind(), RegionKind::Try(id) if *id == protected.id()))
            .expect("selected body try");
        let mut parent = facts.parent(region.id());
        loop {
            let id = parent.expect("try belongs to the selected loop");
            if matches!(facts.regions()[id.0 as usize].kind(), RegionKind::Loop(id) if *id == plan.id())
            {
                break;
            }
            parent = facts.parent(id);
        }
        caught_sites_are_natural(facts);
        assert!(!rendered.text().contains("[exn "), "{}", rendered.text());
        assert!(!rendered.text().contains("goto "), "{}", rendered.text());
    });
}

#[test]
fn nested_cleanup_keeps_both_protection_levels() {
    inspect("nested", |facts, rendered| {
        caught_sites_are_natural(facts);
        assert_eq!(
            facts
                .regions()
                .iter()
                .filter(|r| matches!(r.kind(), RegionKind::Try(_)))
                .count(),
            4,
            "body and close protection for each resource"
        );
        assert!(!rendered.text().contains("[exn "), "{}", rendered.text());
        assert!(!rendered.text().contains("goto "), "{}", rendered.text());
        assert_eq!(rendered.text().matches(".addSuppressed(").count(), 2);
    });
}

#[test]
fn propagation_only_sites_stay_outside_protected_bodies() {
    inspect("nested", |facts, _| {
        for site in facts.throws() {
            if facts.handler_chains()[site.chain().0 as usize]
                .clauses()
                .iter()
                .all(|clause| clause.target == CatchTarget::Uncaught)
            {
                for region in facts.regions() {
                    let RegionKind::Try(id) = region.kind() else {
                        continue;
                    };
                    assert!(
                        !facts.tries()[id.0 as usize].body().contains(&site.node()),
                        "propagation changed by protected-body ownership: {site:?}"
                    );
                }
            }
        }
    });
}

#[test]
fn shared_capture_stays_outside_the_loop() {
    inspect("sharedHandler", |facts, _| {
        let [plan] = facts.loops() else {
            panic!("one original loop");
        };
        assert!(
            facts
                .handler_entries()
                .iter()
                .all(|h| !plan.lexical_members().contains(&h.node()))
        );
        assert!(
            facts
                .regions()
                .iter()
                .any(|r| matches!(r.kind(), RegionKind::Loop(_)))
        );
        caught_sites_are_natural(facts);
    });
}
