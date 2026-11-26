use super::*;
use crate::ir::{
    ArgType, LiteralArg, SemanticCatch, SemanticExpression, SemanticFinally, SemanticOperand,
    SemanticSwitchCase,
};

fn exit(source: u32, kind: SemanticLeaveKind) -> SemanticNode {
    SemanticNode::Leave(SemanticLeave {
        site: None,
        condition: None,
        kind,
        edge: None,
        origin: None,
        source: RegionId::new(source),
        destination: RegionId::new(0),
        target: RegionId::new(1),
        cleanup: vec![RegionId::new(source + 3)],
    })
}

fn throw() -> SemanticLeaveKind {
    SemanticLeaveKind::Throw(SemanticExpression::Literal(LiteralArg::new(
        0,
        ArgType::throwable(),
    )))
}

fn switch(default: bool, arms: Vec<SemanticNode>) -> SemanticNode {
    SemanticNode::Switch {
        region: Some(RegionId::new(1)),
        selector: SemanticOperand::new(SemanticExpression::Literal(LiteralArg::new(
            1,
            ArgType::INT,
        ))),
        cases: arms
            .into_iter()
            .enumerate()
            .map(|(index, body)| SemanticSwitchCase {
                values: vec![index as i32],
                is_default: default && index == 0,
                body,
            })
            .collect(),
    }
}

#[test]
fn projected_returns_merge_across_cleanup_origins_while_detailed_outcomes_keep_both() {
    let node = switch(
        true,
        vec![
            exit(1, SemanticLeaveKind::Return(None)),
            exit(2, SemanticLeaveKind::Return(None)),
        ],
    );
    let detail = SemanticCompletion::analyze(&node);
    assert_eq!(detail.abrupt.iter().count(), 2);
    let control = ControlCompletion::analyze(&node);
    assert_eq!(
        control.abrupt.iter().copied().collect::<Vec<_>>(),
        [SemanticTransfer::Return]
    );
    assert!(!control.normal);
    assert_eq!(
        control,
        ControlCompletion::analyze(&SemanticNode::Empty).void_method()
    );
}

#[test]
fn normal_query_and_projected_completions_preserve_switch_break_and_finally_precedence() {
    for has_default in [false, true] {
        for skip_arm in [false, true] {
            for catches in [false, true] {
                for cleanup_throws in [false, true] {
                    let first = exit(2, SemanticLeaveKind::Break);
                    let first = SemanticNode::If {
                        condition: SemanticOperand::new(if skip_arm {
                            SemanticPredicate::False
                        } else {
                            SemanticPredicate::True
                        }),
                        then_node: Box::new(first),
                        else_node: Some(Box::new(exit(3, throw()))),
                    };
                    let body = switch(
                        has_default,
                        vec![first, exit(4, SemanticLeaveKind::Return(None))],
                    );
                    let node = SemanticNode::Try {
                        region: RegionId::new(5),
                        body: Box::new(body),
                        catches: if catches {
                            vec![SemanticCatch {
                                region: RegionId::new(6),
                                exception_types: Vec::new(),
                                exception_value: None,
                                body: SemanticNode::Empty,
                            }]
                        } else {
                            Vec::new()
                        },
                        finally: Some(SemanticFinally {
                            region: RegionId::new(7),
                            body: Box::new(if cleanup_throws {
                                exit(7, throw())
                            } else {
                                SemanticNode::Empty
                            }),
                        }),
                    };
                    let normal = !cleanup_throws && (!has_default || !skip_arm || catches);
                    let mut abrupt = BTreeSet::new();
                    if cleanup_throws || skip_arm {
                        abrupt.insert(SemanticTransfer::Throw);
                    }
                    if !cleanup_throws {
                        abrupt.insert(SemanticTransfer::Return);
                    }
                    let control = ControlCompletion::analyze(&node);
                    let full = SemanticCompletion::analyze(&node);
                    assert_eq!(ControlCompletion::normally_completes(&node), normal);
                    assert_eq!(control.normal, normal);
                    assert_eq!(
                        control.abrupt.iter().copied().collect::<BTreeSet<_>>(),
                        abrupt
                    );
                    assert_eq!(
                        full.abrupt
                            .iter()
                            .map(AbruptCompletion::transfer)
                            .collect::<BTreeSet<_>>(),
                        abrupt
                    );
                    assert_eq!(full.normal, normal);
                }
            }
        }
    }
}
