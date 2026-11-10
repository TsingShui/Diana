//! P1's directly consumable recovery IR.
//!
//! P1 already owns the checked instruction directory, reference facts, invoke
//! layouts and protection boundaries. This module turns those facts into the
//! instruction list and exception handlers the recovery backend consumes, so
//! the engine performs no second decode or symbol mapping.

use crate::frontend::{ExceptionHandler, TryCatchBlock};
use crate::ir::{
    FieldReference, InsnArg, InsnNode, InsnType, InvokeType, LambdaSite, MemberReference,
    MethodDescriptor, MethodReference,
};
use crate::p1::{
    BootstrapDeclaration, CheckedBootstrapFact, CheckedMemberFact, CheckedPrototype,
    CheckedReferenceFact, DecodedMethod,
};
use crate::{CodeUnitOffset, TypeId};

use super::MethodIr;

/// Build the backend-ready instruction list and handlers from the published P1
/// facts. Failures are retained as an explicit product gap instead of widening
/// the method result.
pub(super) fn build(facts: &DecodedMethod<'_>) -> Result<MethodIr, String> {
    let source = facts.source();
    let bytes = source.instructions();
    let mut insns = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        insns.push(u16::from_le_bytes([pair[0], pair[1]]));
    }
    let tries = tries(facts);
    let result = crate::decoder::method_decoder::MethodDecoder::from_parts(
        &insns,
        &tries,
        source.registers_size(),
        source.ins_size(),
    )
    .decode();
    let mut instructions = result.insns;
    resolve_symbols(facts, &mut instructions)?;
    Ok(MethodIr::new(
        instructions,
        result.handlers,
        result.registers,
        result.ins,
    ))
}

fn resolve_symbols(facts: &DecodedMethod<'_>, insns: &mut [InsnNode]) -> Result<(), String> {
    for insn in insns {
        let pc = CodeUnitOffset(insn.offset);
        if insn.insn_type == InsnType::Invoke {
            match insn.payload.invoke_type {
                Some(InvokeType::Polymorphic) => {
                    let fact = reference(facts, pc)?;
                    let CheckedReferenceFact::Method { owner, name, .. } = fact else {
                        return Err(format!("polymorphic invoke at {pc:?} has another kind"));
                    };
                    let proto = reference_slot(facts, pc, crate::raw::ReferenceSlot::Second)?;
                    let CheckedReferenceFact::Proto { prototype } = proto else {
                        return Err(format!(
                            "polymorphic invoke at {pc:?} has no checked prototype"
                        ));
                    };
                    let method = MethodReference {
                        owner: descriptor(*owner)?,
                        name: text(*name)?,
                        descriptor: prototype_descriptor(prototype)?,
                    };
                    apply_invoke_return_type(insn, &method);
                    regroup_invoke_args(insn, facts, pc, &method.descriptor)?;
                    insn.payload.reference = Some(Box::new(MemberReference::Method(method)));
                    continue;
                }
                Some(InvokeType::Custom) => {
                    let fact = reference(facts, pc)?;
                    let CheckedReferenceFact::CallSite {
                        name: sam_name,
                        prototype,
                        handle,
                        declarations,
                        ..
                    } = fact
                    else {
                        return Err(format!("custom invoke at {pc:?} has another kind"));
                    };
                    let mut bootstrap = None;
                    for declaration in declarations {
                        if declaration.reference != handle.member() {
                            continue;
                        }
                        if let CheckedBootstrapFact::Member(CheckedMemberFact::Method {
                            owner,
                            name,
                            ..
                        }) = &declaration.fact
                        {
                            bootstrap = Some((*owner, *name));
                        }
                        break;
                    }
                    let Some((owner, name)) = bootstrap else {
                        return Err(format!(
                            "custom invoke at {pc:?} has no bootstrap method declaration"
                        ));
                    };
                    let bootstrap_owner = text(owner)?;
                    let bootstrap_name = text(name)?;
                    let bootstrap_owner_name = bootstrap_owner
                        .strip_prefix('L')
                        .and_then(|owner| owner.strip_suffix(';'))
                        .unwrap_or(&bootstrap_owner);
                    if bootstrap_owner_name == "java/lang/invoke/LambdaMetafactory"
                        && matches!(bootstrap_name.as_str(), "metafactory" | "altMetafactory")
                    {
                        insn.payload.lambda =
                            lambda_site(*sam_name, prototype, declarations)?.map(Box::new);
                    }
                    let method = MethodReference {
                        owner: descriptor(owner)?,
                        name: bootstrap_name,
                        descriptor: prototype_descriptor(prototype)?,
                    };
                    apply_invoke_return_type(insn, &method);
                    regroup_invoke_args(insn, facts, pc, &method.descriptor)?;
                    insn.payload.reference = Some(Box::new(MemberReference::Method(method)));
                    continue;
                }
                _ => {}
            }
        }
        if insn.payload.method_index.is_some() {
            let fact = reference(facts, pc)?;
            let CheckedReferenceFact::Method {
                owner,
                name,
                prototype,
                ..
            } = fact
            else {
                return Err(format!("method reference at {pc:?} has another kind"));
            };
            let method = MethodReference {
                owner: descriptor(*owner)?,
                name: text(*name)?,
                descriptor: prototype_descriptor(prototype)?,
            };
            apply_invoke_return_type(insn, &method);
            regroup_invoke_args(insn, facts, pc, &method.descriptor)?;
            insn.payload.reference = Some(Box::new(MemberReference::Method(method)));
        }
        if insn.payload.field_index.is_some() {
            let fact = reference(facts, pc)?;
            let CheckedReferenceFact::Field {
                owner,
                name,
                field_type,
                ..
            } = fact
            else {
                return Err(format!("field reference at {pc:?} has another kind"));
            };
            let field = FieldReference {
                owner: descriptor(*owner)?,
                name: text(*name)?,
                field_type: descriptor(*field_type)?,
            };
            insn.payload.reference = Some(Box::new(MemberReference::Field(field)));
        }
        if insn.payload.string_index.is_some() && insn.payload.string_value.is_none() {
            let fact = reference(facts, pc)?;
            let CheckedReferenceFact::String { value } = fact else {
                return Err(format!("string reference at {pc:?} has another kind"));
            };
            let units = value.units().map_err(|error| error.to_string())?;
            insn.payload.string_value = Some(Box::new(crate::ir::Utf16String::from_utf16(units)));
        }
        if insn.payload.type_index.is_some() && insn.payload.class_type.is_none() {
            let fact = reference(facts, pc)?;
            let CheckedReferenceFact::Type { descriptor, .. } = fact else {
                return Err(format!("type reference at {pc:?} has another kind"));
            };
            insn.payload.class_type = Some(Box::new(self::descriptor(*descriptor)?));
        }
    }
    Ok(())
}

