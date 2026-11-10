//! Deferral requires every original allocation read to follow normal success.

use super::candidate::Admission;
use super::*;
pub(super) fn before_success(
    typed: &TypedMethod<'_>,
    allocation: crate::p2::OpId,
    own_member: &[bool],
    success: TransferId,
    normal: PointId,
) -> Admission<()> {
    let s = typed.ssa_facts();
    let normal_single = s
        .transfers()
        .iter()
        .filter(|t| t.target() == Target::Point(normal))
        .count()
        == 1;
    let mut unsafe_reach = None;
    let mut visited = filled(s.phis().len(), false)?;
    for usage in s.uses() {
        let Some(value) = value(s, usage.state()) else {
            continue;
        };
        if !typed
            .facts()
            .value(value)
            .is_some_and(|v| v.allocations().contains(&allocation))
        {
            continue;
        }
        let own = matches!(
            usage.site(),
            UseSite::Operation { operation, .. } if own_member[operation.0 as usize]
        );
        // The use either belongs to one of the construction's own members, or is
        // reached only after its single normal completion.
        let after_initialization = own || (normal_single && s.dominates(normal, usage.point()));
        if !after_initialization {
            if unsafe_reach.is_none() {
                unsafe_reach = Some(without_success(typed, success)?);
            }
            visited.fill(false);
            if !arrives_unconstructed(
                typed,
                allocation,
                unsafe_reach
                    .as_ref()
                    .ok_or(StorageError::InternalInvariant(
                        "construction lost its unsafe reachability",
                    ))?,
                success,
                usage.point(),
                value,
                &mut visited,
            )? {
                continue;
            }
            let origin = typed
                .facts()
                .initialization()
                .iter()
                .find(|u| u.site == usage.site())
                .and_then(|u| u.allocation);
            if !origin.is_some_and(|a| {
                a.operation == allocation && a.relation == AllocationRelation::Different
            }) {
                return Err(Separation::ObservedBeforeSuccess(usage.site()).into());
            }
        }
    }
    Ok(())
}
