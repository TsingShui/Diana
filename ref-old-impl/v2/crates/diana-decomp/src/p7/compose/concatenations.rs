//! Selects a local StringConcatFactory view from owned references and P4 views.
//! No input decoding, linkage replay, external declarations or value fabrication.

use super::super::*;
use crate::p4::{Family, Initialization, Interpretation, TypeFacts};

fn named(code: &MethodCode, ty: TypeUseId, name: &str) -> bool {
    code.types
        .get(ty.0 as usize)
        .is_some_and(|ty| ty.units.iter().copied().eq(name.encode_utf16()))
}
fn units(code: &MethodCode, reference: ReferenceId) -> Option<&[u16]> {
    match &code.references.get(reference.0 as usize)?.declaration {
        Ok(ReferenceDeclaration::String { units }) => Some(units),
        _ => None,
    }
}
fn string_constant(code: &MethodCode, value: &Bootstrap) -> Option<ReferenceId> {
    let Bootstrap::Value {
        value: BootstrapLiteral::Reference { declaration, .. },
        ..
    } = value
    else {
        return None;
    };
    units(code, *declaration).map(|_| *declaration)
}
fn push(parts: &mut Vec<ConcatPart>, part: ConcatPart) -> Result<(), ConcatUnavailable> {
    parts
        .try_reserve(1)
        .map_err(|_| ConcatUnavailable::ResourceExhausted)?;
    parts.push(part);
    Ok(())
}

fn constant_part(
    code: &MethodCode,
    value: &Bootstrap,
    event: usize,
) -> Result<ConcatPart, ConcatUnavailable> {
    if let Some(reference) = string_constant(code, value) {
        return Ok(ConcatPart::Text {
            reference,
            units: 0..units(code, reference)
                .ok_or(ConcatUnavailable::BootstrapConstant)?
                .len(),
        });
    }
    let Bootstrap::Value { value, tag, .. } = value else {
        return Err(ConcatUnavailable::BootstrapConstant);
    };
    let mut units = Vec::new();
    if let BootstrapLiteral::Unsigned(value) = value
        && *tag == 0x03
    {
        let unit = u16::try_from(*value).map_err(|_| ConcatUnavailable::BootstrapConstant)?;
        units
            .try_reserve(1)
            .map_err(|_| ConcatUnavailable::ResourceExhausted)?;
        units.push(unit);
    } else {
        let text = match value {
            BootstrapLiteral::Signed(value) => {
                super::super::references::text(format_args!("{value}"))
            }
            BootstrapLiteral::Boolean(value) => {
                super::super::references::text(format_args!("{value}"))
            }
            _ => return Err(ConcatUnavailable::BootstrapConstant),
        }
        .map_err(|_| ConcatUnavailable::ResourceExhausted)?;
        units
            .try_reserve(text.len())
            .map_err(|_| ConcatUnavailable::ResourceExhausted)?;
        units.extend(text.encode_utf16());
    }
    Ok(ConcatPart::Constant { units, event })
}

