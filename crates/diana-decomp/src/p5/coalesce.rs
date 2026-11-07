//! Copy-affinity storage reuse. Source values/states and operation/transfer
//! positions do not change. P5 selects representatives before its one copy
//! scheduling pass; later stages do not perform liveness or rename storage.
mod graph;

use super::build::*;
use super::*;
use crate::p2::OpKind;
use crate::p3::ValueId;
use crate::p4::{Identity, Initialization, Interpretation, TypedMethod};

fn identity_source(
    f: &StorageFacts,
    destination: LocalId,
    source: Option<StorageRead>,
) -> Option<LocalId> {
    let StorageRead::Value { local, view, .. } = source? else {
        return None;
    };
    if f.locals[local.0 as usize].storage_type != f.locals[destination.0 as usize].storage_type {
        return None;
    }
    match (view, f.locals[destination.0 as usize].storage_type) {
        (Interpretation::Direct(a), StorageType::Family { family: b, .. }) if a == b => Some(local),
        (Interpretation::CopyBits(a), StorageType::NumericBits { width: b, .. }) if a == b => {
            Some(local)
        }
        _ => None,
    }
}
fn pin_read(pinned: &mut [bool], read: Result<StorageRead, StorageGap>) {
    if let Some(local) = read.ok().and_then(StorageRead::local) {
        pinned[local.0 as usize] = true;
    }
}
fn pinned(typed: &TypedMethod<'_>, f: &StorageFacts) -> Result<Vec<bool>, StorageError> {
    let mut pinned = filled(f.locals.len(), false)?;
    for local in &f.locals {
        let index = local.id.0 as usize;
        pinned[index] = f.local_gaps[index].is_some();
        match local.kind {
            LocalKind::Value(v) | LocalKind::ReferenceCarrier(v) => {
                // Construction publications have a different commit position.
                // They stay independent until that position is represented in
                // storage liveness; a raw allocation commit cannot stand in for it.
                if let Some(Identity::Root(root)) = typed.facts().value(v).map(|v| v.identity())
                    && matches!(f.binding(root), Some(StorageBinding::Uninitialized { .. }))
                {
                    pinned[index] = true;
                }
            }
            _ => pinned[index] = true,
        }
    }
    for p in &f.copies {
        if !p.is_complete() || !matches!(p.location, CopyLocation::Transfer { .. }) {
            for a in &p.assignments {
                if let Ok(local) = a.destination {
                    pinned[local.0 as usize] = true;
                }
                pin_read(&mut pinned, a.source);
                if let Some(local) = a.dependency {
                    pinned[local.0 as usize] = true;
                }
            }
        }
    }
    for u in &f.uses {
        if u.read().is_err() {
            pin_read(&mut pinned, u.original_source());
        }
        if let Some(local) = u.original_source().ok().and_then(StorageRead::local)
            && matches!(
                f.locals[local.0 as usize].storage_type,
                StorageType::Family {
                    family: crate::p4::Family::Reference,
                    ..
                }
            )
            && u.initialization() != Some(Initialization::Initialized)
        {
            pinned[local.0 as usize] = true;
        }
    }
    Ok(pinned)
}
fn value_of(local: &Local) -> Option<ValueId> {
    match local.kind {
        LocalKind::Value(v) | LocalKind::ReferenceCarrier(v) => Some(v),
        _ => None,
    }
}
fn compatible(typed: &TypedMethod<'_>, f: &StorageFacts, a: LocalId, b: LocalId) -> bool {
    if f.locals[a.0 as usize].storage_type != f.locals[b.0 as usize].storage_type {
        return false;
    }
    let (Some(a), Some(b)) = (
        value_of(&f.locals[a.0 as usize]),
        value_of(&f.locals[b.0 as usize]),
    ) else {
        return false;
    };
    let (Some(a), Some(b)) = (typed.facts().value(a), typed.facts().value(b)) else {
        return false;
    };
    // Consumers can retain the representative's declared-type sources only
    // when all members carry precisely those same facts; no type join is guessed.
    a.types() == b.types() && a.has_unknown_type_source() == b.has_unknown_type_source()
}
fn root(parent: &[LocalId], mut id: LocalId) -> LocalId {
    while parent[id.0 as usize] != id {
        id = parent[id.0 as usize];
    }
    id
}
fn redirect(read: &mut Result<StorageRead, StorageGap>, representatives: &[LocalId]) {
    if let Ok(read) = read
        && let Some(local) = read.local()
    {
        read.redirect(representatives[local.0 as usize]);
    }
}
pub(super) fn construct(typed: &TypedMethod<'_>, f: &mut StorageFacts) -> Result<(), StorageError> {
    // An unknown input layout has no complete local read set for interference.
    // Keep the delivered independent storages, not a guessed empty read set.
    if f.operations.iter().any(|o| o.inputs.is_err()) {
        return Ok(());
    }
    let pinned = pinned(typed, f)?;
    let mut affinities = Vec::new();
    for p in &f.copies {
        if !matches!(p.location, CopyLocation::Transfer { .. }) {
            continue;
        }
        for a in &p.assignments {
            if a.issue.is_none()
                && let Ok(destination) = a.destination
                && let Some(source) = identity_source(f, destination, a.source.ok())
                && !pinned[source.0 as usize]
                && !pinned[destination.0 as usize]
            {
                push(&mut affinities, (source, destination))?;
            }
        }
    }
    for op in typed.execution().operations() {
        if !matches!(op.kind(), OpKind::Move { .. }) {
            continue;
        }
        let (Some(Ok([u])), Some([d])) =
            (f.operation_uses(op.id()), f.operation_definitions(op.id()))
        else {
            continue;
        };
        let Materialization::Store(destination) = d.materialization else {
            continue;
        };
        if let Some(source) = identity_source(f, destination, u.read().ok())
            && !pinned[source.0 as usize]
            && !pinned[destination.0 as usize]
        {
            push(&mut affinities, (source, destination))?;
        }
    }
    if affinities.is_empty() {
        return Ok(());
    }
    let graph = graph::Graph::construct(typed, f)?;
    let conflicts = graph.interference(f.locals.len())?;
    let mut parent = Vec::new();
    let mut members = graph::lists(f.locals.len())?;
    for local in &f.locals {
        push(&mut parent, local.id)?;
        push(&mut members[local.id.0 as usize], local.id)?;
    }
    for (a, b) in affinities {
        let (a, b) = (root(&parent, a), root(&parent, b));
        if a == b || !compatible(typed, f, a, b) {
            continue;
        }
        if members[a.0 as usize].iter().any(|member| {
            conflicts[member.0 as usize]
                .iter()
                .any(|&other| root(&parent, other) == b)
        }) {
            continue;
        }
        let (keep, remove) = if a.0 < b.0 { (a, b) } else { (b, a) };
        let additional = members[remove.0 as usize].len();
        crate::substrate::owned::reserve(&mut members[keep.0 as usize], additional)?;
        let removed = std::mem::take(&mut members[remove.0 as usize]);
        members[keep.0 as usize].extend(removed);
        parent[remove.0 as usize] = keep;
    }
    let mut reused = 0usize;
    for local in &mut f.locals {
        let representative = root(&parent, local.id);
        parent[local.id.0 as usize] = representative;
        local.representative = representative;
        local.reused = members[representative.0 as usize].len() > 1;
        reused += usize::from(local.id != representative);
    }
    if reused == 0 {
        return Ok(());
    }
    for binding in &mut f.bindings {
        match binding {
            StorageBinding::Local(l)
            | StorageBinding::ReferenceCarrier(l)
            | StorageBinding::Parameter { local: l, .. }
            | StorageBinding::Uninitialized { local: l, .. }
            | StorageBinding::Deferred { local: l, .. } => *l = parent[l.0 as usize],
            StorageBinding::Literal(_) | StorageBinding::Unavailable(_) => {}
        }
    }
    for d in &mut f.definitions {
        match &mut d.materialization {
            Materialization::Store(l)
            | Materialization::ExceptionCapture(l)
            | Materialization::AllocationCarrier(l)
            | Materialization::Deferred { local: l, .. } => *l = parent[l.0 as usize],
            _ => {}
        }
    }
    for u in &mut f.uses {
        redirect(&mut u.read, &parent);
    }
    for p in &mut f.copies {
        for a in &mut p.assignments {
            if let Ok(local) = &mut a.destination {
                *local = parent[local.0 as usize];
            }
            redirect(&mut a.source, &parent);
            if let Some(local) = &mut a.dependency {
                *local = parent[local.0 as usize];
            }
        }
    }
    for c in &mut f.constructions {
        if let Some(read) = &mut c.receiver {
            redirect(read, &parent);
        }
    }
    #[cfg(feature = "trace-passes")]
    {
        let key = typed.execution().owner_facts().key();
        eprintln!(
            "[storage] dex={} class={} method={} reused_locals={reused}",
            key.dex.0,
            key.class_def.0,
            typed.execution().method_id().0
        );
    }
    Ok(())
}
