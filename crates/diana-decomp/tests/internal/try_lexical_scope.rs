use super::TryLexicalScope;
use crate::ir::{
    BlockId, InsnNode, InsnType, RegionId, SemanticBlock, SemanticCatch, SemanticFinally,
    SemanticLeave, SemanticLeaveKind, SemanticNode, SemanticStatement,
};

fn block(id: u32, kind: InsnType, no_return: bool) -> SemanticNode {
    let mut operation = InsnNode::new(kind, 0);
    operation.payload.no_return = no_return;
    SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(id),
        statements: vec![SemanticStatement::instruction(operation).unwrap()],
    })
}

fn ordinary(id: u32) -> SemanticNode {
    block(id, InsnType::Nop, false)
}

fn returned() -> SemanticNode {
    let root = RegionId::new(0);
    SemanticNode::Leave(SemanticLeave {
        site: None,
        condition: None,
        kind: SemanticLeaveKind::Return(None),
        edge: None,
        origin: None,
        source: root,
        destination: root,
        target: root,
        cleanup: Vec::new(),
    })
}

fn protected(body: SemanticNode, catches: Vec<SemanticNode>, has_finally: bool) -> SemanticNode {
    SemanticNode::Try {
        region: RegionId::new(1),
        body: Box::new(body),
        catches: catches
            .into_iter()
            .map(|body| SemanticCatch {
                region: RegionId::new(2),
                exception_types: Vec::new(),
                exception_value: None,
                body,
            })
            .collect(),
        finally: has_finally.then(|| SemanticFinally {
            region: RegionId::new(3),
            body: Box::new(ordinary(55)),
        }),
    }
}

fn merged_prefix(nodes: Vec<SemanticNode>) -> (Vec<SemanticNode>, Vec<SemanticNode>) {
    let SemanticNode::Sequence(nodes) = TryLexicalScope::extend(SemanticNode::Sequence(nodes))
    else {
        panic!("protected and outside code");
    };
    let mut nodes = nodes.into_iter();
    let Some(SemanticNode::Try { body, .. }) = nodes.next() else {
        panic!("protected region")
    };
    let body = match *body {
        SemanticNode::Sequence(children) => children,
        single => vec![single],
    };
    (body, nodes.collect())
}

fn ids(nodes: &[SemanticNode]) -> Vec<u32> {
    nodes
        .iter()
        .map(|node| match node {
            SemanticNode::BasicBlock(block) => block.id.raw(),
            _ => panic!("expected a block"),
        })
        .collect()
}

#[test]
fn appends_each_normal_successor_in_order_and_stops_at_a_return() {
    let (body, rest) = merged_prefix(vec![
        protected(
            ordinary(1),
            vec![returned(), block(7, InsnType::Invoke, true)],
            false,
        ),
        ordinary(2),
        ordinary(3),
        returned(),
        ordinary(4),
        ordinary(5),
    ]);
    let (last, prefix) = body.split_last().unwrap();
    assert_eq!(ids(prefix), [1, 2, 3]);
    assert!(
        matches!(last, SemanticNode::Leave(exit) if matches!(exit.kind, SemanticLeaveKind::Return(None)))
    );
    assert_eq!(ids(&rest), [4, 5]);
}

#[test]
fn throwing_successor_and_later_statements_stay_outside() {
    let (body, rest) = merged_prefix(vec![
        protected(ordinary(1), vec![returned()], false),
        ordinary(2),
        block(3, InsnType::Invoke, false),
        ordinary(4),
    ]);
    assert_eq!(ids(&body), [1, 2]);
    assert_eq!(ids(&rest), [3, 4]);
}

#[test]
fn no_return_body_fallthrough_handler_or_finally_keeps_the_entire_tail_outside() {
    for first in [
        protected(block(1, InsnType::Invoke, true), vec![returned()], false),
        protected(ordinary(1), vec![returned(), ordinary(7)], false),
        protected(ordinary(1), vec![returned()], true),
    ] {
        let (body, rest) = merged_prefix(vec![first, ordinary(2), ordinary(3)]);
        assert_eq!(ids(&body), [1]);
        assert_eq!(ids(&rest), [2, 3]);
    }
}
