//! Operation requirements consume existing execution facts and original references.

use super::*;

pub(super) fn array(
    ssa: &SsaMethod<'_>,
    f: &mut TypeFacts,
    work: &mut Work,
    op: &Operation,
    out: Option<ValueId>,
    access: ArrayAccess,
    kind: ValueKind,
) -> Result<(), TypeError> {
    let s = ssa.facts();
    let get = access == ArrayAccess::Get;
    let array = usize::from(!get);
    let index = use_index(s, op.id(), array).ok_or(TypeError::InternalInvariant(
        "array operation lost its fixed SSA input layout",
    ))?;
    push(
        &mut work.arrays,
        ArraySite {
            operation: op.id(),
            array: s.uses()[index].state(),
            output: out,
        },
    )?;
    arg(f, s, op.id(), array, mask(Family::Reference));
    arg(f, s, op.id(), array + 1, mask(Family::Integral));
    if !get {
        arg(f, s, op.id(), 0, value_kind(kind));
        if matches!(kind, ValueKind::Word | ValueKind::Wide)
            && let Some(i) = use_index(s, op.id(), 0)
        {
            f.uses[i].requirement.meaning = ReadMeaning::CopyBits;
        }
    }
    if let Some(v) = out {
        f.values[v.0 as usize].families =
            f.values[v.0 as usize].families.intersect(value_kind(kind));
        if let Some(tag) = narrow_tag(kind) {
            primitive(f, v, tag)?;
        }
        if kind == ValueKind::Object {
            f.values[v.0 as usize].identity = Identity::Root(v);
        }
    }
    Ok(())
}

pub(super) fn collect(
    ssa: &SsaMethod<'_>,
    f: &mut TypeFacts,
    work: &mut Work,
) -> Result<(), TypeError> {
    let s = ssa.facts();
    let flow = ssa.execution();
    for op in flow.operations() {
        let out = output(s, op.id());
        match op.kind() {
            OpKind::Nop | OpKind::UnresolvedResult { .. } => {}
            OpKind::Const { bits, width } => scalars::constant(f, out, *bits, *width)?,
            OpKind::Move { kind } => scalars::copy(ssa, f, work, op.id(), *kind, out)?,
            OpKind::ConstString { .. }
            | OpKind::ConstClass { .. }
            | OpKind::ConstMethodHandle { .. }
            | OpKind::ConstMethodType { .. } => references::literal(f, op.kind(), out)?,
            OpKind::Unary { op: unary } => scalars::unary(s, f, op.id(), *unary, out)?,
            OpKind::Binary { op: binary, .. } => scalars::binary(s, f, op.id(), *binary, out)?,
            OpKind::Compare { op: cmp } => scalars::compare(s, f, op.id(), *cmp, out)?,
            OpKind::Array { access, kind } => array(ssa, f, work, op, out, *access, *kind)?,
            OpKind::ArrayLength => {
                arg(f, s, op.id(), 0, mask(Family::Reference));
                if let Some(v) = out {
                    primitive(f, v, b'I')?;
                    f.values[v.0 as usize].range = Some(IntegralRange {
                        min: 0,
                        max: i64::from(i32::MAX),
                    });
                }
            }
            OpKind::InstanceOf { .. } => references::instance(ssa, f, work, op, out)?,
            OpKind::Cast { .. }
            | OpKind::NewInstance { .. }
            | OpKind::NewArray { .. }
            | OpKind::FilledNewArray { .. } => references::allocation(ssa, f, work, op, out)?,
            OpKind::Field {
                access,
                static_field,
                kind,
                ..
            } => references::field(ssa, f, op, out, *access, *static_field, *kind)?,
            OpKind::Invoke { kind, .. } | OpKind::UnresolvedInvoke { kind, .. } => {
                references::invoke(ssa, f, op, out, *kind)?
            }
            OpKind::FillArrayData { .. } => {
                let index = use_index(s, op.id(), 0).ok_or(TypeError::InternalInvariant(
                    "array fill lost its fixed SSA input layout",
                ))?;
                push(
                    &mut work.arrays,
                    ArraySite {
                        operation: op.id(),
                        array: s.uses()[index].state(),
                        output: None,
                    },
                )?;
                arg(f, s, op.id(), 0, mask(Family::Reference));
            }
            OpKind::Monitor { .. } => arg(f, s, op.id(), 0, mask(Family::Reference)),
            OpKind::MoveException => {}
        }
        if flow
            .reference_use_issue(op.pc(), ReferenceSlot::First)
            .is_some()
        {
            issue(
                f,
                IssueSite::Operation(op.id()),
                Problem::OriginalUseIssue(op.id()),
            )?;
        }
    }
    Ok(())
}
