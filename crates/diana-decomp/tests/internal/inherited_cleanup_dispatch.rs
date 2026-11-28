use super::*;
use crate::ir::analysis::SsaValueGraph;
use crate::ir::exception::{CatchHandler, ExceptionAnalysis, HandlerKind, TryRegion};
use crate::ir::{
    ArgType, Block, BlockId, EdgeKind, InsnNode, RegionGraphBuilder, RegisterArg, SemanticBlock,
    SemanticExpression, SemanticLeave,
};

#[test]
fn an_inherited_terminal_cleanup_runs_at_only_the_enclosing_dispatch() {
    let (cfg, analysis, handler_value) = shared_cleanup_graph();
    let facts = SsaValueGraph::build(&cfg).expect("SSA values");
    let regions = RegionGraphBuilder::new(&cfg, &analysis, &facts)
        .build()
        .expect("region graph");
    let inner = regions.exception_regions()[&1][0];
    let cleanup = regions.handlers_of(inner)[0];
    let terminal = SemanticNode::Leave(SemanticLeave {
        site: None,
        condition: None,
        kind: SemanticLeaveKind::Throw(SemanticExpression::Register(handler_value.clone())),
        edge: None,
        origin: Some(BlockId(4)),
        source: cleanup,
        destination: regions.tree().root(),
        target: regions.tree().root(),
        cleanup: Vec::new(),
    });
    let closed = |body| SemanticCatch {
        region: cleanup,
        exception_types: vec![ArgType::throwable()],
        exception_value: Some(handler_value.clone()),
        body,
    };
    let make_inner = |fallback| SemanticNode::Try {
        region: inner,
        body: Box::new(SemanticNode::BasicBlock(SemanticBlock {
            id: BlockId(1),
            statements: Vec::new(),
        })),
        catches: vec![
            SemanticCatch {
                region: inner,
                exception_types: vec![ArgType::object("java/lang/InterruptedException")],
                exception_value: None,
                body: SemanticNode::Empty,
            },
            closed(fallback),
        ],
        finally: None,
    };
    let enclosing = closed(terminal.clone());
    let remove = |body| {
        RemoveInheritedCleanup {
            regions: &regions,
            catch: &enclosing,
            protected_bodies: Vec::new(),
        }
        .fold_node(body)
        .expect("cleanup rewrite")
    };

    assert!(matches!(
        remove(make_inner(terminal.clone())),
        SemanticNode::Try { catches, .. } if catches.len() == 1 && catches[0].region == inner
    ));
    assert!(matches!(
        remove(make_inner(SemanticNode::Empty)),
        SemanticNode::Try { catches, .. } if catches.len() == 2
    ));
    assert!(matches!(
        remove(SemanticNode::Try {
            region: inner,
            body: Box::new(make_inner(terminal.clone())),
            catches: vec![closed(terminal)],
            finally: None,
        }),
        SemanticNode::Try { body, catches, .. }
            if catches.len() == 1
                && matches!(&*body, SemanticNode::BasicBlock(block) if block.id == BlockId(1))
    ));
}

fn shared_cleanup_graph() -> (CFG, ExceptionAnalysis, RegisterArg) {
    let mut cfg = CFG::new("inherited_cleanup");
    let handler_value = RegisterArg::new_ssa(0, 0, ArgType::throwable());
    for raw in [0, 1, 2, 4] {
        let mut block = Block::new(raw);
        match raw {
            2 => block.push(InsnNode::return_void()),
            4 => {
                block.push(InsnNode::move_exception(handler_value.clone()));
                block.push(InsnNode::throw(crate::ir::InsnArg::Reg(
                    handler_value.clone(),
                )));
            }
            _ => block.push(InsnNode::goto(raw as i32 + 1)),
        }
        cfg.add_block(block);
    }
    cfg.add_edge(BlockId(0), BlockId(1), EdgeKind::Normal);
    cfg.add_edge(BlockId(1), BlockId(2), EdgeKind::Normal);
    cfg.add_edge(BlockId(1), BlockId(4), EdgeKind::Exception);
    let handler = CatchHandler {
        id: 4,
        catch_type: None,
        handler_offset: 4,
        entry_blocks: BTreeSet::from([BlockId(4)]),
        handler_block: BlockId(4),
        semantic_entry: BlockId(4),
        canonical_entry: BlockId(4),
        adapter_blocks: BTreeSet::new(),
        blocks: vec![BlockId(4)],
        semantic_blocks: vec![BlockId(4)],
        lexical_blocks: vec![BlockId(4)],
        continuation: None,
        exception_value: Some(handler_value.clone()),
        canonical_exception_value: Some(handler_value.clone()),
        rethrow_blocks: BTreeSet::from([BlockId(4)]),
        kind: HandlerKind::Cleanup,
    };
    let analysis = ExceptionAnalysis {
        regions: vec![TryRegion {
            id: 1,
            start_offset: 1,
            end_offset: 2,
            blocks: vec![BlockId(1)],
            handlers: vec![handler],
            parent: None,
            children: Vec::new(),
            normal_exit_blocks: vec![BlockId(1)],
        }],
        ..ExceptionAnalysis::default()
    };
    (cfg, analysis, handler_value)
}
