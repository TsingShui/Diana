//! Acyclic single-site protocols use the same producer as cyclic/multi-site ones.
use diana_decomp::p2::{FieldAccess, OpKind};
use diana_decomp::p7::CopyStatement;
#[path = "support/inspection.rs"]
mod inspection;

#[test]
fn one_reentry_keeps_saved_seed_preparation_and_both_branch_results() {
    inspection::inspect(
        "kotlin/protocol-single-site-core",
        "Lfixture/OneSite;",
        "choose",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            let protocol = facts.protocol().expect("closed one-site protocol");
            assert!(protocol.iteration.is_none(), "acyclic body needs no loop");
            assert_eq!(protocol.calls.len(), 1);
            let call = &protocol.calls[0];
            assert_eq!(call.state, 1);
            assert_eq!(call.spills.len(), 1);
            let (put, get) = call.spills[0];
            let OpKind::Field {
                access: FieldAccess::Put,
                field: saved,
                ..
            } = code.operations()[put.0 as usize].original
            else {
                panic!("saved seed field");
            };
            let OpKind::Field {
                access: FieldAccess::Get,
                field: restored,
                ..
            } = code.operations()[get.0 as usize].original
            else {
                panic!("restored seed field");
            };
            assert_eq!(saved, restored);
            assert!(
                call.resume
                    .contains(&code.operations()[get.0 as usize].node)
            );
            let delivery = &code.emission().transfers()[call.normal.0 as usize];
            let assignments = delivery
                .copies
                .iter()
                .flat_map(|id| &code.copies()[id.0 as usize].steps)
                .filter(|step| matches!(step, CopyStatement::Assign { .. }))
                .count();
            assert_eq!(
                assignments, 2,
                "normal reply and saved seed each retain delivery"
            );
            let mut owners = vec![0; facts.nodes().len()];
            for (node, _) in inspection::protocol_leaves(code) {
                owners[node.0 as usize] += 1;
            }
            assert_eq!(owners, vec![1; facts.nodes().len()]);
            let text = rendered.text();
            assert!(text.find("OneSite.prepare(").unwrap() < text.find(".I$0 =").unwrap());
            assert!(text.find(".I$0 =").unwrap() < text.find(".label = 1").unwrap());
            assert!(text.find(".label = 1").unwrap() < text.find("OneSite.load(").unwrap());
            let source = text
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(source.contains("suspend java.lang.Object choose(int"));
            assert!(
                !source.contains("switch (")
                    && !source.contains("graph<")
                    && !source.contains("while (")
                    && !source.contains("goto ")
            );
            assert_eq!(source.matches("OneSite.load(").count(), 1);
            assert_eq!(source.matches("OneSite.record(").count(), 1);
            assert_eq!(source.matches("ReadBox(").count(), 1);
            assert!(source.find("OneSite.record(").unwrap() < source.find(" < 0)").unwrap());
            assert!(source.find(" < 0)").unwrap() < source.find("ReadBox(").unwrap());
            assert!(source.contains("Boxing.boxInt(-1)") && source.contains(" + "));
        },
    );
}
