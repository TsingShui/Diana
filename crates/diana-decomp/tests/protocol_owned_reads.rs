//! Fixture-owned expectations for constructor reads and implicit ABI occurrences.

use diana_decomp::p3::UseSite;
use diana_decomp::p7::{ExprKind, MethodCode, ReferenceDeclaration, StorageId};
use diana_decomp::p9::{Origin, Rendered};
#[path = "support/inspection.rs"]
mod inspection;

fn retained_reads(code: &MethodCode, rendered: &Rendered) {
    let boxes: Vec<_> = code
        .constructions()
        .iter()
        .filter(|group| {
            let ExprKind::New { reference, .. } =
                code.expressions()[group.expression.0 as usize].kind
            else {
                return false;
            };
            let Ok(ReferenceDeclaration::Type { type_use }) =
                &code.references()[reference.0 as usize].declaration
            else {
                return false;
            };
            code.types()[type_use.0 as usize].units
                == "Lfixture/ReadBox;".encode_utf16().collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(
        boxes.len(),
        1,
        "one original construction in this source body"
    );
    let group = boxes[0];
    let ExprKind::New { arguments, .. } = &code.expressions()[group.expression.0 as usize].kind
    else {
        unreachable!()
    };
    assert_eq!(arguments.len(), 2);
    for (index, &argument) in arguments.iter().enumerate() {
        let expression = &code.expressions()[argument.0 as usize];
        let origin = expression
            .origin
            .as_ref()
            .expect("original constructor read");
        assert!(
            matches!(origin.site, UseSite::Operation { operation, argument }
            if operation == group.initialization && argument as usize == index + 1)
        );
        if expression.evaluation.is_none()
            && let ExprKind::Read { mut storage, .. } | ExprKind::Snapshot { mut storage } =
                expression.kind
        {
            // The existing final protocol product still supplies selected copy
            // forwarding; original read origins keep the pre-selection storage.
            if let Some(forward) = code.emission().coroutine_loop().and_then(|plan| {
                plan.forwarded_copies
                    .iter()
                    .find(|copy| copy.destination == storage)
            }) {
                storage = forward.source;
            }
            assert!(
                !code.emission().hidden_local(storage),
                "an actual constructor operand keeps its declaration: {storage:?}, expression {expression:?}"
            );
        }
    }
    let continuation = if let Some(plan) = code.emission().coroutine_loop() {
        plan.continuation
    } else {
        code.emission().coroutine_flow().unwrap().sites[0].continuation
    };
    let local = code.expressions()[continuation.0 as usize]
        .origin
        .as_ref()
        .unwrap()
        .read
        .unwrap()
        .local()
        .unwrap();
    assert!(
        code.emission().hidden_local(StorageId(local.0)),
        "an ABI-only continuation does not add an executable local"
    );
    for operation in code.operations() {
        assert_eq!(
            rendered
                .spans()
                .iter()
                .filter(|span| matches!(span.origin,
            Origin::Operation { operation: original, pc, .. }
                if original == operation.source && pc == operation.pc))
                .count(),
            1,
            "missing original operation {:?} at {:?}: {:?}",
            operation.source,
            operation.pc,
            operation.original
        );
    }
}

#[test]
fn a_loop_keeps_constructor_storage_after_immediate_or_resumed_completion() {
    inspection::inspect(
        "kotlin/protocol-owned-reads",
        "Lfixture/OwnedLoop;",
        "total",
        "(IILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            assert!(facts.coroutine_loop().is_some());
            retained_reads(code, rendered);
            let text = rendered.text();
            assert!(text.contains("while (") && text.contains("ReadBox("));
            assert_eq!(text.matches("OwnedLoop.observe(").count(), 1);
            assert!(text.find("ReadBox(").unwrap() < text.find("OwnedLoop.observe(").unwrap());
        },
    );
}

#[test]
fn a_multi_site_body_keeps_constructor_reads_before_its_early_return() {
    inspection::inspect(
        "kotlin/protocol-owned-reads",
        "Lfixture/OwnedFlow;",
        "walk",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            assert_eq!(facts.coroutine_flow().unwrap().sites().len(), 3);
            retained_reads(code, rendered);
            let text = rendered.text();
            assert_eq!(text.matches("OwnedFlow.observe(").count(), 1);
            assert!(text.find("ReadBox(").unwrap() < text.find("OwnedFlow.observe(").unwrap());
            let observe = text.find("OwnedFlow.observe(").unwrap();
            assert!(text[observe..].contains(" == 0)"));
        },
    );
}
