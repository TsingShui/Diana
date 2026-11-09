//! Original value uses and execution storage are separate composition facts.

use super::super::*;
use super::*;
use crate::p3::{StateId, ValueId};
use crate::p5::{Materialization, StorageRead};
use std::collections::HashMap;

#[derive(Clone, Copy, Default)]
pub(super) enum LocalProtection {
    #[default]
    None,
    /// An original normal-edge copy still needs the committed value.
    CopySource,
    /// Snapshots and non-edge phases keep the definition at its own statement.
    Pinned,
}
#[derive(Clone, Copy, Default)]
pub(super) struct ValueAccess {
    pub reads: usize,
    pub protection: LocalProtection,
    definition: Option<(StateId, StatementId)>,
    ambiguous: bool,
}

/// One index over delivered reads/definitions, not another SSA or alias analysis.
pub(super) struct LocalAccess {
    pub reads: Vec<u8>,
    pub protection: Vec<LocalProtection>,
    pub complete: bool,
    values: HashMap<ValueId, ValueAccess>,
}
impl LocalAccess {
    pub fn value(&self, value: ValueId) -> Option<ValueAccess> {
        self.complete
            .then(|| self.values.get(&value).copied())
            .flatten()
    }
    pub fn copy(&self) -> Result<Self, ExpressionError> {
        let mut values = HashMap::new();
        values
            .try_reserve(self.values.len())
            .map_err(crate::RequestError::Allocation)?;
        values.extend(self.values.iter().map(|(&id, &access)| (id, access)));
        Ok(Self {
            reads: super::super::build::copy(&self.reads)?,
            protection: super::super::build::copy(&self.protection)?,
            complete: self.complete,
            values,
        })
    }
    pub fn ignore_read(&mut self, value: ValueId) {
        if let Some(access) = self.values.get_mut(&value) {
            access.reads = access.reads.saturating_sub(1);
        }
    }
    fn at(&mut self, value: ValueId) -> Result<&mut ValueAccess, ExpressionError> {
        if !self.values.contains_key(&value) {
            self.values
                .try_reserve(1)
                .map_err(crate::RequestError::Allocation)?;
        }
        Ok(self.values.entry(value).or_default())
    }
    /// The destination state belongs to one delivered pure Move of this exact
    /// source read. This is the same original correspondence for every carrier.
    pub fn copied_read(
        &self,
        code: &MethodCode,
        source: (ValueId, StateId),
        occurrence: ExprId,
        storage: StorageId,
    ) -> bool {
        let Some((value, state)) = read_of(code, occurrence) else {
            return false;
        };
        let Some(definition) = self
            .values
            .get(&value)
            .filter(|definition| !definition.ambiguous)
        else {
            return false;
        };
        let Some((published, statement)) = definition.definition else {
            return false;
        };
        if published != state {
            return false;
        }
        let statement = &code.statements[statement.0 as usize];
        let StatementKind::Move { expression, .. } = statement.kind else {
            return false;
        };
        matches!(statement.outputs.as_slice(), [output]
            if output.value() == Some(value)
                && matches!(output.materialization(), Materialization::Store(local) if local.0 == storage.0))
            && read_of(code, expression) == Some(source)
    }
}

pub(super) fn read_of(code: &MethodCode, expression: ExprId) -> Option<(ValueId, StateId)> {
    match code
        .expressions
        .get(expression.0 as usize)?
        .origin
        .as_ref()?
        .read
        .ok()?
    {
        StorageRead::Value { value, state, .. } => Some((value, state)),
        _ => None,
    }
}

pub(super) fn local_access(code: &MethodCode) -> Result<LocalAccess, ExpressionError> {
    let mut access = LocalAccess {
        reads: filled(code.locals.len(), 0u8)?,
        protection: filled(code.locals.len(), LocalProtection::None)?,
        complete: true,
        values: HashMap::new(),
    };
    for expression in &code.expressions {
        let storage = match expression.kind {
            ExprKind::Read { storage, .. } | ExprKind::Snapshot { storage } => Some(storage),
            _ => None,
        };
        if let Some(storage) = storage {
            let count = access
                .reads
                .get_mut(storage.0 as usize)
                .ok_or(bug("expression names an unknown local"))?;
            *count = count.saturating_add(1).min(2);
            if matches!(expression.kind, ExprKind::Snapshot { .. }) {
                mark(&mut access.protection, storage, LocalProtection::Pinned)?;
            }
        }
        if let Some((value, _)) = read_of(code, expression.id) {
            let value = access.at(value)?;
            value.reads += 1;
        }
    }
    for statement in &code.statements {
        for output in &statement.outputs {
            if let Some(value) = output.value() {
                let definition = access.at(value)?;
                let site = (output.state(), statement.id);
                definition.ambiguous |= definition
                    .definition
                    .replace(site)
                    .is_some_and(|old| old != site);
            }
        }
    }
    for copy in &code.copies {
        let protection = if matches!(copy.emission, CopyEmission::Original)
            && matches!(
                copy.location,
                crate::p5::CopyLocation::Transfer {
                    site: crate::p5::TransferSite::NormalEdge(_),
                    ..
                }
            ) {
            LocalProtection::CopySource
        } else {
            LocalProtection::Pinned
        };
        for assignment in &copy.assignments {
            access.complete &= assignment.issue().is_none();
            if let Ok(destination) = assignment.destination() {
                mark(
                    &mut access.protection,
                    StorageId(destination.0),
                    LocalProtection::Pinned,
                )?;
            }
            if let Ok(source) = assignment.source() {
                protect_read(&mut access, source, protection)?;
            }
        }
        for step in &copy.steps {
            match *step {
                CopyStatement::Save {
                    destination,
                    source,
                } => {
                    mark(&mut access.protection, destination, LocalProtection::Pinned)?;
                    mark(&mut access.protection, source, protection)?;
                }
                CopyStatement::Assign {
                    destination,
                    original,
                    ..
                } => {
                    mark(&mut access.protection, destination, LocalProtection::Pinned)?;
                    protect_read(&mut access, original, protection)?;
                }
            }
        }
    }
    for group in &code.constructions {
        for publication in &group.publications {
            mark(
                &mut access.protection,
                publication.destination,
                LocalProtection::Pinned,
            )?;
            access.at(publication.original.value)?.protection = LocalProtection::Pinned;
        }
    }
    Ok(access)
}

fn protect_read(
    access: &mut LocalAccess,
    read: StorageRead,
    protection: LocalProtection,
) -> Result<(), ExpressionError> {
    if let Some(local) = read.local() {
        mark(&mut access.protection, StorageId(local.0), protection)?;
    }
    if let StorageRead::Value { value, .. } = read {
        let target = &mut access.at(value)?.protection;
        if matches!(target, LocalProtection::None) || matches!(protection, LocalProtection::Pinned)
        {
            *target = protection;
        }
    }
    Ok(())
}
fn mark(
    protection: &mut [LocalProtection],
    id: StorageId,
    involvement: LocalProtection,
) -> Result<(), ExpressionError> {
    let slot = protection
        .get_mut(id.0 as usize)
        .ok_or(bug("copy names an unknown local"))?;
    if matches!(slot, LocalProtection::None) || matches!(involvement, LocalProtection::Pinned) {
        *slot = involvement;
    }
    Ok(())
}
