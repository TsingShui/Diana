use super::*;
use crate::ir::{ArgType, BlockId, RegisterArg, SemanticBlock, SemanticLeave};

fn protected(owner: u32, handler: u32, register: u32, handler_body: SemanticNode) -> SemanticNode {
    SemanticNode::Try {
        region: RegionId::new(owner),
        body: Box::new(SemanticNode::Empty),
        catches: vec![SemanticCatch {
            region: RegionId::new(handler),
            exception_types: vec![ArgType::throwable()],
            exception_value: Some(RegisterArg::new(register, ArgType::throwable())),
            body: handler_body,
        }],
        finally: None,
    }
}

#[test]
fn collection_preserves_key_order_and_the_first_matching_handler_body() {
    let block = |id| {
        SemanticNode::BasicBlock(SemanticBlock {
            id: BlockId::new(id),
            statements: Vec::new(),
        })
    };
    let root = SemanticNode::Sequence(vec![
        protected(1, 9, 0, block(41)),
        protected(2, 5, 0, block(51)),
        protected(3, 9, 0, block(42)),
        protected(4, 5, 0, block(52)),
        protected(5, 3, 0, block(61)),
    ]);
    let families = EnvelopeFamilies::collect(&root);
    let summary: Vec<_> = families
        .iter()
        .map(|family| {
            let catch = &family.envelope.catches[0];
            let SemanticNode::BasicBlock(block) = &catch.body else {
                panic!("handler body")
            };
            (
                catch.region.raw(),
                family.regions.iter().map(|id| id.raw()).collect::<Vec<_>>(),
                block.id.raw(),
            )
        })
        .collect();
    // The collector visits the sequence from the stack's last child first.
    assert_eq!(summary, [(5, vec![2, 4], 52), (9, vec![1, 3], 42)]);
}

#[test]
fn repeated_families_require_identical_exception_values_and_free_labels() {
    let owner = RegionId::new(16);
    let free_label = SemanticNode::Leave(SemanticLeave {
        site: None,
        condition: None,
        kind: SemanticLeaveKind::BreakLabel(SemanticLabel::block(owner, BlockId::new(8))),
        edge: None,
        origin: None,
        source: owner,
        destination: owner,
        target: owner,
        cleanup: Vec::new(),
    });
    let root = SemanticNode::Sequence(vec![
        protected(1, 6, 0, free_label),
        protected(2, 6, 0, SemanticNode::Empty),
        protected(3, 6, 0, SemanticNode::Empty),
        protected(4, 7, 2, SemanticNode::Empty),
        protected(5, 7, 1, SemanticNode::Empty),
    ]);
    assert!(EnvelopeFamilies::collect(&root).is_empty());
}
