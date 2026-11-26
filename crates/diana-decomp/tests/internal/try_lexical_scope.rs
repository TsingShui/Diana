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

#[test]
fn normalizer_keeps_normal_children_and_still_rewrites_their_siblings() {
    use crate::ir::{SemanticFoldError, SemanticFolder};

    struct RecordBlocks(Vec<u32>);
    impl SemanticFolder for RecordBlocks {
        type Error = SemanticFoldError;

        fn preserves_subtree(&self, node: &SemanticNode) -> bool {
            matches!(node, SemanticNode::BasicBlock(block) if block.id.raw() % 2 == 0)
        }

        fn finish_node(&mut self, node: SemanticNode) -> Result<SemanticNode, Self::Error> {
            if let SemanticNode::BasicBlock(block) = &node {
                self.0.push(block.id.raw());
                if block.id.raw() == 3 {
                    return Ok(ordinary(33));
                }
            }
            Ok(node)
        }
    }
    let mut folder = RecordBlocks(Vec::new());
    let sequence = SemanticNode::Sequence((0..7).map(ordinary).collect());
    let SemanticNode::Sequence(nodes) = folder.fold_node(sequence).unwrap() else {
        panic!("expected sequence");
    };
    assert_eq!(folder.0, [1, 3, 5]);
    assert_eq!(ids(&nodes), [0, 1, 2, 33, 4, 5, 6]);
    assert_eq!(ids(&[folder.fold_node(ordinary(8)).unwrap()]), [8]);
    assert_eq!(folder.0, [1, 3, 5]);
}

#[test]
fn flat_sequences_prune_a_no_return_suffix_only_after_phi_lowering() {
    use super::{ReachabilityMode, SemanticNormalization, SourceSemanticNormalizer};
    use crate::ir::SemanticFolder;

    let sequence = SemanticNode::Sequence(vec![
        ordinary(1),
        block(2, InsnType::Invoke, true),
        ordinary(3),
    ]);
    for (normalized, expected) in [
        (
            SemanticNormalization::new(ReachabilityMode::PreserveCfgTopology)
                .rewrite(sequence.clone())
                .unwrap(),
            &[1, 2, 3][..],
        ),
        (
            SemanticNormalization::new(ReachabilityMode::PruneUnreachable)
                .rewrite(sequence.clone())
                .unwrap(),
            &[1, 2][..],
        ),
        (
            SourceSemanticNormalizer.fold_node(sequence).unwrap(),
            &[1, 2][..],
        ),
    ] {
        let SemanticNode::Sequence(nodes) = normalized else {
            panic!("expected the reachable prefix");
        };
        assert_eq!(ids(&nodes), expected);
    }
}

#[test]
fn canonical_cfg_retires_only_if_sites_and_keeps_both_block_identities() {
    use super::SemanticNormalization;
    use crate::ir::{SemanticOperand, SemanticPredicate, SemanticSiteId};

    let mut first = ordinary(6);
    let SemanticNode::BasicBlock(block) = &mut first else {
        unreachable!()
    };
    block.statements[0].site = Some(SemanticSiteId(7));
    let mut body = SemanticNode::If {
        condition: SemanticOperand {
            site: Some(SemanticSiteId(23)),
            value: SemanticPredicate::True,
        },
        then_node: Box::new(first),
        else_node: Some(Box::new(SemanticNode::BasicBlock(SemanticBlock {
            id: BlockId::new(17),
            statements: Vec::new(),
        }))),
    };
    assert!(SemanticNormalization::canonical_cfg(&mut body));
    let SemanticNode::If {
        condition,
        then_node,
        else_node: Some(else_node),
    } = body
    else {
        panic!("both original branches must remain");
    };
    assert_eq!(condition.site, None);
    assert!(matches!(condition.value, SemanticPredicate::True));
    let SemanticNode::BasicBlock(first) = *then_node else {
        unreachable!()
    };
    let SemanticNode::BasicBlock(second) = *else_node else {
        unreachable!()
    };
    assert_eq!(first.id.raw(), 6);
    assert_eq!(first.statements[0].site, Some(SemanticSiteId(7)));
    assert_eq!(second.id.raw(), 17);
    assert!(second.statements.is_empty());
}

#[test]
fn noncanonical_cfg_keeps_sites_for_the_ordinary_normalizer() {
    use super::SemanticNormalization;
    use crate::ir::{SemanticOperand, SemanticPredicate, SemanticSiteId};

    let mut body = SemanticNode::If {
        condition: SemanticOperand {
            site: Some(SemanticSiteId(41)),
            value: SemanticPredicate::False,
        },
        then_node: Box::new(SemanticNode::Sequence(vec![ordinary(2)])),
        else_node: None,
    };
    assert!(!SemanticNormalization::canonical_cfg(&mut body));
    let SemanticNode::If { condition, .. } = body else {
        unreachable!()
    };
    assert_eq!(condition.site, Some(SemanticSiteId(41)));
}