/// Recover the `LambdaMetafactory` bootstrap arguments that describe a SAM
/// implementation. The first prototype is the SAM signature and the first
/// method handle is the implementation.
fn lambda_site<'a>(
    sam_name: crate::input::DexString<'a>,
    interface: &CheckedPrototype<'a>,
    declarations: &[BootstrapDeclaration<'a>],
) -> Result<Option<LambdaSite>, String> {
    let mut sam = None;
    let mut implementation = None;
    let mut prototypes = 0usize;
    for declaration in declarations {
        match &declaration.fact {
            CheckedBootstrapFact::Proto { prototype } => {
                if prototypes == 0 {
                    sam = Some(prototype);
                }
                prototypes += 1;
            }
            CheckedBootstrapFact::MethodHandle {
                handle,
                member:
                    CheckedMemberFact::Method {
                        owner,
                        name,
                        prototype,
                        ..
                    },
            } if implementation.is_none() => {
                implementation = Some((handle, *owner, *name, prototype));
            }
            _ => {}
        }
    }
    let (Some(sam), Some((handle, owner, name, implementation_prototype))) = (sam, implementation)
    else {
        return Ok(None);
    };
    let interface = descriptor(interface.return_type())?;
    Ok(Some(LambdaSite {
        interface,
        sam_name: text(sam_name)?,
        sam_descriptor: prototype_descriptor(sam)?,
        implementation: MethodReference {
            owner: descriptor(owner)?,
            name: text(name)?,
            descriptor: prototype_descriptor(implementation_prototype)?,
        },
        // DEX method-handle kinds: 0x04 invoke-static, 0x05 invoke-instance,
        // 0x06 invoke-constructor, 0x07 invoke-direct, 0x08 invoke-interface.
        implementation_static: handle.kind() == 4,
        implementation_constructor: handle.kind() == 6,
    }))
}

