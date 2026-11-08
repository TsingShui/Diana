//! Initializes one record per original value/use and retains declaration identities.

use super::*;

pub(super) fn records(ssa: &SsaMethod<'_>, f: &mut TypeFacts) -> Result<(), TypeError> {
    let s = ssa.facts();
    for v in s.values() {
        let width = s
            .width(v.id())
            .ok_or(TypeError::InternalInvariant("SSA value lost its width"))?;
        push(
            &mut f.values,
            ValueFacts {
                value: v.id(),
                families: Families::width(width),
                types: Vec::new(),
                type_unknown: false,
                literal: None,
                mixed_literal: true,
                range: None,
                range_unknown: true,
                identity: Identity::Unknown,
                allocations: Vec::new(),
            },
        )?;
    }
    for u in s.uses() {
        let width = s
            .slot(u.slot())
            .ok_or(TypeError::InternalInvariant("SSA use lost its slot"))?
            .width;
        push(
            &mut f.uses,
            UseView {
                site: u.site(),
                state: u.state(),
                requirement: Requirement {
                    meaning: ReadMeaning::Value,
                    families: Families::width(width),
                    types: Vec::new(),
                    missing: None,
                },
                interpretation: Interpretation::Unknown(Problem::ConstructionPending),
            },
        )?;
    }
    Ok(())
}

pub(super) fn declarations(
    ssa: &SsaMethod<'_>,
    f: &mut TypeFacts,
) -> Result<(Vec<TypeNameId>, TypeNameId), TypeError> {
    let flow = ssa.execution();
    let decl = flow.declaration();
    let mut params = Vec::new();
    for (i, ty) in decl.parameter_types.iter().enumerate() {
        let original = *decl
            .parameter_type_ids
            .get(i)
            .ok_or(TypeError::InternalInvariant(
                "method declaration lost a parameter identity",
            ))?;
        let n = add_name(
            f,
            copy_units(ty.units())?,
            TypeOrigin::Parameter {
                index: id(i)?,
                original: crate::TypeId(original),
            },
        )?;
        push(&mut params, n)?;
    }
    let returned = add_name(
        f,
        copy_units(decl.return_type.units())?,
        TypeOrigin::Return {
            original: crate::TypeId(decl.return_type_id),
        },
    )?;
    Ok((params, returned))
}
