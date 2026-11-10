//! Comparison/parameter entry uses the ordinary site and copy correspondence.
use diana_decomp::p9::Origin;
#[path = "support/inspection.rs"]
mod inspection;

#[test]
fn two_parameter_reentries_keep_captures_shared_marker_and_negative_exit() {
    inspection::inspect(
        "kotlin/protocol-entry-associations",
        "Lfixture/AssociatedBody;",
        "invokeSuspend",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        |facts, code, rendered| {
            let protocol = facts.protocol().unwrap();
            assert_eq!(
                protocol.calls.iter().map(|s| s.state).collect::<Vec<_>>(),
                [5, 2]
            );
            assert_eq!(
                protocol
                    .calls
                    .iter()
                    .map(|s| s.spills.len())
                    .collect::<Vec<_>>(),
                [1, 2]
            );
            assert_eq!(
                facts.links()[protocol.calls[0].suspended.0 as usize].to(),
                facts.links()[protocol.calls[1].suspended.0 as usize].to()
            );
            assert!(protocol.calls.iter().all(|s| s.publication.is_none()));
            let mut owners = vec![0; facts.nodes().len()];
            for (node, _) in inspection::protocol_leaves(code) {
                owners[node.0 as usize] += 1;
            }
            assert_eq!(owners, vec![1; facts.nodes().len()]);
            for operation in code.operations() {
                assert_eq!(rendered.spans().iter().filter(|span| matches!(span.origin, Origin::Operation { operation: source, .. } if source == operation.source)).count(), 1);
            }
            let source = rendered
                .text()
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(source.contains("suspend java.lang.Object invokeSuspend(java.lang.Object p0)"));
            assert_eq!(source.matches("AssociationEffects.load(").count(), 2);
            assert_eq!(source.matches("this.seed").count(), 1);
            assert!(source.contains("Boxing.boxInt(-1)"));
            assert_eq!(source.matches("AssociationEffects.record(").count(), 1);
            assert_eq!(source.matches("AssociationEffects.observe(").count(), 1);
            assert!(
                source.find("AssociationEffects.prepare(").unwrap()
                    < source.find("AssociationEffects.load(").unwrap()
            );
            assert!(
                source.find("AssociationEffects.record(").unwrap() < source.find(" < 0)").unwrap()
            );
            assert!(
                source.find(" < 0)").unwrap() < source.rfind("AssociationEffects.load(").unwrap()
            );
            assert!(
                source.rfind("AssociationEffects.load(").unwrap()
                    < source.find("AssociationEffects.observe(").unwrap()
            );
            assert!(!source.contains("goto ") && !source.contains("return v2;"));
            assert_eq!(rendered.text().matches("resume state=5").count(), 1);
            assert_eq!(rendered.text().matches("resume state=2").count(), 1);
            assert!(rendered.text().contains("invalid state"));
        },
    );
}

#[test]
fn resume_only_effect_keeps_the_complete_original_member_explicit() {
    inspection::inspect(
        "kotlin/protocol-entry-associations",
        "Lfixture/AssociatedBadResume;",
        "invokeSuspend",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        |facts, code, rendered| {
            assert!(facts.protocol().is_none());
            assert!(code.emission().protocol().is_none());
            assert_eq!(
                rendered.text().matches("AssociationEffects.load(").count(),
                2
            );
            assert_eq!(
                rendered
                    .text()
                    .matches("AssociationEffects.record(")
                    .count(),
                2
            );
            assert_eq!(
                rendered
                    .text()
                    .matches("AssociationEffects.observe(")
                    .count(),
                1
            );
            assert!(!rendered.text().contains("suspend java.lang.Object"));
            assert!(rendered.text().contains("invalid state"));
            assert!(rendered.text().contains("throwOnFailure(p0)"));
        },
    );
}

#[test]
fn two_publications_keep_one_shared_marker_return_owner() {
    for owner in [
        "Lfixture/AssociatedPublications;",
        "Lfixture/AssociatedSwitchPublications;",
    ] {
        inspection::inspect(
            "kotlin/protocol-entry-associations",
            owner,
            "invokeSuspend",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            |facts, code, rendered| {
                let protocol = facts.protocol().unwrap();
                assert_eq!(protocol.calls.len(), 2);
                assert!(protocol.calls.iter().all(|s| s.publication.is_some()));
                let mut owners = vec![0; facts.nodes().len()];
                for (node, _) in inspection::protocol_leaves(code) {
                    owners[node.0 as usize] += 1;
                }
                assert_eq!(owners, vec![1; facts.nodes().len()]);
                for site in &protocol.calls {
                    let publication = code.operations()[site.call.0 as usize]
                        .publication
                        .as_ref()
                        .unwrap();
                    assert_eq!(publication.transfer.0, site.normal.0);
                    assert_eq!(
                        code.copies()[publication.copy.0 as usize].emission,
                        diana_decomp::p7::CopyEmission::Publication(site.call)
                    );
                    assert!(
                        code.emission()
                            .transfers()
                            .iter()
                            .all(|transfer| !transfer.copies.contains(&publication.copy))
                    );
                }
                assert_eq!(
                    rendered.text().matches("AssociationEffects.load(").count(),
                    2
                );
            },
        );
    }
}
