//! Outside construction observes a checksum captured before the final load.
use diana_decomp::p7::{EmissionLayout, ExprKind, ReferenceDeclaration, StorageId};
#[path = "support/inspection.rs"]
mod inspection;

#[test]
fn consumed_candidates_retain_reentry_facts_and_outside_copy_observers() {
    inspection::inspect(
        "kotlin/protocol-candidate-lifetimes",
        "Lfixture/IntervalLoop;",
        "total",
        "(IILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            let protocol = facts.protocol().unwrap();
            assert_eq!(protocol.calls.len(), 1);
            let call = &protocol.calls[0];
            assert_eq!(call.state, 1);
            assert_eq!(call.spills.len(), 4);
            assert!(
                !call.resume.contains(&code.operations()[0].node),
                "entry prefix is not a resume corridor"
            );
            for &(_, get) in &call.spills {
                assert!(
                    call.resume
                        .contains(&code.operations()[get.0 as usize].node),
                    "resume correspondence excludes unrelated ABI nodes"
                );
            }
            let mut owners = vec![0; facts.nodes().len()];
            for (node, _) in inspection::protocol_leaves(code) {
                owners[node.0 as usize] += 1;
            }
            assert_eq!(owners, vec![1; facts.nodes().len()]);
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
                        code.references()[reference.0 as usize].declaration
                    else {
                        return false;
                    };
                    code.types()[type_use.0 as usize].units
                        == "Lfixture/ReadBox;".encode_utf16().collect::<Vec<_>>()
                })
                .collect();
            assert_eq!(
                boxes.len(),
                2,
                "body and tail each construct an observed box"
            );
            let tail = boxes
                .into_iter()
                .max_by_key(|group| group.initialization.0)
                .unwrap();
            let iteration = protocol.iteration.as_ref().unwrap();
            let EmissionLayout::Loop { body, .. } =
                code.controls()[iteration.control.0 as usize].layout
            else {
                panic!("common loop");
            };
            let mut work = vec![body];
            let mut repeated = Vec::new();
            while let Some(id) = work.pop() {
                let control = &code.controls()[id.0 as usize];
                if let EmissionLayout::Block(node) = control.layout {
                    repeated.push(node);
                }
                work.extend_from_slice(control.children());
            }
            assert!(!repeated.contains(&code.operations()[tail.initialization.0 as usize].node));
            for read in &tail.arguments {
                let local = StorageId(read.local().unwrap().0);
                assert!(!code.emission().hidden_local(local));
                assert!(
                    code.emission()
                        .protocol()
                        .unwrap()
                        .forwarded_copies
                        .iter()
                        .all(|copy| copy.destination != local),
                    "an outside observer retains its original storage"
                );
            }
            let source = rendered
                .text()
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                source.contains("while (")
                    && !source.contains("switch (")
                    && !source.contains("graph<")
            );
            assert_eq!(source.matches("ReadBox(").count(), 2);
            assert_eq!(source.matches("IntervalLoop.observe(").count(), 2);
            assert_eq!(source.matches("IntervalLoop.record(").count(), 3);
            assert!(
                source.rfind("IntervalLoop.observe(").unwrap()
                    < source.rfind("IntervalLoop.record(").unwrap()
            );
            assert!(
                source.rfind("IntervalLoop.record(").unwrap()
                    < source.rfind("Boxing.boxInt(").unwrap()
            );
        },
    );
}
