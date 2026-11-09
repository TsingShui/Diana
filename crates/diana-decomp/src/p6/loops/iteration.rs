//! Establishes a source-level iteration continuation from an original shared
//! pure update and its unique backedge. Original local reads stay at that update
//! point; they need not be invariant. Core/lexical membership never changes.

use super::super::build::*;
use super::super::graph::Graph;
use super::super::*;
use crate::p2::{OpKind, Terminal};
use crate::p5::{
    CopyLocation, Materialization, OperationEmission, StorageMethod, StorageRead, ThrowSite,
    TransferSite,
};

fn update(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    plan: &LoopPlan,
) -> Result<Option<LoopUpdate>, RegionError> {
    if !matches!(plan.form, LoopForm::While(_)) {
        return Ok(None);
    }
    let [repeat] = plan.latches.as_slice() else {
        return Ok(None);
    };
    let latch = f.links[repeat.0 as usize].from;
    let end = &f.nodes[latch.0 as usize];
    if !end.operations.is_empty()
        || !end.terminal
        || !matches!(
            storage.execution().blocks()[end.block.0 as usize].terminal(),
            Some(Terminal::Goto { .. })
        )
    {
        return Ok(None);
    }
    let [incoming] = g.incoming[latch.0 as usize].as_slice() else {
        return Ok(None);
    };
    let entry = f.links[incoming.0 as usize].from;
    let node = &f.nodes[entry.0 as usize];
    if node.block != end.block
        || node.terminal
        || node.operations.len() != 1
        || g.outgoing[entry.0 as usize].as_slice() != [*incoming]
        || [entry, latch]
            .iter()
            .any(|n| !plan.members.contains(n) || plan.evaluation.contains(n))
    {
        return Ok(None);
    }
    let operation = crate::p2::OpId(node.operations.start);
    let op = &storage.execution().operations()[operation.0 as usize];
    if !matches!(op.kind(), OpKind::Unary { .. } | OpKind::Binary { .. })
        || op.effects().may_throw()
        || !matches!(
            storage.facts().operation_emission(operation),
            Some(OperationEmission::Ordinary)
        )
    {
        return Ok(None);
    }
    let Some([definition]) = storage.facts().operation_definitions(operation) else {
        return Ok(None);
    };
    let Materialization::Store(destination) = definition.materialization() else {
        return Ok(None);
    };
    let Some(Ok(uses)) = storage.facts().operation_uses(operation) else {
        return Ok(None);
    };
    if !uses
        .iter()
        .any(|u| u.read().is_ok_and(|r| r.local() == Some(destination)))
        || uses.iter().any(|u| {
            !matches!(
                u.read(),
                Ok(StorageRead::Literal { .. } | StorageRead::Value { .. })
            ) && !u.read().is_ok_and(|r| r.local() == Some(destination))
        })
    {
        return Ok(None);
    }
    let contains = |n| n == entry || n == latch;
    if f.links
        .iter()
        .any(|l| l.to == plan.header && plan.lexical_members.contains(&l.from) && l.id != *repeat)
        || f.loops.iter().any(|p| {
            p.id != plan.id
                && p.lexical_members.iter().any(|n| contains(*n))
                && !plan
                    .lexical_members
                    .iter()
                    .all(|n| p.lexical_members.contains(n))
        })
        || f.tries.iter().any(|p| {
            p.members.iter().any(|n| contains(*n))
                && !plan.lexical_members.iter().all(|n| p.members.contains(n))
        })
        || f.synchronized.iter().any(|p| {
            (p.members.iter().any(|n| contains(*n))
                && !plan.lexical_members.iter().all(|n| p.members.contains(n)))
                || p.exits.iter().any(|exit| {
                    matches!(exit.kind,
                    SyncExitKind::Loop { loop_id, kind: SyncLoopTransfer::Continue, target }
                    if loop_id == plan.id && target != entry)
                })
        })
    {
        return Ok(None);
    }
    if g.exceptions.iter().flatten().any(|&edge| {
        Graph::exception_possible(f, edge)
            && Graph::exception_target(f, storage, edge).is_some_and(contains)
    }) {
        return Ok(None);
    }
    if storage.facts().construction_groups().iter().any(|group| {
        group
            .members()
            .iter()
            .any(|op| contains(g.op_node[op.0 as usize]))
            && group
                .members()
                .iter()
                .any(|op| !contains(g.op_node[op.0 as usize]))
    }) {
        return Ok(None);
    }
    // Incoming selected-edge copies remain in the body before its continuation.
    // No phase executed by the update or its backedge can be hidden in a header.
    if storage.facts().copy_plans().iter().filter(|p| !p.is_complete() || !p.steps().is_empty()).any(|p| match p.location() {
        CopyLocation::Transfer { site, .. } => match site {
            TransferSite::InitialEntry => contains(NodeId(0)),
            TransferSite::NormalEdge(e) => matches!(f.edges[e.0 as usize].route, EdgeRoute::Normal(l) if contains(f.links[l.0 as usize].from)),
            TransferSite::AfterCommit(op) | TransferSite::BeforeOperation(op) => contains(g.op_node[op.0 as usize]),
            TransferSite::BeforeTerminal(b) => contains(g.terminal_node[b.0 as usize]),
        },
        CopyLocation::BeforeThrow(ThrowSite::Operation(op)) => contains(g.op_node[op.0 as usize]),
        CopyLocation::BeforeThrow(ThrowSite::Terminal(b)) => contains(g.terminal_node[b.0 as usize]),
        CopyLocation::HandlerEntry { block, .. } => contains(Graph::entry(f, block)),
    }) {
        return Ok(None);
    }
    let mut nodes = Vec::new();
    push(&mut nodes, entry)?;
    push(&mut nodes, latch)?;
    Ok(Some(LoopUpdate {
        entry,
        operation,
        repeat: *repeat,
        nodes,
    }))
}

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
) -> Result<(), RegionError> {
    for index in 0..f.loops.len() {
        let iteration = update(storage, f, g, &f.loops[index])?;
        f.loops[index].iteration = iteration;
    }
    Ok(())
}
