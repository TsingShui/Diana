//! Local continuation protocols from normalized original entry associations.
//! One site/copy producer selects common source controls and retains ABI facts;
//! no external callee body, replacement CFG or downstream rematching is inferred.

use super::build::{bug, push};
use super::graph::Graph;
use super::*;
use crate::p1::CheckedReferenceFact;
use crate::p2::{BranchOp, InvokeKind, OpId, OpKind, Terminal};
use crate::p3::EntryRole;
use crate::p4::{Family, Interpretation};
use crate::p5::{
    CopyStep, LocalId, LocalKind, Materialization, StorageMethod, StorageRead, TransferSite,
};
use crate::raw::ReferenceSlot;

mod association;
mod invalid;
mod nomination;
pub(in crate::p6) use nomination::*;
mod facts;
mod protocol_body;

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    graph: &Graph,
    structure: &super::dominance::Structure,
) -> Result<Option<ProtocolNomination>, RegionError> {
    protocol_body::construct(storage, f, graph, structure)
}

fn reject(storage: &StorageMethod<'_>, dispatch: RegionId, reason: &str) {
    if cfg!(feature = "trace-passes") {
        let flow = storage.execution();
        let key = flow.owner_facts().key();
        eprintln!(
            "[suspension] dex={} class={} method={} dispatch={} status=rejected reason={}",
            key.dex.0,
            key.class_def.0,
            flow.method_id().0,
            dispatch.0,
            reason
        );
    }
}
fn units(value: &[u16], expected: &[u8]) -> bool {
    value
        .iter()
        .copied()
        .eq(expected.iter().copied().map(u16::from))
}
fn ascii(value: crate::input::DexString<'_>, expected: &[u8]) -> Result<bool, RegionError> {
    if u64::from(value.utf16_len().0) != expected.len() as u64 {
        return Ok(false);
    }
    let mut reader = value.reader().map_err(crate::RequestError::Input)?;
    for &byte in expected {
        if reader.next().map_err(crate::RequestError::Input)? != Some(u16::from(byte)) {
            return Ok(false);
        }
    }
    Ok(reader.next().map_err(crate::RequestError::Input)?.is_none())
}
fn inputs<'s>(storage: &'s StorageMethod<'_>, op: OpId) -> Option<&'s [crate::p5::UseAccess]> {
    storage.facts().operation_uses(op)?.ok()
}
fn plain_operation(storage: &StorageMethod<'_>, f: &RegionFacts, op: OpId) -> bool {
    let flow = storage.execution();
    flow.reference_use_issue(flow.operations()[op.0 as usize].pc(),ReferenceSlot::First).is_none()
        && !f.copies.iter().any(|copy| matches!(copy.location,
        crate::p5::CopyLocation::BeforeThrow(crate::p5::ThrowSite::Operation(at))
        | crate::p5::CopyLocation::Transfer { site: TransferSite::BeforeOperation(at) | TransferSite::AfterCommit(at), .. } if at == op))
        && f.throws.iter().filter(|site| site.site == crate::p5::ThrowSite::Operation(op))
            .all(|site| f.chains[site.chain.0 as usize].clauses.iter().all(|c| c.target == CatchTarget::Uncaught))
}
fn terminal_plain(f: &RegionFacts, block: crate::p2::BlockId) -> bool {
    !f.copies.iter().any(|copy| matches!(copy.location,
        crate::p5::CopyLocation::BeforeThrow(crate::p5::ThrowSite::Terminal(at))
        | crate::p5::CopyLocation::Transfer { site: TransferSite::BeforeTerminal(at), .. } if at == block))
        && !f.throws.iter().any(|site| site.site == crate::p5::ThrowSite::Terminal(block))
}
fn same_definition(read: StorageRead, d: &crate::p5::DefinitionPlan) -> bool {
    matches!(read, StorageRead::Value { local, state, .. }
        if d.materialization() == Materialization::Store(local) && d.state() == state)
}
fn edge_copy(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    link: LinkId,
) -> Option<(crate::p5::PlanId, LocalId, StorageRead)> {
    let [id] = f.link_plans[link.0 as usize].copies.as_slice() else {
        return None;
    };
    let plan = storage.facts().copy_plans().get(id.0 as usize)?;
    if storage.facts().copy_emission(*id) != Some(crate::p5::CopyEmission::Original)
        || plan.assignments().len() != 1
        || plan.assignments()[0].issue().is_some()
    {
        return None;
    }
    let [
        CopyStep::Assign {
            destination,
            source,
            ..
        },
    ] = plan.steps()
    else {
        return None;
    };
    if !matches!((plan.location(), f.links[link.0 as usize].origin),
        (crate::p5::CopyLocation::Transfer { site: TransferSite::NormalEdge(a), .. }, LinkOrigin::Edge(b)) if a == b)
    {
        return None;
    }
    Some((*id, *destination, *source))
}
fn private_result(
    storage: &StorageMethod<'_>,
    read: StorageRead,
    predicate: crate::p2::BlockId,
    copy: crate::p5::PlanId,
) -> bool {
    let Some(local) = read.local() else {
        return false;
    };
    let flow = storage.execution();
    if flow.operations().iter().any(|op| {
        inputs(storage, op.id()).is_none_or(|uses| {
            uses.iter()
                .any(|u| !u.read().is_ok_and(|read| read.local() != Some(local)))
        })
    }) {
        return false;
    }
    if flow
        .blocks()
        .iter()
        .filter(|block| block.id() != predicate && !flow.terminal_reads(block).is_empty())
        .any(|block| {
            storage
                .facts()
                .terminal_uses(block.id())
                .is_none_or(|uses| {
                    uses.iter()
                        .any(|u| !u.read().is_ok_and(|read| read.local() != Some(local)))
                })
        })
    {
        return false;
    }
    !storage
        .facts()
        .copy_plans()
        .iter()
        .filter(|p| p.id() != copy)
        .any(|p| {
            p.assignments().iter().any(|a| {
                a.issue().is_some()
                    || a.source().ok().and_then(StorageRead::local) == Some(local)
                    || a.destination().ok() == Some(local)
            })
        })
}
fn state_store(
    storage: &StorageMethod<'_>,
    id: OpId,
    label: crate::FieldId,
    this: LocalId,
    state: i32,
) -> Result<bool, RegionError> {
    let flow = storage.execution();
    let op = &flow.operations()[id.0 as usize];
    if !matches!(op.kind(), OpKind::Field { access: crate::p2::FieldAccess::Put, static_field:false, field, .. } if *field==label)
    {
        return Ok(false);
    }
    let Some([value, receiver]) = inputs(storage, id) else {
        return Ok(false);
    };
    let Ok(expected) = u64::try_from(state) else {
        return Ok(false);
    };
    Ok(
        receiver.read().ok().and_then(StorageRead::local) == Some(this)
            && matches!(value.read(), Ok(StorageRead::Literal { bits, .. }) if bits.width==crate::SlotWidth::One && bits.value==expected),
    )
}
fn failure_check(
    storage: &StorageMethod<'_>,
    id: OpId,
    parameter: LocalId,
) -> Result<bool, RegionError> {
    let flow = storage.execution();
    let op = &flow.operations()[id.0 as usize];
    if !matches!(
        op.kind(),
        OpKind::Invoke {
            kind: InvokeKind::Static,
            result: None,
            ..
        }
    ) {
        return Ok(false);
    }
    let Some([value]) = inputs(storage, id) else {
        return Ok(false);
    };
    if value.read().ok().and_then(StorageRead::local) != Some(parameter) {
        return Ok(false);
    }
    let Some(Ok(CheckedReferenceFact::Method {
        owner,
        name,
        prototype,
        ..
    })) = flow.reference_fact(op.pc(), ReferenceSlot::First)
    else {
        return Ok(false);
    };
    Ok(ascii(*owner, b"Lkotlin/ResultKt;")?
        && ascii(*name, b"throwOnFailure")?
        && prototype.parameters().len() == 1
        && ascii(
            prototype.parameters()[0].descriptor(),
            b"Ljava/lang/Object;",
        )?
        && ascii(prototype.return_type(), b"V")?)
}
fn marker_source(storage: &StorageMethod<'_>, local: LocalId) -> Result<bool, RegionError> {
    let Some(d) = storage
        .facts()
        .definitions()
        .iter()
        .find(|d| d.materialization() == Materialization::Store(local))
    else {
        return Ok(false);
    };
    let flow = storage.execution();
    let op = &flow.operations()[d.operation().0 as usize];
    if !matches!(
        op.kind(),
        OpKind::Invoke {
            kind: InvokeKind::Static,
            result: Some(crate::SlotWidth::One),
            ..
        }
    ) {
        return Ok(false);
    }
    let Some(Ok(CheckedReferenceFact::Method {
        owner,
        name,
        prototype,
        ..
    })) = flow.reference_fact(op.pc(), ReferenceSlot::First)
    else {
        return Ok(false);
    };
    Ok((ascii(
        *owner,
        b"Lkotlin/coroutines/intrinsics/IntrinsicsKt__IntrinsicsKt;",
    )? || ascii(*owner, b"Lkotlin/coroutines/intrinsics/IntrinsicsKt;")?)
        && ascii(*name, b"getCOROUTINE_SUSPENDED")?
        && prototype.parameters().is_empty()
        && ascii(prototype.return_type(), b"Ljava/lang/Object;")?)
}
fn suspending_signature(
    storage: &StorageMethod<'_>,
    id: OpId,
    this: LocalId,
) -> Result<bool, RegionError> {
    let flow = storage.execution();
    let op = &flow.operations()[id.0 as usize];
    if !matches!(
        op.kind(),
        OpKind::Invoke {
            kind: InvokeKind::Static
                | InvokeKind::Virtual
                | InvokeKind::Interface
                | InvokeKind::Direct,
            result: Some(crate::SlotWidth::One),
            ..
        }
    ) {
        return Ok(false);
    }
    let Some(uses) = inputs(storage, id) else {
        return Ok(false);
    };
    if uses
        .last()
        .and_then(|u| u.read().ok())
        .and_then(StorageRead::local)
        != Some(this)
    {
        return Ok(false);
    }
    let Some(Ok(CheckedReferenceFact::Method { prototype, .. })) =
        flow.reference_fact(op.pc(), ReferenceSlot::First)
    else {
        return Ok(false);
    };
    let Some(last) = prototype.parameters().last() else {
        return Ok(false);
    };
    Ok(
        ascii(last.descriptor(), b"Lkotlin/coroutines/Continuation;")?
            && ascii(prototype.return_type(), b"Ljava/lang/Object;")?,
    )
}
