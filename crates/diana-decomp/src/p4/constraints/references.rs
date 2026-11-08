//! Original reference declarations supply bounds without external resolution.

use super::*;

pub(super) fn literal(
    f: &mut TypeFacts,
    kind: &OpKind,
    out: Option<ValueId>,
) -> Result<(), TypeError> {
    if let Some(v) = out {
        let text: &[u8] = match kind {
            OpKind::ConstString { .. } => b"Ljava/lang/String;",
            OpKind::ConstClass { .. } => b"Ljava/lang/Class;",
            OpKind::ConstMethodHandle { .. } => b"Ljava/lang/invoke/MethodHandle;",
            _ => b"Ljava/lang/invoke/MethodType;",
        };
        let n = intrinsic(f, text)?;
        seed(f, v, n)?;
        f.values[v.0 as usize].identity = Identity::Root(v);
    }
    Ok(())
}

pub(super) fn instance(
    ssa: &SsaMethod<'_>,
    f: &mut TypeFacts,
    work: &mut Work,
    op: &Operation,
    out: Option<ValueId>,
) -> Result<(), TypeError> {
    let s = ssa.facts();
    let flow = ssa.execution();
    arg(f, s, op.id(), 0, mask(Family::Reference));
    if let Some(v) = out {
        primitive(f, v, b'Z')?;
    }
    // A result without a resolved type reference or an available
    // tested operand cannot establish any membership later.
    if let OpKind::InstanceOf { type_id } = op.kind()
        && out.is_some()
        && flow
            .reference_fact(op.pc(), ReferenceSlot::First)
            .is_some_and(|fact| matches!(fact, Ok(Reference::Type { .. })))
        && flow
            .reference_use_issue(op.pc(), ReferenceSlot::First)
            .is_none()
        && let Some(input) = use_index(s, op.id(), 0)
    {
        push(
            &mut work.instances,
            InstanceBound {
                operation: op.id(),
                receiver: s.uses()[input].state(),
                target: *type_id,
            },
        )?;
    }
    Ok(())
}

pub(super) fn allocation(
    ssa: &SsaMethod<'_>,
    f: &mut TypeFacts,
    work: &mut Work,
    op: &Operation,
    out: Option<ValueId>,
) -> Result<(), TypeError> {
    let s = ssa.facts();
    let flow = ssa.execution();
    if matches!(op.kind(), OpKind::Cast { .. }) {
        arg(f, s, op.id(), 0, mask(Family::Reference));
    }
    if matches!(op.kind(), OpKind::NewArray { .. }) {
        arg(f, s, op.id(), 0, mask(Family::Integral));
    }
    let name = match flow.reference_fact(op.pc(), ReferenceSlot::First) {
        Some(Ok(Reference::Type { descriptor, .. })) => Some(reference_name(
            f,
            op,
            ReferenceSlot::First,
            SignaturePart::Type,
            match op.kind() {
                OpKind::Cast { type_id }
                | OpKind::NewInstance { type_id }
                | OpKind::NewArray { type_id }
                | OpKind::FilledNewArray { type_id } => Some(*type_id),
                _ => None,
            },
            *descriptor,
        )?),
        _ => None,
    };
    if matches!(op.kind(), OpKind::Cast { .. }) {
        if let OpKind::Cast { type_id } = op.kind()
            && let Some(target) = name
            && flow
                .reference_use_issue(op.pc(), ReferenceSlot::First)
                .is_none()
            && let Some(input) = use_index(s, op.id(), 0)
        {
            push(
                &mut work.casts,
                CastBound {
                    operation: op.id(),
                    receiver: s.uses()[input].state(),
                    target,
                    target_id: *type_id,
                },
            )?;
        }
    } else if let Some(v) = out {
        f.values[v.0 as usize].families = mask(Family::Reference);
        if let Some(n) = name {
            seed(f, v, n)?;
        } else {
            f.values[v.0 as usize].type_unknown = true;
            issue(
                f,
                IssueSite::Value(v),
                Problem::Missing(MissingFact::ReferenceDeclaration),
            )?;
        }
        f.values[v.0 as usize].identity = Identity::Root(v);
    }
    if matches!(op.kind(), OpKind::FilledNewArray { .. })
        && let Some(array) = name
        && let Some(element) = crate::p4::components::declaration(f, array)?
    {
        let count = s.operation_site(op.id()).map_or(0, |p| p.use_len());
        for i in 0..count {
            arg_type(f, s, op.id(), i as usize, element)?;
        }
    }
    Ok(())
}

