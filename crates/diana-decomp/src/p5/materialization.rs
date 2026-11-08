mod arguments;
mod candidate;
mod members;
mod observations;
mod snapshots;

use super::build::*;
use super::*;
use crate::p2::{EdgeRole, EdgeTarget, MoveKind, NormalEdge, OpKind};
use crate::p3::{PointId, Target, TransferId, TransferOrigin, UseSite};
use crate::p4::{AllocationRelation, ConstructorRelation, Initialization, TypedMethod};
use candidate::candidate;

fn source_ready(typed: &TypedMethod<'_>, read: StorageRead, at: PointId) -> bool {
    match read {
        StorageRead::Literal { .. } => true,
        StorageRead::Value { value, .. } => typed
            .ssa_facts()
            .dominates(typed.ssa_facts().values()[value.0 as usize].point(), at),
        StorageRead::Snapshot { .. } => false,
    }
}
fn same_read(a: StorageRead, b: StorageRead) -> bool {
    match (a, b) {
        (
            StorageRead::Value {
                local: a,
                value: av,
                view: ai,
                ..
            },
            StorageRead::Value {
                local: b,
                value: bv,
                view: bi,
                ..
            },
        ) => a == b && av == bv && ai == bi,
        (
            StorageRead::Literal {
                bits: a, view: ai, ..
            },
            StorageRead::Literal {
                bits: b, view: bi, ..
            },
        ) => a == b && ai == bi,
        _ => a == b,
    }
}
fn normal_success(
    typed: &TypedMethod<'_>,
    op: crate::p2::OpId,
) -> Result<(TransferId, PointId), StorageError> {
    let s = typed.ssa_facts();
    let from = s
        .operation_site(op)
        .ok_or(StorageError::InternalInvariant(
            "construction lost its execution point",
        ))?
        .execute();
    s.transfers().iter().find_map(|t| {
        if t.from()!=from { return None; }
        if matches!(t.origin(),TransferOrigin::Edge(e) if matches!(typed.execution().edges()[e.0 as usize].role(),EdgeRole::Exception{..})) { return None; }
        if let Target::Point(p)=t.target() {Some((t.id(),p))} else {None}
    }).ok_or(StorageError::InternalInvariant("constructor has no normal continuation"))
}
#[derive(Eq, PartialEq)]
struct Handler {
    order: u32,
    catch: Option<crate::TypeId>,
    target: Option<crate::p2::BlockId>,
}
struct Corridor {
    members: Vec<crate::p2::OpId>,
    edges: Vec<crate::p2::EdgeId>,
}
fn protection(typed: &TypedMethod<'_>, op: crate::p2::OpId) -> Result<Vec<Handler>, StorageError> {
    let mut result = Vec::new();
    for e in typed
        .execution()
        .edges()
        .iter()
        .filter(|e| e.source_op() == Some(op))
    {
        if let EdgeRole::Exception { order, catch, .. } = e.role() {
            push(
                &mut result,
                Handler {
                    order,
                    catch,
                    target: match e.target() {
                        EdgeTarget::Block(b) => Some(b),
                        EdgeTarget::Exit(_) => None,
                    },
                },
            )?;
        }
    }
    Ok(result)
}
fn corridor(
    typed: &TypedMethod<'_>,
    allocation: crate::p2::OpId,
    init: crate::p2::OpId,
) -> Result<Result<Corridor, Separation>, StorageError> {
    let flow = typed.execution();
    let mut members = Vec::new();
    let mut edges = Vec::new();
    let mut block = flow.operations()[allocation.0 as usize].block();
    let mut next = allocation.0;
    let mut visited = filled(flow.blocks().len(), false)?;
    loop {
        if visited[block.0 as usize] {
            return Ok(Err(Separation::ControlBoundary(block)));
        }
        visited[block.0 as usize] = true;
        for index in flow
            .block(block)
            .ok_or(StorageError::InternalInvariant(
                "construction corridor lost its block",
            ))?
            .operations()
        {
            if index < next {
                continue;
            }
            let op = &flow.operations()[index as usize];
            push(&mut members, op.id())?;
            if op.id() == init {
                return Ok(Ok(Corridor { members, edges }));
            }
        }
        let mut outgoing = flow
            .edges()
            .iter()
            .filter(|e| e.from() == block && matches!(e.role(), EdgeRole::Normal(_)));
        let Some(edge) = outgoing.next() else {
            return Ok(Err(Separation::ControlBoundary(block)));
        };
        if outgoing.next().is_some()
            || !matches!(
                edge.role(),
                EdgeRole::Normal(NormalEdge::Goto | NormalEdge::Fallthrough)
            )
        {
            return Ok(Err(Separation::ControlBoundary(block)));
        }
        let EdgeTarget::Block(target) = edge.target() else {
            return Ok(Err(Separation::ControlBoundary(block)));
        };
        if flow
            .edges()
            .iter()
            .filter(|e| e.target() == EdgeTarget::Block(target))
            .count()
            != 1
        {
            return Ok(Err(Separation::ControlBoundary(target)));
        }
        push(&mut edges, edge.id())?;
        block = target;
        next = 0;
    }
}
// Removing the constructor's normal transfer asks whether an original use can
// still observe the early committed carrier. It is not a second SSA solver.
fn without_success(
    typed: &TypedMethod<'_>,
    omitted: TransferId,
) -> Result<Vec<bool>, StorageError> {
    let s = typed.ssa_facts();
    let mut seen = filled(s.points().len(), false)?;
    let mut queue = Vec::new();
    let entry = s
        .points()
        .iter()
        .find(|p| p.kind() == crate::p3::PointKind::Entry)
        .ok_or(StorageError::InternalInvariant(
            "construction has no method entry",
        ))?
        .id();
    push(&mut queue, entry)?;
    seen[entry.0 as usize] = true;
    let mut outgoing = Vec::new();
    for _ in s.points() {
        push(&mut outgoing, Vec::new())?;
    }
    for t in s.transfers() {
        if t.id() != omitted
            && let Target::Point(p) = t.target()
        {
            push(&mut outgoing[t.from().0 as usize], p)?;
        }
    }
    while let Some(p) = queue.pop() {
        for &q in &outgoing[p.0 as usize] {
            if !seen[q.0 as usize] {
                seen[q.0 as usize] = true;
                push(&mut queue, q)?;
            }
        }
    }
    Ok(seen)
}
/// Whether the allocated object can reach `at` while carrying `value`, on a path
/// that skipped the constructor's normal completion. A merged Phi imports the
/// allocation through particular incomings only, so each such incoming is
/// decided at its own transfer instead of treating the whole Phi as one early
/// observation. The removed `omitted` transfer cannot carry the object, so it is
/// not an unconstructed arrival. The walk is bounded by this method's Phi values.
fn arrives_unconstructed(
    typed: &TypedMethod<'_>,
    allocation: crate::p2::OpId,
    unsafe_reach: &[bool],
    omitted: TransferId,
    at: PointId,
    observed: crate::p3::ValueId,
    visited: &mut [bool],
) -> Result<bool, StorageError> {
    let s = typed.ssa_facts();
    if !unsafe_reach.get(at.0 as usize).copied().unwrap_or(false)
        || !typed
            .facts()
            .value(observed)
            .is_some_and(|v| v.allocations().contains(&allocation))
    {
        return Ok(false);
    }
    let Some(phi) = s.phis().iter().find(|p| p.value() == observed) else {
        return Ok(true);
    };
    let slot = visited
        .get_mut(phi.id().0 as usize)
        .ok_or(StorageError::InternalInvariant(
            "construction observed an unknown Phi",
        ))?;
    if *slot {
        return Ok(true);
    }
    *slot = true;
    for incoming in phi.incoming() {
        if incoming.transfer() == omitted {
            continue;
        }
        let Some(from) = s
            .transfers()
            .get(incoming.transfer().0 as usize)
            .map(|t| t.from())
        else {
            continue;
        };
        let Some(source) = value(s, incoming.state()) else {
            return Ok(true);
        };
        if arrives_unconstructed(
            typed,
            allocation,
            unsafe_reach,
            omitted,
            from,
            source,
            visited,
        )? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether this operation only copies the object allocated at `allocation`.
fn aliases_allocation(
    typed: &TypedMethod<'_>,
    allocation: crate::p2::OpId,
    operation: crate::p2::OpId,
) -> bool {
    let flow = typed.execution();
    matches!(
        flow.operations()[operation.0 as usize].kind(),
        OpKind::Move {
            kind: MoveKind::Object
        }
    ) && typed
        .facts()
        .initialization()
        .iter()
        .find(|u| matches!(u.site,UseSite::Operation{operation: o,argument:0}if o==operation))
        .is_some_and(|u| {
            u.allocation.is_some_and(|a| {
                a.operation == allocation && a.relation == AllocationRelation::Same
            }) && u.state == Initialization::BeforeConstructor
        })
}

pub(super) fn construct(typed: &TypedMethod<'_>, f: &mut StorageFacts) -> Result<(), StorageError> {
    f.operation_emission = filled(
        typed.execution().operations().len(),
        OperationEmission::Ordinary,
    )?;
    f.copy_emission = filled(f.copies.len(), CopyEmission::Original)?;
    for op in typed.execution().operations() {
        if matches!(op.kind(), OpKind::NewInstance { .. }) {
            push(
                &mut f.allocation_plans,
                AllocationPlan {
                    operation: op.id(),
                    mode: ConstructionMode::Explicit(Separation::NoEstablishedConstructor),
                },
            )?;
        }
    }
    for i in 0..f.constructions.len() {
        let Some(origin) = f.constructions[i].fact.allocation else {
            if matches!(
                f.constructions[i].fact.relation,
                ConstructorRelation::This
                    | ConstructorRelation::Super
                    | ConstructorRelation::Ancestor
                    | ConstructorRelation::OtherOwner
                    | ConstructorRelation::NotAllocation
            ) {
                f.constructions[i].mode =
                    ConstructionMode::Explicit(Separation::NonAllocationConstructor);
            }
            continue;
        };
        let allocation = origin.operation;
        let result = candidate(typed, f, i, allocation)?;
        let mode = match result {
            Err(reason) => ConstructionMode::Explicit(reason),
            Ok(group) => {
                let id = group.id;
                for &op in &group.members {
                    let role = if op == group.allocation {
                        ConstructionRole::Allocation
                    } else if op == group.initialization {
                        ConstructionRole::Initialization
                    } else if matches!(
                        typed.execution().operations()[op.0 as usize].kind(),
                        OpKind::Const { .. }
                    ) {
                        ConstructionRole::Literal
                    } else if aliases_allocation(typed, group.allocation, op) {
                        ConstructionRole::Alias
                    } else if group
                        .prepared_arguments
                        .iter()
                        .any(|prepared| prepared.operation == op)
                    {
                        ConstructionRole::Argument
                    } else {
                        return Err(StorageError::InternalInvariant(
                            "construction retained an intervening operation",
                        ));
                    };
                    f.operation_emission[op.0 as usize] =
                        OperationEmission::Construction { group: id, role };
                }
                for &p in &group.snapshot_members {
                    f.copy_emission[p.0 as usize] = CopyEmission::Construction { group: id };
                }
                for publication in &group.publications {
                    f.bindings[publication.value.0 as usize] = StorageBinding::Deferred {
                        local: publication.local,
                        group: id,
                    };
                    for definition in &mut f.definitions {
                        if typed
                            .ssa_facts()
                            .state(definition.state)
                            .is_some_and(|state| {
                                state.kind() == crate::p3::StateKind::Value(publication.value)
                            })
                        {
                            definition.materialization = Materialization::Deferred {
                                group: id,
                                local: publication.local,
                            };
                        }
                    }
                }
                push(&mut f.groups, group)?;
                ConstructionMode::Natural(id)
            }
        };
        f.constructions[i].mode = mode;
        if let Some(plan) = f
            .allocation_plans
            .iter_mut()
            .find(|p| p.operation == allocation)
            && !matches!(plan.mode, ConstructionMode::Natural(_))
        {
            plan.mode = mode;
        }
    }
    Ok(())
}
