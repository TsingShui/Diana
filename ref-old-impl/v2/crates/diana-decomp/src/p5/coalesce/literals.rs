//! Materialize numeric constant definitions only when copy-affinity reuse can
//! share their storage. Ordinary literal reads remain folded. Definitions and
//! normal copies keep their original positions.

use super::*;
use crate::p3::ValueOrigin;
use crate::p4::{Bits, Family};

struct Promotion {
    value: ValueId,
    local: LocalId,
    bits: Bits,
}

pub(super) struct Promotions {
    original_locals: usize,
    values: Vec<Promotion>,
}

fn redirect_literal(read: &mut Result<StorageRead, StorageGap>, bindings: &[StorageBinding]) {
    if let Ok(StorageRead::Literal {
        value, state, view, ..
    }) = *read
        && let Some(local) = bindings[value.0 as usize].local()
    {
        *read = Ok(StorageRead::Value {
            local,
            value,
            state,
            view,
        });
    }
}

pub(super) fn prepare(
    typed: &TypedMethod<'_>,
    f: &mut StorageFacts,
) -> Result<Promotions, StorageError> {
    let original_locals = f.locals.len();
    let mut candidates = Vec::new();
    for plan in &f.copies {
        if !plan.is_complete() || !matches!(plan.location, CopyLocation::Transfer { .. }) {
            continue;
        }
        for assignment in &plan.assignments {
            let (
                Ok(destination),
                Ok(StorageRead::Literal {
                    value,
                    bits,
                    view: Interpretation::Literal { family, .. },
                    ..
                }),
            ) = (assignment.destination, assignment.source)
            else {
                continue;
            };
            if family == Family::Reference
                || typed
                    .facts()
                    .value(value)
                    .is_none_or(|v| !v.families().contains(family))
                || f.locals[destination.0 as usize].storage_type
                    != (StorageType::Family {
                        family,
                        width: bits.width,
                    })
            {
                continue;
            }
            let ValueOrigin::Operation { operation, .. } =
                typed.ssa_facts().values()[value.0 as usize].origin()
            else {
                continue;
            };
            if !matches!(
                typed.execution().operations()[operation.0 as usize].kind(),
                OpKind::Const { .. }
            ) {
                continue;
            }
            let Some(other) = value_of(&f.locals[destination.0 as usize]) else {
                continue;
            };
            let (Some(a), Some(b)) = (typed.facts().value(value), typed.facts().value(other))
            else {
                continue;
            };
            if a.has_unknown_type_source() || b.has_unknown_type_source() {
                continue;
            }
            if same_type_sources(a.types(), b.types())? || compatible(typed, value, other, family) {
                if candidates.is_empty() {
                    candidates = filled(f.bindings.len(), None)?;
                }
                candidates[value.0 as usize] = Some((bits, family));
            }
        }
    }
    if candidates.is_empty() {
        return Ok(Promotions {
            original_locals,
            values: Vec::new(),
        });
    }
    for access in &f.uses {
        if let Some(value) = value(typed.ssa_facts(), access.state)
            && let Some((_, family)) = candidates[value.0 as usize]
            && !matches!(access.read(), Ok(StorageRead::Literal { view, .. }) if same_view(view, family))
        {
            candidates[value.0 as usize] = None;
        }
    }
    for transfer in &f.transfers {
        if let Some(value) = value(typed.ssa_facts(), transfer.source)
            && let Some((_, family)) = candidates[value.0 as usize]
            && !same_view(transfer.view, family)
        {
            candidates[value.0 as usize] = None;
        }
    }
    let mut values = Vec::new();
    for (index, candidate) in candidates.into_iter().enumerate() {
        let Some((bits, family)) = candidate else {
            continue;
        };
        let value = ValueId(id(index)?);
        let local = local(
            f,
            StorageType::Family {
                family,
                width: bits.width,
            },
            LocalKind::Value(value),
        )?;
        f.bindings[index] = StorageBinding::Local(local);
        push(&mut values, Promotion { value, local, bits })?;
    }
    for definition in &mut f.definitions {
        if matches!(definition.materialization, Materialization::Literal(_))
            && matches!(
                typed.execution().operations()[definition.operation.0 as usize].kind(),
                OpKind::Const { .. }
            )
            && let Some(value) = value(typed.ssa_facts(), definition.state)
            && let Some(local) = f.bindings[value.0 as usize].local()
        {
            definition.materialization = Materialization::Store(local);
        }
    }
    for plan in &mut f.copies {
        if !matches!(plan.location, CopyLocation::Transfer { .. }) {
            continue;
        }
        for assignment in &mut plan.assignments {
            let was_literal = matches!(assignment.source, Ok(StorageRead::Literal { .. }));
            redirect_literal(&mut assignment.source, &f.bindings);
            if was_literal {
                assignment.dependency = assignment.source.ok().and_then(StorageRead::local);
            }
        }
    }
    Ok(Promotions {
        original_locals,
        values,
    })
}

