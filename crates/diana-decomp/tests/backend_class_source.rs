//! End-to-end check of the pluggable backend driven by original P0/P1 facts.
//! Input: atoms/expr-arith/classes.dex, prepared beside its sources.

use diana_decomp::{
    ByteOffset, ClassDefId, ClassKey, DexId, DexImage, Engine, ImageOrigin, InputId, Request,
};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../quality/fixtures")
            .join(name)
            .join("classes.dex"),
    )
    .expect("prepared input is required")
}

#[test]
fn the_backend_recovers_a_class_from_original_facts() {
    let bytes = fixture("atoms/expr-arith");
    let index = diana_decomp::input::ProgramIndex::open(&bytes).unwrap();
    let mut class = None;
    for ordinal in 0..index.class_count() {
        let key = ClassDefId(ordinal);
        let facts = index.class_facts_cached(key).unwrap();
        if String::from_utf16(facts.descriptor().units()).unwrap() == "Lfixture/ExprArith;" {
            class = Some(key);
        }
    }
    let class = class.expect("fixture class");
    let engine = Engine::open(vec![DexImage {
        bytes: &bytes,
        header_offset: ByteOffset(0),
        origin: ImageOrigin { input: InputId(0) },
    }])
    .unwrap();
    let source = engine
        .class_source(Request {
            class: ClassKey {
                dex: DexId(0),
                class_def: class,
            },
            method: None,
        })
        .unwrap();
    assert!(
        source.text.contains("class ExprArith"),
        "backend produced no class: {}",
        source.text
    );
    assert!(source.text.contains("add"), "{}", source.text);
    assert!(source.methods.iter().all(|method| method.failure.is_none()));
}
