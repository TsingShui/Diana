//! Shared ordinary-Engine inspection for prepared fixture checks.

use diana_decomp::p2::OpKind;
use diana_decomp::p6::RegionFacts;
use diana_decomp::p7::MethodCode;
use diana_decomp::p9::{Origin, RenderOutcome, Rendered};
use diana_decomp::{
    ByteOffset, ClassDefId, ClassKey, DexId, DexImage, Engine, ImageOrigin, InputId,
};

pub(super) fn inspect(
    fixture: &str,
    owner: &str,
    name: &str,
    prototype: &str,
    check: impl FnOnce(&RegionFacts, &MethodCode, &Rendered),
) {
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../quality/fixtures")
            .join(fixture)
            .join("classes.dex"),
    )
    .expect("prepared input is required");
    let index = diana_decomp::input::ProgramIndex::open(&bytes).unwrap();
    let owner: Vec<_> = owner.encode_utf16().collect();
    let class = ClassKey {
        dex: DexId(0),
        class_def: (0..index.class_count())
            .map(ClassDefId)
            .find(|&id| index.class(id).unwrap().descriptor.units().unwrap() == owner)
            .expect("original fixture owner"),
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
        panic!("{name}: expression unavailable");
    };
    let RenderOutcome::Ready(rendered) = analysis.rendered else {
        panic!("{name}: rendering unavailable");
    };
    assert!(rendered.issues().is_empty());
    let code = expressed.result().code().unwrap();
    for op in code.operations() {
        if matches!(op.original, OpKind::Invoke { .. } | OpKind::MoveException) {
            let origin = Origin::Operation {
                method,
                operation: op.source,
                pc: op.pc,
            };
            assert_eq!(
                rendered
                    .spans()
                    .iter()
                    .filter(|s| s.origin == origin)
                    .count(),
                1,
                "{name}: one original call/capture anchor {origin:?}"
            );
        }
    }
    check(expressed.inspection().facts(), code, &rendered);
}

/// Traverse the delivered static roles, including ABI and origin attachments.
/// This independently checks ownership on fixtures; it runs no production solver.
#[allow(dead_code)]
pub(super) fn protocol_leaves(
    code: &MethodCode,
) -> Vec<(diana_decomp::p6::NodeId, diana_decomp::p7::SourceMode)> {
    use diana_decomp::p7::{EmissionLayout, SourceMode};
    let mut leaves = Vec::new();
    let mut work = vec![(code.root().unwrap(), SourceMode::Code)];
    while let Some((id, mode)) = work.pop() {
        let control = &code.controls()[id.0 as usize];
        let mode = if let EmissionLayout::Scope { mode, .. } = control.layout {
            mode
        } else {
            mode
        };
        if let EmissionLayout::Block(node) = control.layout {
            leaves.push((node, mode));
        }
        for &child in control.children() {
            work.push((child, mode));
        }
        for &child in &control.source_only {
            work.push((child, SourceMode::Origins));
        }
    }
    leaves
}
