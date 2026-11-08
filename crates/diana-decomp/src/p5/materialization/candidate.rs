//! A declined construction remains explicit; only an actual construction
//! error stops P5. The local error type preserves that distinction across the
//! private admission steps without publishing an incomplete group.

use super::*;

pub(super) enum AdmissionError {
    Separate(Separation),
    Stop(StorageError),
}
impl From<StorageError> for AdmissionError {
    fn from(error: StorageError) -> Self {
        Self::Stop(error)
    }
}
impl From<Separation> for AdmissionError {
    fn from(reason: Separation) -> Self {
        Self::Separate(reason)
    }
}
pub(super) type Admission<T> = Result<T, AdmissionError>;

pub(super) fn candidate(
    typed: &TypedMethod<'_>,
    f: &StorageFacts,
    index: usize,
    allocation: crate::p2::OpId,
) -> Result<Result<ConstructionGroup, Separation>, StorageError> {
    match construct(typed, f, index, allocation) {
        Ok(group) => Ok(Ok(group)),
        Err(AdmissionError::Separate(reason)) => Ok(Err(reason)),
        Err(AdmissionError::Stop(error)) => Err(error),
    }
}

fn construct(
    typed: &TypedMethod<'_>,
    f: &StorageFacts,
    index: usize,
    allocation: crate::p2::OpId,
) -> Admission<ConstructionGroup> {
    let s = typed.ssa_facts();
    let constructor = f.constructions[index].fact;
    let init = constructor.operation;
    members::receiver(typed, constructor, allocation)?;
    let Corridor {
        members,
        edges: interior_edges,
    } = match corridor(typed, allocation, init)? {
        Ok(corridor) => corridor,
        Err(reason) => return Err(AdmissionError::Separate(reason)),
    };
    // One membership mask serves member admission and original-use observation.
    let mut own_member = crate::p5::build::filled(typed.execution().operations().len(), false)?;
    for &operation in &members {
        own_member[operation.0 as usize] = true;
    }
    let preparations = members::preparations(typed, f, allocation, init, &members, &own_member)?;
    members::compatible(typed, f, allocation, init, &interior_edges)?;
    let at = s
        .operation_site(allocation)
        .ok_or(StorageError::InternalInvariant(
            "allocation lost its execute point",
        ))?
        .execute();
    let (success, normal) = normal_success(typed, init)?;
    observations::before_success(typed, allocation, &own_member, success, normal)?;
    let arguments = arguments::reads(typed, f, init)?;
    let prepared_arguments =
        arguments::prepare(typed, f, allocation, init, &preparations, &arguments)?;
    snapshots::prepared_protection(
        typed,
        f,
        allocation,
        init,
        &preparations,
        &prepared_arguments,
    )?;
    let (snapshots, prelude) = snapshots::collect(typed, f, allocation, init, at)?;
    let publications = snapshots::publications(typed, f, allocation, &members)?;
    Ok(ConstructionGroup {
        id: GroupId(id(f.groups.len())?),
        allocation,
        initialization: init,
        normal,
        members,
        interior_edges,
        arguments: arguments.reads,
        prepared_arguments,
        publications,
        prelude,
        snapshot_members: snapshots,
    })
}