impl Promotions {
    pub fn restore_independent(&self, f: &mut StorageFacts, parents: Option<&[LocalId]>) {
        if self.values.is_empty() {
            return;
        }
        for promotion in &self.values {
            if parents.is_none_or(|p| p[promotion.local.0 as usize] == promotion.local) {
                f.bindings[promotion.value.0 as usize] = StorageBinding::Literal(promotion.bits);
            }
        }
        for definition in &mut f.definitions {
            if let Materialization::Store(local) = definition.materialization
                && let Some(promotion) = (local.0 as usize)
                    .checked_sub(self.original_locals)
                    .and_then(|index| self.values.get(index))
                && matches!(
                    f.bindings[promotion.value.0 as usize],
                    StorageBinding::Literal(_)
                )
            {
                definition.materialization = Materialization::Literal(promotion.bits);
            }
        }
        for plan in &mut f.copies {
            for assignment in &mut plan.assignments {
                restore_read(&mut assignment.source, &f.bindings);
                if matches!(assignment.source, Ok(StorageRead::Literal { .. })) {
                    assignment.dependency = None;
                }
            }
        }
    }

    pub fn finish(self, f: &mut StorageFacts) {
        // A retained promotion joins an original destination. The minimum-id
        // representative is therefore original; all temporary candidate locals
        // have either been redirected or restored before publication.
        f.locals.truncate(self.original_locals);
        f.local_gaps.truncate(self.original_locals);
    }
}

fn same_view(view: Interpretation, family: Family) -> bool {
    matches!(view, Interpretation::Literal { family: observed, .. } if observed == family)
        || matches!(view, Interpretation::LiteralBits(_))
}

/// An undeclared numeric constant can use the destination's canonical primitive
/// storage. No reference/narrow declaration or unknown type source is imported.
pub(super) fn compatible(
    typed: &TypedMethod<'_>,
    literal: ValueId,
    other: ValueId,
    family: Family,
) -> bool {
    let (Some(a), Some(b)) = (typed.facts().value(literal), typed.facts().value(other)) else {
        return false;
    };
    if a.literal().is_none()
        || !a.types().is_empty()
        || a.has_unknown_type_source()
        || b.has_unknown_type_source()
    {
        return false;
    }
    let descriptor = match family {
        Family::Integral => b'I',
        Family::Long => b'J',
        Family::Float => b'F',
        Family::Double => b'D',
        Family::Reference => return false,
    };
    b.types().iter().all(|&name| {
        typed
            .facts()
            .name(name)
            .is_some_and(|name| name.units() == [u16::from(descriptor)])
    })
}

fn restore_read(read: &mut Result<StorageRead, StorageGap>, bindings: &[StorageBinding]) {
    if let Ok(StorageRead::Value {
        value, state, view, ..
    }) = *read
        && let StorageBinding::Literal(bits) = bindings[value.0 as usize]
    {
        *read = Ok(StorageRead::Literal {
            value,
            state,
            bits,
            view,
        });
    }
}