pub(super) fn field(
    ssa: &SsaMethod<'_>,
    f: &mut TypeFacts,
    op: &Operation,
    out: Option<ValueId>,
    access: FieldAccess,
    static_field: bool,
    kind: ValueKind,
) -> Result<(), TypeError> {
    let s = ssa.facts();
    let flow = ssa.execution();
    let get = access == FieldAccess::Get;
    if !static_field {
        arg(f, s, op.id(), usize::from(!get), mask(Family::Reference));
    }
    if !get {
        arg(f, s, op.id(), 0, value_kind(kind));
    }
    if let Some(v) = out {
        f.values[v.0 as usize].families =
            f.values[v.0 as usize].families.intersect(value_kind(kind));
        if kind == ValueKind::Object {
            f.values[v.0 as usize].identity = Identity::Root(v);
        }
        if let Some(tag) = narrow_tag(kind) {
            primitive(f, v, tag)?;
        }
    }
    if let Some(Ok(Reference::Field {
        owner,
        owner_type,
        field_type,
        field_type_id,
        ..
    })) = flow.reference_fact(op.pc(), ReferenceSlot::First)
    {
        let n = reference_name(
            f,
            op,
            ReferenceSlot::First,
            SignaturePart::Field,
            Some(*field_type_id),
            *field_type,
        )?;
        if get {
            if let Some(v) = out {
                seed(f, v, n)?;
            }
        } else {
            arg_type(f, s, op.id(), 0, n)?;
        }
        if !static_field {
            let owner = reference_name(
                f,
                op,
                ReferenceSlot::First,
                SignaturePart::Owner,
                Some(*owner_type),
                *owner,
            )?;
            arg_type(f, s, op.id(), usize::from(!get), owner)?;
        }
    } else {
        missing_args(f, s, op.id());
        if let Some(v) = out {
            f.values[v.0 as usize].type_unknown = true;
        }
    }
    Ok(())
}

pub(super) fn invoke(
    ssa: &SsaMethod<'_>,
    f: &mut TypeFacts,
    op: &Operation,
    out: Option<ValueId>,
    kind: InvokeKind,
) -> Result<(), TypeError> {
    let s = ssa.facts();
    let flow = ssa.execution();
    let slot = if kind == InvokeKind::Polymorphic {
        ReferenceSlot::Second
    } else {
        ReferenceSlot::First
    };
    let prototype = match flow.reference_fact(op.pc(), slot) {
        Some(Ok(
            Reference::Method { prototype, .. }
            | Reference::Proto { prototype }
            | Reference::CallSite { prototype, .. },
        )) => Some(prototype),
        _ => None,
    };
    let receiver = !matches!(kind, InvokeKind::Static | InvokeKind::Custom);
    if receiver {
        arg(f, s, op.id(), 0, mask(Family::Reference));
    }
    if receiver
        && let Some(Ok(Reference::Method {
            owner, owner_type, ..
        })) = flow.reference_fact(op.pc(), ReferenceSlot::First)
    {
        let n = reference_name(
            f,
            op,
            ReferenceSlot::First,
            SignaturePart::Owner,
            Some(*owner_type),
            *owner,
        )?;
        arg_type(f, s, op.id(), 0, n)?;
    }
    if let Some(proto) = prototype {
        for (i, p) in proto.parameters().iter().enumerate() {
            let n = reference_name(
                f,
                op,
                slot,
                SignaturePart::Parameter(id(i)?),
                Some(p.type_id()),
                p.descriptor(),
            )?;
            arg_type(f, s, op.id(), i + usize::from(receiver), n)?;
        }
        if let Some(v) = out {
            let n = reference_name(
                f,
                op,
                slot,
                SignaturePart::Return,
                Some(proto.return_type_id()),
                proto.return_type(),
            )?;
            seed(f, v, n)?;
            if name_family(f, n) == Some(Family::Reference) {
                f.values[v.0 as usize].identity = Identity::Root(v);
            }
        }
    } else {
        missing_args(f, s, op.id());
        if let Some(v) = out {
            f.values[v.0 as usize].type_unknown = true;
            issue(
                f,
                IssueSite::Value(v),
                Problem::Missing(MissingFact::ReferenceDeclaration),
            )?;
        }
    }
    if matches!(op.kind(), OpKind::UnresolvedInvoke { .. }) {
        issue(
            f,
            IssueSite::Operation(op.id()),
            Problem::Missing(MissingFact::ArgumentLayout),
        )?;
    }
    Ok(())
}
