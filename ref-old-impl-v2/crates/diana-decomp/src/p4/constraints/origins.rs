//! Seeds entry, Phi and exception values from delivered SSA and catch facts.

use super::*;

pub(super) fn handler_types(
    ssa: &SsaMethod<'_>,
) -> Result<Vec<Vec<Option<crate::TypeId>>>, TypeError> {
    let flow = ssa.execution();
    let mut handler_types = if flow.catch_types().is_empty() {
        Vec::new()
    } else {
        lists(flow.blocks().len())?
    };
    if !handler_types.is_empty() {
        for edge in flow.exception_edges() {
            if let EdgeTarget::Block(handler) = edge.target()
                && let EdgeRole::Exception { catch, .. } = edge.role()
                && !handler_types[handler.0 as usize].contains(&catch)
            {
                push(&mut handler_types[handler.0 as usize], catch)?;
            }
        }
    }
    Ok(handler_types)
}

fn exception(
    ssa: &SsaMethod<'_>,
    f: &mut TypeFacts,
    value: ValueId,
    operation: OpId,
    handler_types: &[Vec<Option<crate::TypeId>>],
) -> Result<(), TypeError> {
    let flow = ssa.execution();
    f.values[value.0 as usize].identity = Identity::Root(value);
    f.values[value.0 as usize].families = mask(Family::Reference);
    let handler = flow.operations()[operation.0 as usize].block();
    let catches = handler_types
        .get(handler.0 as usize)
        .map_or(&[][..], Vec::as_slice);
    for &catch in catches {
        let name = if let Some(original) = catch {
            let fact = flow
                .catch_type(original)
                .ok_or(TypeError::InternalInvariant(
                    "exception edge has no original catch-type fact",
                ))?;
            if fact.use_issue().is_some() {
                f.values[value.0 as usize].type_unknown = true;
                issue(
                    f,
                    IssueSite::Value(value),
                    Problem::InvalidCatchType(original),
                )?;
                continue;
            }
            match fact.descriptor() {
                Ok(descriptor) => {
                    if let Some(i) = f
                        .names
                        .iter()
                        .position(|n| n.origin == TypeOrigin::Catch { original })
                    {
                        TypeNameId(id(i)?)
                    } else {
                        add_name(
                            f,
                            owned::copy_units(descriptor)?,
                            TypeOrigin::Catch { original },
                        )?
                    }
                }
                Err(_) => {
                    f.values[value.0 as usize].type_unknown = true;
                    issue(
                        f,
                        IssueSite::Value(value),
                        Problem::Missing(MissingFact::CatchDescriptor),
                    )?;
                    continue;
                }
            }
        } else {
            intrinsic(f, b"Ljava/lang/Throwable;")?
        };
        if !f.values[value.0 as usize].types.contains(&name) {
            seed(f, value, name)?;
        }
    }
    if f.values[value.0 as usize].types.is_empty() {
        let throwable = intrinsic(f, b"Ljava/lang/Throwable;")?;
        seed(f, value, throwable)?;
    }
    Ok(())
}

pub(super) fn values(
    ssa: &SsaMethod<'_>,
    f: &mut TypeFacts,
    work: &mut Work,
    params: &[TypeNameId],
    handler_types: &[Vec<Option<crate::TypeId>>],
) -> Result<(), TypeError> {
    let s = ssa.facts();
    let flow = ssa.execution();
    for v in s.values() {
        match v.origin() {
            ValueOrigin::Entry(EntryRole::Parameter(i)) => {
                let name = *params.get(i as usize).ok_or(TypeError::InternalInvariant(
                    "SSA parameter has no declaration",
                ))?;
                seed(f, v.id(), name)?;
                if name_family(f, name) == Some(Family::Reference) {
                    f.values[v.id().0 as usize].identity = Identity::Root(v.id());
                }
            }
            ValueOrigin::Entry(EntryRole::This) => {
                let owner = flow.owner_facts();
                let name = add_name(
                    f,
                    copy_units(owner.descriptor().units())?,
                    TypeOrigin::Owner {
                        key: owner.key(),
                        original: owner.type_id(),
                    },
                )?;
                seed(f, v.id(), name)?;
                f.values[v.id().0 as usize].identity = Identity::Root(v.id());
            }
            ValueOrigin::Phi(p) => {
                let mut inputs = Vec::new();
                for input in s.phis()[p.0 as usize].incoming() {
                    push(&mut inputs, input.state())?;
                }
                let fact = &mut f.values[v.id().0 as usize];
                fact.mixed_literal = false;
                fact.range_unknown = false;
                push(
                    &mut work.rules,
                    Rule {
                        target: v.id(),
                        inputs,
                        kind: RuleKind::Phi,
                    },
                )?;
            }
            ValueOrigin::Exception(operation) => {
                exception(ssa, f, v.id(), operation, handler_types)?
            }
            ValueOrigin::Operation { .. } => {}
        }
    }
    Ok(())
}
