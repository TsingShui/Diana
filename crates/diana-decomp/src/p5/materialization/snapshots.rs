//! Retains common save obligations and publications after constructor success.

use super::candidate::Admission;
use super::*;
pub(super) fn collect(
    typed: &TypedMethod<'_>,
    f: &StorageFacts,
    members: &[crate::p2::OpId],
    at: PointId,
) -> Admission<(Vec<PlanId>, Option<PlanId>)> {
    let mut snapshots = Vec::new();
    for plan in &f.copies {
        if matches!(plan.location,CopyLocation::BeforeThrow(ThrowSite::Operation(op))if members.contains(&op))
        {
            if !plan.is_complete() {
                return Err(Separation::SnapshotMismatch(plan.id).into());
            }
            push(&mut snapshots, plan.id)?;
        }
    }
    let prelude = snapshots.first().copied();
    if let Some(first) = prelude {
        let base = &f.copies[first.0 as usize];
        for a in &base.assignments {
            if !a.source.is_ok_and(|r| source_ready(typed, r, at)) {
                return Err(Separation::SnapshotMismatch(first).into());
            }
        }
        for &p in snapshots.iter().skip(1) {
            let other = &f.copies[p.0 as usize];
            if base.assignments.len() != other.assignments.len()
                || !base.assignments.iter().all(|a| {
                    other.assignments.iter().any(|b| {
                        a.destination == b.destination
                            && matches!((a.source,b.source),(Ok(a),Ok(b)) if same_read(a,b))
                    })
                })
            {
                return Err(Separation::SnapshotMismatch(p).into());
            }
        }
        // Every throwing stage must have the same selected save obligations.
        for &op in members.iter().filter(|op| {
            typed.execution().operations()[op.0 as usize]
                .effects()
                .may_throw()
        }) {
            if !snapshots.iter().any(|p|matches!(f.copies[p.0 as usize].location,CopyLocation::BeforeThrow(ThrowSite::Operation(o))if o==op)){return Err(Separation::SnapshotMismatch(first).into());}
        }
    }
    Ok((snapshots, prelude))
}

pub(super) fn publications(
    typed: &TypedMethod<'_>,
    f: &StorageFacts,
    allocation: crate::p2::OpId,
    members: &[crate::p2::OpId],
) -> Admission<Vec<Publication>> {
    let s = typed.ssa_facts();
    // Only the allocated object and its transparent aliases become visible when
    // the constructor succeeds.
    let mut publications = Vec::new();
    for &op in members {
        if op != allocation && !aliases_allocation(typed, allocation, op) {
            continue;
        }
        for output in s
            .operation_outputs(op)
            .ok_or(StorageError::InternalInvariant(
                "construction member lost outputs",
            ))?
        {
            if let Some(v) = s.output_value(output)
                && let Some(local) = f.binding(v).and_then(StorageBinding::local)
            {
                if let Some(gap) = f.local_gap(local) {
                    return Err(Separation::MissingStorage(gap).into());
                }
                push(&mut publications, Publication { value: v, local })?;
            }
        }
    }
    Ok(publications)
}
