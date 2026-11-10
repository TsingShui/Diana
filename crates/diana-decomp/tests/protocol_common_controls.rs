//! A single reentry with effectful preparation and a branching construction tail.
use diana_decomp::p7::{CopyEmission, SourceMode};
use diana_decomp::p9::Origin;
#[path = "support/inspection.rs"]
mod inspection;

#[test]
fn protocol_scopes_share_one_control_arena_and_one_original_leaf_placement() {
    inspection::inspect(
        "kotlin/protocol-common-controls",
        "Lfixture/UnifiedBody;",
        "invokeSuspend",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        |facts, code, rendered| {
            assert!(facts.protocol().is_some());
            assert_ne!(facts.root(), facts.control_root());
            assert_eq!(code.root().unwrap().0, facts.control_root().unwrap().0);
            assert_eq!(code.controls().len(), facts.controls().len());
            let leaves = inspection::protocol_leaves(code);
            let mut ownership = vec![0; facts.nodes().len()];
            let mut modes = [false; 3];
            for (node, mode) in leaves {
                ownership[node.0 as usize] += 1;
                modes[match mode {
                    SourceMode::Code => 0,
                    SourceMode::Abi => 1,
                    SourceMode::Origins => 2,
                }] = true;
            }
            assert_eq!(modes, [true; 3]);
            assert_eq!(ownership, vec![1; facts.nodes().len()]);
            for operation in code.operations() {
                assert_eq!(
                    rendered
                        .spans()
                        .iter()
                        .filter(|span| matches!(span.origin,
                    Origin::Operation { operation: source, pc, .. }
                    if source == operation.source && pc == operation.pc))
                        .count(),
                    1
                );
            }
        },
    );
}

#[test]
fn result_publication_and_state_barrier_keep_the_shared_tail_after_success() {
    inspection::inspect(
        "kotlin/protocol-common-controls",
        "Lfixture/UnifiedBody;",
        "invokeSuspend",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        |facts, code, rendered| {
            let original = facts.suspensions()[0];
            let protocol = code.emission().protocol().unwrap();
            assert_eq!(protocol.calls.len(), 1);
            assert_eq!(protocol.calls[0].call, original.call());
            let operation = &code.operations()[original.call().0 as usize];
            let publication = operation.publication.as_ref().unwrap();
            assert_eq!(publication.destination.0, original.merged().0);
            assert_eq!(publication.copy.0, original.normal_copy().0);
            assert_eq!(publication.transfer.0, original.links()[0].0);
            assert_eq!(
                code.copies()[publication.copy.0 as usize].emission,
                CopyEmission::Publication(original.call())
            );
            assert!(
                code.emission()
                    .transfers()
                    .iter()
                    .all(|transfer| !transfer.copies.contains(&publication.copy))
            );
            let text = rendered.text();
            let store = rendered
                .spans()
                .iter()
                .find_map(|span| {
                    matches!(span.origin,
                Origin::Operation { operation, .. } if operation == original.state_store())
                    .then_some(span.start.0)
                })
                .unwrap();
            assert!(text.find("ProtocolEffects.prepare(").unwrap() < store);
            assert!(store < text.find("ProtocolEffects.load(").unwrap());
            let source = text
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(source.contains("suspend java.lang.Object invokeSuspend(java.lang.Object p0)"));
            assert_eq!(source.matches("ProtocolEffects.load(").count(), 1);
            assert_eq!(source.matches("ProtocolEffects.record(").count(), 1);
            assert_eq!(source.matches("ReadBox(").count(), 1);
            assert!(
                source.find("ProtocolEffects.record(").unwrap() < source.find(" < 0)").unwrap()
            );
            assert!(source.find(" < 0)").unwrap() < source.find("ReadBox(").unwrap());
            assert!(source.contains("Boxing.boxInt(-1)"));
            assert!(!source.contains("when (") && !source.contains("goto "));
            assert!(
                text.lines()
                    .any(|line| line.trim_start().starts_with("// ABI:")
                        && line.contains("ResultKt.throwOnFailure(p0)"))
            );
            assert!(text.contains(r#"IllegalStateException("invalid state")"#));
        },
    );
}
