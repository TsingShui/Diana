use super::*;
use crate::ir::{
    ArgType, BlockId, InsnNode, InsnType, RegisterArg, SemanticBlock, SemanticNode, SemanticSiteId,
    SemanticStatement,
};

fn register(number: u32) -> RegisterArg {
    let mut register = RegisterArg::new(number, ArgType::INT);
    register.code_var = Some(number);
    register
}

fn negation(target: u32) -> InsnNode {
    let mut operation = InsnNode::new(InsnType::Neg, 1);
    operation.result = Some(register(target));
    operation.args.push(InsnArg::lit(7, ArgType::INT));
    operation
}

fn store(value: u32) -> InsnNode {
    let mut operation = InsnNode::new(InsnType::Sput, 1);
    operation.args.push(InsnArg::Reg(register(value)));
    operation
}

fn block(operations: impl IntoIterator<Item = InsnNode>) -> SemanticNode {
    SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements: operations
            .into_iter()
            .enumerate()
            .map(|(site, mut op)| {
                op.id = crate::ir::InstructionId::new(site);
                let mut statement = SemanticStatement::instruction(op).unwrap();
                statement.site = Some(SemanticSiteId(site as u64));
                statement
            })
            .collect(),
    })
}

#[test]
fn numbered_occurrences_keep_original_and_alias_order_and_share_unaffected_uses() {
    let negate_input = |target| {
        let mut op = negation(target);
        op.args[0] = InsnArg::Reg(register(100));
        op
    };
    let root = block([
        negate_input(54),
        negate_input(3),
        negate_input(22),
        store(22),
        store(99),
        store(54),
        store(3),
        store(3),
    ]);
    let graph = ValueFlowGraph::build_source(&root, &BTreeSet::new()).unwrap();
    let facts = SparseValueFacts::analyze(&graph).unwrap();
    let key = |n| SsaVar::new(n, 0);
    assert_eq!(
        facts.uses_with_aliases.keys().copied().collect::<Vec<_>>(),
        [key(54)]
    );
    let events = |n| {
        facts
            .uses_of(key(n))
            .iter()
            .map(|usage| usage.event)
            .collect::<Vec<_>>()
    };
    assert_eq!(events(54), [5, 6, 7, 3]);
    assert_eq!(events(3), [6, 7]);
    assert_eq!(events(22), [3]);
    assert!(std::ptr::eq(facts.uses_of(key(99)), &graph.uses[&key(99)]));
}

#[test]
fn coalesced_classes_use_their_last_event_and_preserve_incoming_bindings() {
    let root = block([negation(8), negation(4), negation(8), negation(2)]);
    let bound = SsaVar::new(4, 0);
    let graph = ValueFlowGraph::build_source(&root, &BTreeSet::from([bound])).unwrap();
    let classes = graph.classes_by_last_definition();
    let key_order: Vec<_> = classes
        .iter()
        .map(|class| class.definitions[0].key)
        .collect();
    assert_eq!(key_order, [bound, SsaVar::new(8, 0), SsaVar::new(2, 0)]);
    assert_eq!(classes[1].definitions.len(), 2);
    let canonical = graph.canonical_values(&classes);
    assert!(!canonical.contains_key(&bound));
    for value in [8, 2] {
        assert_eq!(canonical[&SsaVar::new(value, 0)].literal_value(), Some(-7));
    }
}

#[test]
fn operand_versions_use_ssa_identity_only_after_flow_is_available() {
    use crate::ir::analysis::{SemanticFlowPoint, SsaValueGraph};
    use crate::ir::{block::Block, CFG};
    let key = SsaVar::new(3, 2);
    let mut result = RegisterArg::new_ssa(3, 2, ArgType::INT);
    result.code_var = Some(3);
    let mut op = InsnNode::new(InsnType::Const, 1);
    op.result = Some(result);
    op.args.push(InsnArg::lit(27, ArgType::INT));
    let mut entry = Block::new(0u32);
    entry.push(op.clone());
    let mut cfg = CFG::new("operand_versions");
    cfg.add_block(entry);
    cfg.identify_instructions();
    let values = SsaValueGraph::build(&cfg).unwrap();
    let root = block([op]);
    let canonical = Default::default();
    let graph = ValueFlowGraph::build(&root, &values, &canonical).unwrap();
    let facts = SparseValueFacts::analyze(&graph).unwrap();
    let before = SemanticFlowPoint::before(SemanticSiteId(0));
    let after = SemanticFlowPoint::after(SemanticSiteId(0));
    assert!(facts.operands_unchanged(before, after, || -> [SsaVar; 1] {
        panic!("complete SSA versions need no operand enumeration")
    }));
    assert!(!facts.operands_unchanged(before, before, || [key]));
    let graph = ValueFlowGraph::build_source(&root, &BTreeSet::new()).unwrap();
    let facts = SparseValueFacts::analyze(&graph).unwrap();
    // before is not a movement target in this graph; the ordinary source
    // query cannot establish a value fact there.
    assert!(!facts.operands_unchanged(before, after, || [SsaVar::new(3, 0)]));
    assert!(facts.operands_unchanged(before, after, || [SsaVar::new(8, 0)]));
}