fn argument(
    code: &MethodCode,
    types: &TypeFacts,
    expression: ExprId,
    parameter: TypeUseId,
) -> Result<ConcatPart, ConcatUnavailable> {
    let expr = code
        .expressions
        .get(expression.0 as usize)
        .ok_or(ConcatUnavailable::ArgumentView)?;
    let (view, storage) = match expr.kind {
        ExprKind::Read { view, storage } => (view, Some(storage)),
        ExprKind::Literal { view, .. } => (view, None),
        _ => return Err(ConcatUnavailable::ArgumentView),
    };
    if expr.origin.as_ref().is_none_or(|o| o.read.is_err()) {
        return Err(ConcatUnavailable::ArgumentView);
    }
    let local = match storage {
        Some(id) => Some(
            code.locals
                .get(id.0 as usize)
                .ok_or(ConcatUnavailable::ArgumentView)?,
        ),
        None => None,
    };
    if local.is_some_and(|local| local.gap.is_some()) {
        return Err(ConcatUnavailable::ArgumentView);
    }
    let expected_family = if named(code, parameter, "Ljava/lang/String;") {
        Family::Reference
    } else if named(code, parameter, "J") {
        Family::Long
    } else if named(code, parameter, "F") {
        Family::Float
    } else if named(code, parameter, "D") {
        Family::Double
    } else if ["I", "C", "B", "S", "Z"]
        .iter()
        .any(|name| named(code, parameter, name))
    {
        Family::Integral
    } else {
        return Err(ConcatUnavailable::ParameterType);
    };
    if local.is_some_and(|local| {
        !matches!(local.original.storage_type(), crate::p5::StorageType::Family { family, .. } if family == expected_family)
    }) {
        return Err(ConcatUnavailable::ArgumentView);
    }
    let visible_char = local.is_some_and(|local| {
        matches!(code.emission.local_types[local.id.0 as usize], LocalSpelling::Named(ty) if named(code, ty, "C"))
            || matches!(local.original.kind(), crate::p5::LocalKind::Parameter { .. })
                && !local.type_source_unknown
                && !local.declared_types.is_empty()
                && local.declared_types.iter().all(|&ty| named(code, ty, "C"))
    });
    let visible_bool = local.is_some_and(|local| {
        matches!(code.emission.local_types[local.id.0 as usize], LocalSpelling::Named(ty) if named(code, ty, "Z"))
            || matches!(local.original.kind(), crate::p5::LocalKind::Parameter { .. })
                && !local.type_source_unknown
                && !local.declared_types.is_empty()
                && local.declared_types.iter().all(|&ty| named(code, ty, "Z"))
    });
    let matches_integral = |narrow: bool| match view {
        Interpretation::IntegralView { declared } => types
            .name(declared)
            .is_some_and(|ty| !narrow || ty.units() == code.types[parameter.0 as usize].units),
        Interpretation::Direct(Family::Integral)
        | Interpretation::Literal {
            family: Family::Integral,
            ..
        } => !narrow,
        _ => false,
    };
    let conversion = if named(code, parameter, "Ljava/lang/String;") {
        if !matches!(
            view,
            Interpretation::Direct(Family::Reference) | Interpretation::Null
        ) || !matches!(
            expr.origin.as_ref().and_then(|o| o.initialization),
            Some(Initialization::Initialized)
        ) || local.is_some_and(|local| {
            local.type_source_unknown
                || local.declared_types.is_empty()
                || !local
                    .declared_types
                    .iter()
                    .all(|&ty| named(code, ty, "Ljava/lang/String;"))
        }) {
            return Err(ConcatUnavailable::ArgumentView);
        }
        ConcatConversion::Value
    } else if named(code, parameter, "I") && matches_integral(false) && !visible_bool {
        if visible_char {
            ConcatConversion::Int
        } else {
            ConcatConversion::Value
        }
    } else if named(code, parameter, "C") && matches_integral(true) {
        ConcatConversion::Char
    } else if named(code, parameter, "B") && matches_integral(true) {
        if visible_char {
            ConcatConversion::Byte
        } else {
            ConcatConversion::Value
        }
    } else if named(code, parameter, "S") && matches_integral(true) {
        if visible_char {
            ConcatConversion::Short
        } else {
            ConcatConversion::Value
        }
    } else if named(code, parameter, "Z") && matches_integral(true) {
        ConcatConversion::Value
    } else {
        let family = if named(code, parameter, "J") {
            Family::Long
        } else if named(code, parameter, "F") {
            Family::Float
        } else if named(code, parameter, "D") {
            Family::Double
        } else {
            return Err(ConcatUnavailable::ParameterType);
        };
        let integral_long = family == Family::Long
            && matches!(view, Interpretation::IntegralView { declared } if types.name(declared)
                .is_some_and(|ty| ty.units().iter().copied().eq("J".encode_utf16())));
        if !integral_long
            && !matches!(view, Interpretation::Direct(f) | Interpretation::Literal { family: f, .. } if f == family)
        {
            return Err(ConcatUnavailable::ArgumentView);
        }
        ConcatConversion::Value
    };
    if let ExprKind::Literal { bits, .. } = expr.kind
        && (matches!(
            view,
            Interpretation::Direct(Family::Float)
                | Interpretation::Literal {
                    family: Family::Float,
                    ..
                }
        ) && (bits.width != crate::SlotWidth::One
            || u32::try_from(bits.value)
                .ok()
                .is_none_or(|word| !f32::from_bits(word).is_finite()))
            || matches!(
                view,
                Interpretation::Direct(Family::Double)
                    | Interpretation::Literal {
                        family: Family::Double,
                        ..
                    }
            ) && !f64::from_bits(bits.value).is_finite())
    {
        return Err(ConcatUnavailable::ArgumentView);
    }
    Ok(ConcatPart::Argument {
        expression,
        conversion,
    })
}

