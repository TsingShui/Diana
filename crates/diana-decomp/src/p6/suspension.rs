//! Local suspension protocols over the existing continuation decision regions.
//! Original ABI dispatch, state stores and resume/error facts remain owned. A
//! local plan joins a call's marker exit and normal result with its resume ingress;
//! the closed body alternative also retains explicit ABI-only attachments.
//! Neither product rebuilds the CFG or infers an external callee body.

use super::build::{bug, push};
use super::graph::Graph;
use super::*;
use crate::p1::CheckedReferenceFact;
use crate::p2::{BranchOp, CommitRule, InvokeKind, OpId, OpKind, Terminal};
use crate::p3::EntryRole;
use crate::p4::{Family, Interpretation};
use crate::p5::{
    CopyStep, LocalId, LocalKind, Materialization, StorageMethod, StorageRead, TransferSite,
};
use crate::raw::ReferenceSlot;

mod body;
mod facts;
mod loop_body;
mod multi_body;

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    graph: &Graph,
    structure: &super::dominance::Structure,
) -> Result<(), RegionError> {
    multi_body::construct(storage, f, graph, structure)?;
    if f.coroutine_flow.is_some() {
        return Ok(());
    }
    loop_body::construct(storage, f, graph, structure)?;
    let flow = storage.execution();
    let d = flow.declaration();
    if !units(d.name_units(), b"invokeSuspend")
        || !units(
            d.prototype_units(),
            b"(Ljava/lang/Object;)Ljava/lang/Object;",
        )
        || flow.access_flags()
            & (crate::substrate::flags::STATIC | crate::substrate::flags::SYNCHRONIZED)
            != 0
        || !matches!(flow.owner_facts().superclass(), crate::p0::Fact::Present(name)
            if units(name.units(), b"Lkotlin/coroutines/jvm/internal/SuspendLambda;"))
    {
        return Ok(());
    }
    let this = storage
        .facts()
        .locals()
        .iter()
        .find_map(|local| match local.kind() {
            LocalKind::Parameter {
                role: EntryRole::This,
                ..
            } => Some(local.id()),
            _ => None,
        });
    let parameter = storage
        .facts()
        .locals()
        .iter()
        .find_map(|local| match local.kind() {
            LocalKind::Parameter {
                role: EntryRole::Parameter(0),
                ..
            } => Some(local.id()),
            _ => None,
        });
    let (Some(this), Some(parameter)) = (this, parameter) else {
        return Ok(());
    };
    let mut saw_dispatch = false;
    for region_index in 0..f.regions.len() {
        let Some(decision) = f.regions[region_index].decision.as_ref() else {
            continue;
        };
        if decision.tests.len() != 2 || decision.tests[0].key != 0 || decision.tests[1].key <= 0 {
            continue;
        }
        saw_dispatch = true;
        let initial = decision.tests[0].body;
        let resume = decision.tests[1].body;
        let state = decision.tests[1].key;
        let dispatch = f.regions[region_index].id;
        let Some(follow) = f.regions[region_index].follow else {
            continue;
        };
        let Some(label) = label_source(storage, f, decision.tests[0].predicate, this)? else {
            reject(storage, dispatch, "label_source");
            continue;
        };
        let RegionKind::Block(resume_node) = f.regions[resume.0 as usize].kind else {
            continue;
        };
        let resume_range = f.nodes[resume_node.0 as usize].operations.clone();
        if resume_range.end.checked_sub(resume_range.start) != Some(1) {
            reject(
                storage,
                dispatch,
                "resume_body_contains_spill_or_other_operations",
            );
            continue;
        }
        let resume_check = OpId(resume_range.start);
        if !failure_check(storage, resume_check, parameter)?
            || !plain_operation(storage, f, resume_check)
        {
            reject(storage, dispatch, "resume_failure_check_or_phases");
            continue;
        }
        if graph.incoming[resume_node.0 as usize].as_slice() != [decision.tests[1].selected] {
            reject(storage, dispatch, "resume_has_other_ingress");
            continue;
        }
        let [resume_link] = graph.outgoing[resume_node.0 as usize].as_slice() else {
            continue;
        };
        if f.links[resume_link.0 as usize].to != follow {
            continue;
        }
        let Some((resume_copy, merged, resumed)) = edge_copy(storage, f, *resume_link) else {
            reject(storage, dispatch, "resume_copy");
            continue;
        };
        if resumed.local() != Some(parameter) {
            continue;
        }
        for region in &f.regions {
            let RegionKind::If {
                true_link,
                false_link,
            } = region.kind
            else {
                continue;
            };
            if !within(f, region.id, initial) {
                continue;
            }
            let Some(predicate) = region.entry else {
                continue;
            };
            let block = f.nodes[predicate.0 as usize].block;
            let Some(Terminal::Branch {
                op: BranchOp::Ne, ..
            }) = flow.blocks()[block.0 as usize].terminal()
            else {
                continue;
            };
            let Some(uses) = storage.facts().terminal_uses(block) else {
                continue;
            };
            let [left, right] = uses else { continue };
            let (Ok(call_result), Ok(marker)) = (left.read(), right.read()) else {
                continue;
            };
            let (Some(result_local), Some(marker_local)) = (call_result.local(), marker.local())
            else {
                continue;
            };
            if !matches!(
                call_result,
                StorageRead::Value {
                    view: Interpretation::Direct(Family::Reference),
                    ..
                }
            ) || !matches!(
                marker,
                StorageRead::Value {
                    view: Interpretation::Direct(Family::Reference),
                    ..
                }
            ) {
                continue;
            }
            let Some(definition) = storage
                .facts()
                .definitions()
                .iter()
                .find(|d| d.materialization() == Materialization::Store(result_local))
            else {
                continue;
            };
            let call = definition.operation();
            let source = &flow.operations()[call.0 as usize];
            if source.block() != block
                || flow
                    .reference_use_issue(source.pc(), ReferenceSlot::First)
                    .is_some()
                || definition.commit() != CommitRule::AfterResult
                || source.result_pc().is_none()
                || !same_definition(call_result, definition)
            {
                continue;
            }
            if !suspending_signature(storage, call, this)? || !plain_operation(storage, f, call) {
                reject(storage, dispatch, "call_signature_or_phases");
                continue;
            }
            let Some(store_index) = call.0.checked_sub(1) else {
                continue;
            };
            let store = OpId(store_index);
            if flow.operations()[store_index as usize].block() != block
                || !state_store(storage, store, label, this, state)?
            {
                reject(storage, dispatch, "state_store");
                continue;
            }
            // Every original operation after the call belongs to its predicate:
            // no effect or captured value can be hidden by the suspension form.
            let call_node = graph.op_node[call.0 as usize];
            if ![call_node, predicate, resume_node]
                .into_iter()
                .all(|node| f.nodes[node.0 as usize].reachable && f.lock_entries.empty_at(node))
                || flow.operations()[f.nodes[call_node.0 as usize].operations.start as usize
                    ..f.nodes[call_node.0 as usize].operations.end as usize]
                    .iter()
                    .any(|op| matches!(op.kind(), OpKind::Monitor { .. }))
            {
                reject(storage, dispatch, "local_lock_emptiness_not_established");
                continue;
            }
            if f.nodes[call_node.0 as usize].operations.end
                != call
                    .0
                    .checked_add(1)
                    .ok_or(crate::RequestError::PlatformCapacity)?
                || !f.nodes[predicate.0 as usize].operations.is_empty()
                || graph.incoming[predicate.0 as usize].len() != 1
            {
                continue;
            }
            let incoming = f.links[graph.incoming[predicate.0 as usize][0].0 as usize];
            if incoming.from != call_node
                || !matches!(incoming.origin, LinkOrigin::WithinBlock(_))
                || !f.link_plans[incoming.id.0 as usize].copies.is_empty()
            {
                continue;
            }
            let [header, normal_body, suspended_body] = region.children.as_slice() else {
                continue;
            };
            let normal = &f.regions[normal_body.0 as usize];
            if !normal.nodes.is_empty()
                || normal.follow != Some(follow)
                || !matches!(normal.continuation, Continuation::Parent)
                || f.links[true_link.0 as usize].to != follow
            {
                continue;
            }
            let suspended_node = f.links[false_link.0 as usize].to;
            if f.regions[suspended_body.0 as usize].nodes.as_slice() != [suspended_node]
                || !f.nodes[suspended_node.0 as usize].operations.is_empty()
                || !f.nodes[suspended_node.0 as usize].terminal
                || graph.incoming[suspended_node.0 as usize].as_slice() != [false_link]
                || !f.link_plans[false_link.0 as usize].copies.is_empty()
            {
                continue;
            }
            if !f.lock_entries.empty_at(suspended_node)
                || !f.nodes[suspended_node.0 as usize].reachable
            {
                continue;
            }
            let returned = f.nodes[suspended_node.0 as usize].block;
            if !matches!(
                flow.blocks()[returned.0 as usize].terminal(),
                Some(Terminal::Return { .. })
            ) {
                continue;
            }
            let Some([returned_value]) = storage.facts().terminal_uses(returned) else {
                continue;
            };
            if !same_read(returned_value.read().ok(), Some(marker))
                || !terminal_plain(f, block)
                || !terminal_plain(f, returned)
            {
                continue;
            }
            if !marker_source(storage, marker_local)? {
                reject(storage, dispatch, "marker_source");
                continue;
            }
            let Some((normal_copy, destination, copied)) = edge_copy(storage, f, true_link) else {
                continue;
            };
            if destination != merged || !same_read(Some(copied), Some(call_result)) {
                continue;
            }
            let ingress = &graph.incoming[follow.0 as usize];
            if ingress.len() != 2 || !ingress.contains(&true_link) || !ingress.contains(resume_link)
            {
                continue;
            }
            if !private_result(storage, call_result, block, normal_copy) {
                reject(storage, dispatch, "call_result_has_other_use");
                continue;
            }
            if graph.exceptions.iter().flatten().any(|&edge| Graph::exception_possible(f, edge)
                && matches!(Graph::exception_target(f, storage, edge), Some(node) if node == predicate || node == suspended_node || node == resume_node)) { continue }
            let plan = SuspensionPlan {
                dispatch,
                control: region.id,
                header: *header,
                normal_body: *normal_body,
                suspended_body: *suspended_body,
                state,
                state_store: store,
                call,
                result: result_local,
                merged,
                continuation_argument: u32::try_from(
                    storage
                        .facts()
                        .operation_uses(call)
                        .and_then(Result::ok)
                        .ok_or(bug("matched suspension lost its inputs"))?
                        .len()
                        .checked_sub(1)
                        .ok_or(bug("matched suspension has no continuation"))?,
                )
                .map_err(|_| crate::RequestError::PlatformCapacity)?,
                marker: marker_local,
                predicate: block,
                normal: true_link,
                suspended: false_link,
                marker_return: returned,
                normal_copy,
                resume,
                resume_check,
                resume_link: *resume_link,
                resume_copy,
            };
            if cfg!(feature = "trace-passes") {
                let key = flow.owner_facts().key();
                eprintln!(
                    "[suspension] dex={} class={} method={} call={} state={} dispatch={} normal_copy={} resume_copy={} status=protocol",
                    key.dex.0,
                    key.class_def.0,
                    flow.method_id().0,
                    call.0,
                    state,
                    dispatch.0,
                    normal_copy.0,
                    resume_copy.0
                );
            }
            push(&mut f.suspensions, plan)?;
            break;
        }
    }
    if !saw_dispatch && cfg!(feature = "trace-passes") {
        let key = flow.owner_facts().key();
        eprintln!(
            "[coroutine-body] dex={} class={} method={} layout=explicit reason=state_dispatch_plan_unavailable",
            key.dex.0,
            key.class_def.0,
            flow.method_id().0
        );
    }
    body::construct(storage, f, graph)
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
fn within(f: &RegionFacts, mut child: RegionId, parent: RegionId) -> bool {
    for _ in 0..f.regions.len() {
        if child == parent {
            return true;
        }
        let Some(up) = f.parent(child) else {
            return false;
        };
        child = up;
    }
    false
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
fn same_read(a: Option<StorageRead>, b: Option<StorageRead>) -> bool {
    matches!((a,b), (Some(StorageRead::Value { local:a, value:av, state:as_, view:aa }),
        Some(StorageRead::Value { local:b, value:bv, state:bs, view:ba })) if a==b && av==bv && as_==bs && aa==ba)
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
fn label_source(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    predicate: NodeId,
    this: LocalId,
) -> Result<Option<crate::FieldId>, RegionError> {
    let Some(uses) = storage
        .facts()
        .terminal_uses(f.nodes[predicate.0 as usize].block)
    else {
        return Ok(None);
    };
    let Some(local) = uses
        .first()
        .and_then(|u| u.read().ok())
        .and_then(StorageRead::local)
    else {
        return Ok(None);
    };
    let Some(d) = storage
        .facts()
        .definitions()
        .iter()
        .find(|d| d.materialization() == Materialization::Store(local))
    else {
        return Ok(None);
    };
    let flow = storage.execution();
    let op = &flow.operations()[d.operation().0 as usize];
    let OpKind::Field {
        access: crate::p2::FieldAccess::Get,
        static_field: false,
        field,
        ..
    } = op.kind()
    else {
        return Ok(None);
    };
    let Some([receiver]) = inputs(storage, op.id()) else {
        return Ok(None);
    };
    if receiver.read().ok().and_then(StorageRead::local) != Some(this) {
        return Ok(None);
    }
    let Some(Ok(CheckedReferenceFact::Field {
        owner_type,
        name,
        field_type,
        ..
    })) = flow.reference_fact(op.pc(), ReferenceSlot::First)
    else {
        return Ok(None);
    };
    Ok(((*owner_type == flow.owner_facts().type_id())
        && ascii(*name, b"label")?
        && ascii(*field_type, b"I")?)
    .then_some(*field))
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
