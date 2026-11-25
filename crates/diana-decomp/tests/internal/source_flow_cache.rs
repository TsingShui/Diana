use super::SourceFlowCache;
use crate::ir::analysis::SemanticFlowPoint;
use crate::ir::{
    ArgType, BlockId, InsnArg, InsnNode, InsnType, RegionId, RegisterArg, SemanticBlock,
    SemanticCatch, SemanticLoopControl, SemanticLoopKind, SemanticLoopTest, SemanticNode,
    SemanticOperand, SemanticPredicate, SemanticSiteId, SemanticStatement,
};
use std::sync::Arc;

fn block(id: u32, sites: &[u64], may_throw: bool) -> SemanticNode {
    let statements = sites
        .iter()
        .map(|&site| {
            let insn = if may_throw {
                InsnNode::array_length(
                    RegisterArg::new(0, ArgType::INT),
                    InsnArg::reg(1, ArgType::array(ArgType::INT)),
                )
            } else {
                InsnNode::new(InsnType::Nop, 0)
            };
            let mut statement = SemanticStatement::instruction(insn).expect("semantic instruction");
            statement.site = Some(SemanticSiteId(site));
            statement
        })
        .collect();
    SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(id),
        statements,
    })
}

fn before(id: u64) -> SemanticFlowPoint {
    SemanticFlowPoint::before(SemanticSiteId(id))
}

#[test]
fn cache_reuses_unprotected_effect_changes_but_tracks_throwing_sites_in_a_try() {
    let mut cache = None;
    let plain = SourceFlowCache::get_or_analyze(&mut cache, &block(0, &[0], false));
    let changed = SourceFlowCache::get_or_analyze(&mut cache, &block(0, &[0], true));
    assert!(Arc::ptr_eq(&plain, &changed));

    let protected = |may_throw| SemanticNode::Try {
        region: RegionId::new(0),
        body: Box::new(block(0, &[0], may_throw)),
        catches: vec![SemanticCatch {
            region: RegionId::new(1),
            exception_types: vec![ArgType::object("java.lang.RuntimeException")],
            exception_value: None,
            body: block(1, &[1], false),
        }],
        finally: None,
    };
    let safe = SourceFlowCache::get_or_analyze(&mut cache, &protected(false));
    assert!(!safe.reaches(before(0), before(1)));
    let throwing = SourceFlowCache::get_or_analyze(&mut cache, &protected(true));
    assert!(throwing.reaches(before(0), before(1)));
}

#[test]
fn cache_tracks_occurrence_owners_and_constant_loop_conditions() {
    let mut cache = None;
    let branch = |then_sites: &[u64], else_sites: &[u64]| {
        let mut condition = SemanticOperand::new(SemanticPredicate::True);
        condition.site = Some(SemanticSiteId(0));
        SemanticNode::If {
            condition,
            then_node: Box::new(block(1, then_sites, false)),
            else_node: Some(Box::new(block(2, else_sites, false))),
        }
    };
    let alternatives = SourceFlowCache::get_or_analyze(&mut cache, &branch(&[1], &[2]));
    assert!(!alternatives.reaches(before(1), before(2)));
    let consecutive = SourceFlowCache::get_or_analyze(&mut cache, &branch(&[], &[1, 2]));
    assert!(consecutive.reaches(before(1), before(2)));

    let loop_node = |predicate| {
        let mut test = SemanticLoopTest::pure(predicate);
        test.condition.site = Some(SemanticSiteId(0));
        SemanticNode::Loop {
            control: SemanticLoopControl::Region(RegionId::new(0)),
            header: None,
            kind: SemanticLoopKind::PreTested,
            test,
            body: Box::new(block(1, &[1], false)),
        }
    };
    let skipped = SourceFlowCache::get_or_analyze(&mut cache, &loop_node(SemanticPredicate::False));
    assert!(!skipped.reaches(before(0), before(1)));
    let repeated = SourceFlowCache::get_or_analyze(&mut cache, &loop_node(SemanticPredicate::True));
    assert!(repeated.reaches(before(0), before(1)));
}