fn plan(
    code: &MethodCode,
    types: &TypeFacts,
    call: &Call,
    constants: bool,
) -> Result<Concatenation, ConcatUnavailable> {
    let Ok(ReferenceDeclaration::CallSite {
        prototype,
        handle_kind,
        bootstrap_member,
        argument_count,
        arguments,
        ..
    }) = &code.references[call.target.0 as usize].declaration
    else {
        return Err(ConcatUnavailable::CallSignature);
    };
    let Ok(ReferenceDeclaration::Member(MemberDeclaration::Method {
        prototype: bootstrap,
        ..
    })) = &code.references[bootstrap_member.0 as usize].declaration
    else {
        return Err(ConcatUnavailable::BootstrapSignature);
    };
    let parameters = [
        "Ljava/lang/invoke/MethodHandles$Lookup;",
        "Ljava/lang/String;",
        "Ljava/lang/invoke/MethodType;",
        "Ljava/lang/String;",
        "[Ljava/lang/Object;",
    ];
    let length = if constants { 5 } else { 3 };
    if *handle_kind != 4
        || !named(code, bootstrap.returns, "Ljava/lang/invoke/CallSite;")
        || bootstrap.parameters.len() != length
        || !bootstrap
            .parameters
            .iter()
            .zip(parameters)
            .all(|(&ty, name)| named(code, ty, name))
    {
        return Err(ConcatUnavailable::BootstrapSignature);
    }
    if !named(code, prototype.returns, "Ljava/lang/String;")
        || call.receiver.is_some()
        || call.invocation_prototype.is_some()
    {
        return Err(ConcatUnavailable::CallSignature);
    }
    let Arguments::Known(values) = &call.arguments else {
        return Err(ConcatUnavailable::ArgumentLayout);
    };
    if values.len() != prototype.parameters.len() {
        return Err(ConcatUnavailable::ArgumentLayout);
    }
    let mut parts = Vec::new();
    if !constants {
        if *argument_count != 0 || !arguments.is_empty() {
            return Err(ConcatUnavailable::BootstrapConstant);
        }
        for (&value, &parameter) in values.iter().zip(&prototype.parameters) {
            push(&mut parts, argument(code, types, value, parameter)?)?;
        }
    } else {
        if usize::try_from(*argument_count).ok() != Some(arguments.len()) {
            return Err(ConcatUnavailable::BootstrapConstant);
        }
        let recipe = arguments
            .first()
            .and_then(|v| string_constant(code, v))
            .ok_or(ConcatUnavailable::BootstrapConstant)?;
        let text = units(code, recipe).ok_or(ConcatUnavailable::BootstrapConstant)?;
        let mut dynamic = 0usize;
        let mut constant = 1usize;
        let mut start = 0usize;
        for (position, &unit) in text.iter().enumerate() {
            if unit != 1 && unit != 2 {
                continue;
            }
            if start != position {
                push(
                    &mut parts,
                    ConcatPart::Text {
                        reference: recipe,
                        units: start..position,
                    },
                )?;
            }
            if unit == 1 {
                let value = *values.get(dynamic).ok_or(ConcatUnavailable::RecipeArity)?;
                let parameter = *prototype
                    .parameters
                    .get(dynamic)
                    .ok_or(ConcatUnavailable::RecipeArity)?;
                push(&mut parts, argument(code, types, value, parameter)?)?;
                dynamic = dynamic
                    .checked_add(1)
                    .ok_or(ConcatUnavailable::ResourceExhausted)?;
            } else {
                let value = arguments
                    .get(constant)
                    .ok_or(ConcatUnavailable::RecipeArity)?;
                push(&mut parts, constant_part(code, value, constant)?)?;
                constant = constant
                    .checked_add(1)
                    .ok_or(ConcatUnavailable::ResourceExhausted)?;
            }
            start = position + 1;
        }
        if start != text.len() {
            push(
                &mut parts,
                ConcatPart::Text {
                    reference: recipe,
                    units: start..text.len(),
                },
            )?;
        }
        if dynamic != values.len() || constant != arguments.len() {
            return Err(ConcatUnavailable::RecipeArity);
        }
    }
    let first_string = match parts.first() {
        Some(ConcatPart::Text { .. } | ConcatPart::Constant { .. }) => true,
        Some(ConcatPart::Argument { expression, .. }) => values
            .iter()
            .zip(&prototype.parameters)
            .any(|(&id, &parameter)| {
                id == *expression
                    && named(code, parameter, "Ljava/lang/String;")
                    && !matches!(
                        code.expressions[id.0 as usize].kind,
                        ExprKind::Read {
                            view: Interpretation::Null,
                            ..
                        } | ExprKind::Literal {
                            view: Interpretation::Null,
                            ..
                        }
                    )
            }),
        None => false,
    };
    Ok(Concatenation {
        empty_prefix: parts.len() < 2 || !first_string,
        parts,
    })
}

pub(super) fn construct(code: &mut MethodCode, types: &TypeFacts) {
    for index in 0..code.expressions.len() {
        let outcome = {
            let ExprKind::Call(call) = &code.expressions[index].kind else {
                continue;
            };
            if call.kind != crate::p2::InvokeKind::Custom {
                continue;
            }
            let Some(Reference {
                declaration:
                    Ok(ReferenceDeclaration::CallSite {
                        bootstrap_member, ..
                    }),
                ..
            }) = code.references.get(call.target.0 as usize)
            else {
                continue;
            };
            let Some(Reference {
                declaration:
                    Ok(ReferenceDeclaration::Member(MemberDeclaration::Method { owner, name, .. })),
                ..
            }) = code.references.get(bootstrap_member.0 as usize)
            else {
                continue;
            };
            if !named(code, *owner, "Ljava/lang/invoke/StringConcatFactory;") {
                continue;
            }
            let constants = if name.units.iter().copied().eq("makeConcat".encode_utf16()) {
                false
            } else if name
                .units
                .iter()
                .copied()
                .eq("makeConcatWithConstants".encode_utf16())
            {
                true
            } else {
                continue;
            };
            Some(match plan(code, types, call, constants) {
                Ok(plan) => ConcatOutcome::Inferred(plan),
                Err(reason) => ConcatOutcome::Explicit(reason),
            })
        };
        if let ExprKind::Call(call) = &mut code.expressions[index].kind {
            call.concatenation = outcome;
        }
    }
}