/// Collapse the flat encoded register list into one IR operand per declared
/// parameter, using P1's checked layout. A wide parameter keeps its first
/// register and drops the high half.
fn regroup_invoke_args(
    insn: &mut InsnNode,
    facts: &DecodedMethod<'_>,
    pc: CodeUnitOffset,
    descriptor: &MethodDescriptor,
) -> Result<(), String> {
    let Some(Ok(layout)) = facts.invoke_layout_at(pc) else {
        return Ok(());
    };
    let mut words = insn.args.iter().cloned();
    let mut args = Vec::new();
    args.try_reserve_exact(usize::from(layout.has_receiver()) + layout.arguments().len())
        .map_err(|_| "invoke argument allocation failed".to_owned())?;
    if layout.has_receiver() {
        let receiver = words
            .next()
            .ok_or_else(|| format!("invoke at {pc:?} lost its receiver"))?;
        args.push(receiver);
    }
    for (width, parameter) in layout.arguments().iter().zip(&descriptor.parameters) {
        let Some(InsnArg::Reg(mut register)) = words.next() else {
            return Err(format!("invoke at {pc:?} lost a declared argument"));
        };
        register.ty = parameter.clone();
        args.push(InsnArg::Reg(register));
        if width.words() == 2 {
            words.next();
        }
    }
    insn.args = args;
    Ok(())
}

fn reference<'a>(
    facts: &'a DecodedMethod<'_>,
    pc: CodeUnitOffset,
) -> Result<&'a CheckedReferenceFact<'a>, String> {
    reference_slot(facts, pc, crate::raw::ReferenceSlot::First)
}

fn reference_slot<'a>(
    facts: &'a DecodedMethod<'_>,
    pc: CodeUnitOffset,
    slot: crate::raw::ReferenceSlot,
) -> Result<&'a CheckedReferenceFact<'a>, String> {
    facts
        .reference_fact(pc, slot)
        .ok_or_else(|| format!("missing reference fact at {pc:?}"))?
        .map_err(|error| error.to_string())
}

fn text(value: crate::input::DexString<'_>) -> Result<String, String> {
    value
        .utf8_lossy()
        .map(|text| text.into_owned())
        .map_err(|error| error.to_string())
}

fn descriptor(value: crate::input::DexString<'_>) -> Result<crate::ir::ArgType, String> {
    text(value)?
        .parse()
        .map_err(|error: crate::ir::DescriptorParseError| error.to_string())
}

fn prototype_descriptor(prototype: &CheckedPrototype<'_>) -> Result<MethodDescriptor, String> {
    let mut parameters = Vec::new();
    parameters
        .try_reserve_exact(prototype.parameters().len())
        .map_err(|_| "prototype parameter allocation failed".to_owned())?;
    for parameter in prototype.parameters() {
        parameters.push(descriptor(parameter.descriptor())?);
    }
    Ok(MethodDescriptor {
        parameters,
        return_type: descriptor(prototype.return_type())?,
    })
}

fn apply_invoke_return_type(insn: &mut InsnNode, method: &MethodReference) {
    if insn.insn_type != InsnType::Invoke {
        return;
    }
    let return_type = method.descriptor.return_type.clone();
    if return_type == crate::ir::ArgType::VOID {
        return;
    }
    if let Some(result) = insn.result.as_mut() {
        result.ty = return_type;
    }
}

fn tries(facts: &DecodedMethod<'_>) -> Vec<TryCatchBlock> {
    let boundaries: Vec<u32> = facts.try_boundaries().collect();
    let mut blocks = Vec::new();
    for pair in boundaries.chunks_exact(2) {
        let (start, end) = (pair[0], pair[1]);
        let clauses = facts
            .catch_clauses_at(CodeUnitOffset(start))
            .unwrap_or_default();
        let mut handlers = Vec::new();
        for (exception, target) in clauses {
            let handler = match exception {
                Some(id) => facts
                    .catch_type(TypeId(id))
                    .and_then(|fact| fact.descriptor().ok())
                    .and_then(|text| text.utf8_lossy().ok())
                    .and_then(|text| text.parse::<crate::ir::ArgType>().ok())
                    .map_or_else(
                        || ExceptionHandler::catch_all(target),
                        |ty| ExceptionHandler::typed(ty, target),
                    ),
                None => ExceptionHandler::catch_all(target),
            };
            handlers.push(handler);
        }
        blocks.push(TryCatchBlock {
            start_addr: start,
            end_addr: end,
            handlers,
        });
    }
    blocks
}
