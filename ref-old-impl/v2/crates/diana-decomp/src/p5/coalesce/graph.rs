//! Storage liveness events on P3's already established position/transfer graph.
//! Commit nodes remain off exceptional edges. Copy bundles use their original
//! transfer, with AfterCommit explicitly placed after the committed definition.

use super::*;
use crate::p3::{Target, TransferOrigin};

#[derive(Default)]
pub(super) struct Event {
    pub reads: Vec<LocalId>,
    pub writes: Vec<(LocalId, Option<LocalId>)>,
}
pub(super) struct Graph {
    pub events: Vec<Event>,
    pub outgoing: Vec<Vec<usize>>,
    pub incoming: Vec<Vec<usize>>,
}
pub(super) fn lists<T>(n: usize) -> Result<Vec<Vec<T>>, StorageError> {
    let mut out = Vec::new();
    crate::substrate::owned::reserve(&mut out, n)?;
    out.resize_with(n, Vec::new);
    Ok(out)
}
pub(super) fn insert(values: &mut Vec<LocalId>, value: LocalId) -> Result<bool, StorageError> {
    match values.binary_search(&value) {
        Ok(_) => Ok(false),
        Err(at) => {
            crate::substrate::owned::reserve(values, 1)?;
            values.insert(at, value);
            Ok(true)
        }
    }
}
impl Graph {
    pub fn construct(typed: &TypedMethod<'_>, f: &StorageFacts) -> Result<Self, StorageError> {
        let s = typed.ssa_facts();
        let offset = s.points().len();
        let count = offset
            .checked_add(s.transfers().len())
            .ok_or(crate::RequestError::PlatformCapacity)?;
        let mut events = Vec::new();
        crate::substrate::owned::reserve(&mut events, count)?;
        events.resize_with(count, Event::default);
        let mut graph = Self {
            events,
            outgoing: lists(count)?,
            incoming: lists(count)?,
        };
        let mut tails = Vec::new();
        for point in 0..offset {
            push(&mut tails, point)?;
        }
        for u in &f.uses {
            if let Some(local) = u.read().ok().and_then(StorageRead::local) {
                insert(&mut graph.events[u.point.0 as usize].reads, local)?;
            }
        }
        for d in &f.definitions {
            let local = match d.materialization {
                Materialization::Store(l)
                | Materialization::ExceptionCapture(l)
                | Materialization::AllocationCarrier(l) => l,
                Materialization::Literal(_) | Materialization::Unavailable(_) => continue,
                Materialization::Deferred { .. } => {
                    return Err(StorageError::InternalInvariant(
                        "coalescing must precede construction materialization",
                    ));
                }
            };
            let source = if matches!(
                typed.execution().operations()[d.operation.0 as usize].kind(),
                OpKind::Move { .. }
            ) {
                match f.operation_uses(d.operation) {
                    Some(Ok([u])) => identity_source(f, local, u.read().ok()),
                    _ => None,
                }
            } else {
                None
            };
            push(
                &mut graph.events[d.point.0 as usize].writes,
                (local, source),
            )?;
        }
        for p in &f.copies {
            let CopyLocation::Transfer { transfer, site } = p.location else {
                continue;
            };
            let at = offset
                .checked_add(transfer.0 as usize)
                .ok_or(crate::RequestError::PlatformCapacity)?;
            if at >= count {
                return Err(StorageError::InternalInvariant(
                    "copy transfer outside P3 graph",
                ));
            }
            if let TransferSite::AfterCommit(op) = site {
                let t = &s.transfers()[transfer.0 as usize];
                let Target::Point(target) = t.target() else {
                    return Err(StorageError::InternalInvariant("commit copy has no target"));
                };
                if t.origin() != TransferOrigin::NormalCommit(op)
                    || tails[target.0 as usize] != target.0 as usize
                {
                    return Err(StorageError::InternalInvariant(
                        "commit copy has inconsistent placement",
                    ));
                }
                tails[target.0 as usize] = at;
                graph.edge(target.0 as usize, at)?;
            }
            for a in &p.assignments {
                if let Some(source) = a.source.ok().and_then(StorageRead::local) {
                    insert(&mut graph.events[at].reads, source)?;
                }
                if let Ok(destination) = a.destination {
                    let source = if a.issue.is_none() {
                        identity_source(f, destination, a.source.ok())
                    } else {
                        None
                    };
                    push(&mut graph.events[at].writes, (destination, source))?;
                }
            }
        }
        for t in s.transfers() {
            let from = tails[t.from().0 as usize];
            let transfer = offset + t.id().0 as usize; // count was checked above
            match t.target() {
                Target::Point(to) if tails[to.0 as usize] == transfer => {
                    // This bundle is after this normal commit, not before it.
                    graph.edge(from, to.0 as usize)?;
                }
                Target::Point(to) => {
                    graph.edge(from, transfer)?;
                    graph.edge(transfer, to.0 as usize)?;
                }
                Target::Exit(_) => graph.edge(from, transfer)?,
            }
        }
        Ok(graph)
    }
    fn edge(&mut self, from: usize, to: usize) -> Result<(), StorageError> {
        push(&mut self.outgoing[from], to)?;
        push(&mut self.incoming[to], from)
    }
    pub fn interference(&self, locals: usize) -> Result<Vec<Vec<LocalId>>, StorageError> {
        let mut live = lists(self.events.len())?;
        let mut queued = filled(self.events.len(), true)?;
        let mut work = Vec::new();
        for (n, incoming) in live.iter_mut().enumerate() {
            for &l in &self.events[n].reads {
                insert(incoming, l)?;
            }
            push(&mut work, n)?;
        }
        let mut out = Vec::new();
        while let Some(n) = work.pop() {
            queued[n] = false;
            out.clear();
            for &next in &self.outgoing[n] {
                for &l in &live[next] {
                    insert(&mut out, l)?;
                }
            }
            let mut changed = false;
            for &l in &out {
                if !self.events[n].writes.iter().any(|&(d, _)| d == l) {
                    changed |= insert(&mut live[n], l)?;
                }
            }
            if changed {
                for &previous in &self.incoming[n] {
                    if !queued[previous] {
                        queued[previous] = true;
                        push(&mut work, previous)?;
                    }
                }
            }
        }
        let mut conflicts = lists(locals)?;
        for (n, event) in self.events.iter().enumerate() {
            out.clear();
            for &next in &self.outgoing[n] {
                for &l in &live[next] {
                    insert(&mut out, l)?;
                }
            }
            for &(d, source) in &event.writes {
                for &l in &out {
                    if Some(l) != source {
                        conflict(&mut conflicts, d, l)?;
                    }
                }
                // Even dead destinations cannot acquire the same writer slot in
                // one simultaneous bundle. The scheduler must keep one writer.
                for &(other, _) in &event.writes {
                    conflict(&mut conflicts, d, other)?;
                }
            }
        }
        Ok(conflicts)
    }
}
fn conflict(conflicts: &mut [Vec<LocalId>], a: LocalId, b: LocalId) -> Result<(), StorageError> {
    if a != b {
        insert(&mut conflicts[a.0 as usize], b)?;
        insert(&mut conflicts[b.0 as usize], a)?;
    }
    Ok(())
}
